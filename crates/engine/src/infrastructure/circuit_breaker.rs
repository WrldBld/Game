//! Circuit breaker pattern implementation for resilient service calls.
//!
//! The circuit breaker prevents cascading failures by temporarily rejecting
//! requests when a service is failing. It has three states:
//!
//! - **Closed**: Normal operation, requests pass through
//! - **Open**: Service failing, all requests rejected immediately
//! - **HalfOpen**: Testing recovery, limited requests allowed
//!
//! See `docs/designs/LLM_RESILIENCE_AND_CUSTOM_EVALUATION.md` for design details.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit
    pub failure_threshold: u32,
    /// Duration the circuit stays open before transitioning to half-open
    pub open_duration: Duration,
    /// Maximum requests allowed in half-open state before deciding to open or close
    pub half_open_max_requests: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            open_duration: Duration::from_secs(60),
            half_open_max_requests: 1,
        }
    }
}

impl CircuitBreakerConfig {
    /// Create config from AppSettings values
    pub fn from_settings(
        failure_threshold: u32,
        open_duration_secs: u64,
        half_open_max_requests: u32,
    ) -> Self {
        Self {
            failure_threshold,
            open_duration: Duration::from_secs(open_duration_secs),
            half_open_max_requests,
        }
    }
}

/// Current state of the circuit breaker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Service failing - requests rejected immediately
    Open,
    /// Testing recovery - limited requests allowed
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::Open => write!(f, "open"),
            CircuitState::HalfOpen => write!(f, "half-open"),
        }
    }
}

/// Error returned when circuit breaker rejects a request
#[derive(Debug, Clone)]
pub struct CircuitOpenError {
    /// Time when circuit will transition to half-open
    pub retry_after: Duration,
}

impl std::fmt::Display for CircuitOpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Circuit breaker is open, retry after {:?}",
            self.retry_after
        )
    }
}

impl std::error::Error for CircuitOpenError {}

/// Internal state tracking
struct InternalState {
    state: CircuitState,
    /// When the circuit was opened (only valid when state is Open)
    opened_at: Option<Instant>,
    /// Number of half-open requests that have been allowed
    half_open_requests: u32,
    /// Number of successful half-open requests
    half_open_successes: u32,
}

/// Thread-safe circuit breaker implementation
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    /// Current internal state (protected by RwLock for state transitions)
    state: RwLock<InternalState>,
    /// Consecutive failure count (atomic for fast path)
    consecutive_failures: AtomicU32,
    /// Total failure count since last reset (for metrics)
    total_failures: AtomicU64,
    /// Total success count since last reset (for metrics)
    total_successes: AtomicU64,
    /// Number of times circuit has opened (for metrics)
    open_count: AtomicU64,
    /// Optional callback when state changes
    on_state_change: Option<Box<dyn Fn(CircuitState, CircuitState) + Send + Sync>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: RwLock::new(InternalState {
                state: CircuitState::Closed,
                opened_at: None,
                half_open_requests: 0,
                half_open_successes: 0,
            }),
            consecutive_failures: AtomicU32::new(0),
            total_failures: AtomicU64::new(0),
            total_successes: AtomicU64::new(0),
            open_count: AtomicU64::new(0),
            on_state_change: None,
        }
    }

    /// Set a callback to be invoked when circuit state changes
    pub fn with_state_change_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(CircuitState, CircuitState) + Send + Sync + 'static,
    {
        self.on_state_change = Some(Box::new(callback));
        self
    }

    /// Get the current circuit state
    pub fn state(&self) -> CircuitState {
        let state = self.state.read().unwrap();

        // Check if we should transition from Open to HalfOpen
        if state.state == CircuitState::Open {
            if let Some(opened_at) = state.opened_at {
                if opened_at.elapsed() >= self.config.open_duration {
                    // Need to upgrade to write lock to transition
                    drop(state);
                    return self.try_transition_to_half_open();
                }
            }
        }

        state.state
    }

    /// Try to transition from Open to HalfOpen if duration has elapsed
    fn try_transition_to_half_open(&self) -> CircuitState {
        // Collect state transition info while holding lock, then release before callback
        let (current_state, transition) = {
            let mut state = self.state.write().unwrap();

            // Double-check after acquiring write lock
            let transition = if state.state == CircuitState::Open {
                if let Some(opened_at) = state.opened_at {
                    if opened_at.elapsed() >= self.config.open_duration {
                        let old_state = state.state;
                        state.state = CircuitState::HalfOpen;
                        state.half_open_requests = 0;
                        state.half_open_successes = 0;

                        tracing::info!(
                            "Circuit breaker transitioning from {:?} to {:?}",
                            old_state,
                            state.state
                        );

                        Some((old_state, state.state))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            (state.state, transition)
        }; // Lock released here

        // Invoke callback outside of lock to prevent deadlock
        if let (Some((old_state, new_state)), Some(ref callback)) =
            (transition, &self.on_state_change)
        {
            callback(old_state, new_state);
        }

        current_state
    }

    /// Check if a request should be allowed through
    ///
    /// Returns Ok(()) if the request should proceed, or Err(CircuitOpenError)
    /// if the circuit is open and the request should be rejected.
    pub fn allow_request(&self) -> Result<(), CircuitOpenError> {
        let current_state = self.state();

        match current_state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                let state = self.state.read().unwrap();
                let retry_after = if let Some(opened_at) = state.opened_at {
                    self.config.open_duration.saturating_sub(opened_at.elapsed())
                } else {
                    self.config.open_duration
                };
                Err(CircuitOpenError { retry_after })
            }
            CircuitState::HalfOpen => {
                // Check if we can allow another half-open request
                let mut state = self.state.write().unwrap();
                if state.half_open_requests < self.config.half_open_max_requests {
                    state.half_open_requests += 1;
                    Ok(())
                } else {
                    // Already at max half-open requests, wait for them to complete
                    Err(CircuitOpenError {
                        retry_after: Duration::from_secs(1),
                    })
                }
            }
        }
    }

    /// Record a successful request
    pub fn record_success(&self) {
        self.total_successes.fetch_add(1, Ordering::Relaxed);
        self.consecutive_failures.store(0, Ordering::Relaxed);

        // Collect state transition info while holding lock, then release before callback
        let transition = {
            let mut state = self.state.write().unwrap();

            match state.state {
                CircuitState::Closed => None,
                CircuitState::HalfOpen => {
                    state.half_open_successes += 1;

                    // If all half-open requests succeeded, close the circuit
                    if state.half_open_successes >= self.config.half_open_max_requests {
                        let old_state = state.state;
                        state.state = CircuitState::Closed;
                        state.opened_at = None;

                        tracing::info!(
                            "Circuit breaker closing after {} successful half-open requests",
                            state.half_open_successes
                        );

                        Some((old_state, state.state))
                    } else {
                        None
                    }
                }
                CircuitState::Open => None,
            }
        }; // Lock released here

        // Invoke callback outside of lock to prevent deadlock
        if let (Some((old_state, new_state)), Some(ref callback)) =
            (transition, &self.on_state_change)
        {
            callback(old_state, new_state);
        }
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;

        // Collect state transition info while holding lock, then release before callback
        let transition = {
            let mut state = self.state.write().unwrap();

            match state.state {
                CircuitState::Closed => {
                    if failures >= self.config.failure_threshold {
                        let old_state = state.state;
                        state.state = CircuitState::Open;
                        state.opened_at = Some(Instant::now());
                        self.open_count.fetch_add(1, Ordering::Relaxed);

                        tracing::warn!(
                            consecutive_failures = failures,
                            threshold = self.config.failure_threshold,
                            open_duration_secs = self.config.open_duration.as_secs(),
                            "Circuit breaker opening due to consecutive failures"
                        );

                        Some((old_state, state.state))
                    } else {
                        None
                    }
                }
                CircuitState::HalfOpen => {
                    // Any failure in half-open immediately re-opens the circuit
                    let old_state = state.state;
                    state.state = CircuitState::Open;
                    state.opened_at = Some(Instant::now());
                    self.open_count.fetch_add(1, Ordering::Relaxed);

                    tracing::warn!(
                        "Circuit breaker re-opening after failure in half-open state"
                    );

                    Some((old_state, state.state))
                }
                CircuitState::Open => {
                    // Already open, just update opened_at to extend the open period
                    state.opened_at = Some(Instant::now());
                    None
                }
            }
        }; // Lock released here

        // Invoke callback outside of lock to prevent deadlock
        if let (Some((old_state, new_state)), Some(ref callback)) =
            (transition, &self.on_state_change)
        {
            callback(old_state, new_state);
        }
    }

    /// Get metrics for the circuit breaker
    pub fn metrics(&self) -> CircuitBreakerMetrics {
        let state = self.state.read().unwrap();
        CircuitBreakerMetrics {
            state: state.state,
            consecutive_failures: self.consecutive_failures.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            total_successes: self.total_successes.load(Ordering::Relaxed),
            open_count: self.open_count.load(Ordering::Relaxed),
            time_until_half_open: if state.state == CircuitState::Open {
                state.opened_at.map(|t| self.config.open_duration.saturating_sub(t.elapsed()))
            } else {
                None
            },
        }
    }

    /// Force the circuit to a specific state (for testing/admin purposes)
    pub fn force_state(&self, new_state: CircuitState) {
        // Collect state transition info while holding lock, then release before callback
        let transition = {
            let mut state = self.state.write().unwrap();
            let old_state = state.state;

            state.state = new_state;

            match new_state {
                CircuitState::Closed => {
                    state.opened_at = None;
                    self.consecutive_failures.store(0, Ordering::Relaxed);
                }
                CircuitState::Open => {
                    state.opened_at = Some(Instant::now());
                }
                CircuitState::HalfOpen => {
                    state.half_open_requests = 0;
                    state.half_open_successes = 0;
                }
            }

            if old_state != new_state {
                tracing::info!(
                    old_state = %old_state,
                    new_state = %new_state,
                    "Circuit breaker state forced"
                );
                Some((old_state, new_state))
            } else {
                None
            }
        }; // Lock released here

        // Invoke callback outside of lock to prevent deadlock
        if let (Some((old_state, new_state)), Some(ref callback)) =
            (transition, &self.on_state_change)
        {
            callback(old_state, new_state);
        }
    }

    /// Reset the circuit breaker to closed state and clear all counters
    pub fn reset(&self) {
        let mut state = self.state.write().unwrap();
        state.state = CircuitState::Closed;
        state.opened_at = None;
        state.half_open_requests = 0;
        state.half_open_successes = 0;

        self.consecutive_failures.store(0, Ordering::Relaxed);
        self.total_failures.store(0, Ordering::Relaxed);
        self.total_successes.store(0, Ordering::Relaxed);
        self.open_count.store(0, Ordering::Relaxed);

        tracing::info!("Circuit breaker reset to closed state");
    }
}

/// Metrics for circuit breaker state
#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    /// Current state of the circuit
    pub state: CircuitState,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
    /// Total failures since last reset
    pub total_failures: u64,
    /// Total successes since last reset
    pub total_successes: u64,
    /// Number of times circuit has opened
    pub open_count: u64,
    /// Time remaining until circuit transitions to half-open (if open)
    pub time_until_half_open: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_starts_closed() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.allow_request().is_ok());
    }

    #[test]
    fn test_opens_after_threshold_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            open_duration: Duration::from_secs(60),
            half_open_max_requests: 1,
        };
        let cb = CircuitBreaker::new(config);

        // First 2 failures shouldn't open
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        // Third failure should open
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(cb.allow_request().is_err());
    }

    #[test]
    fn test_success_resets_failure_count() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            open_duration: Duration::from_secs(60),
            half_open_max_requests: 1,
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        cb.record_failure();
        cb.record_success(); // Reset
        cb.record_failure();
        cb.record_failure();

        // Should still be closed because success reset the count
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_transitions_to_half_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration: Duration::from_millis(50),
            half_open_max_requests: 1,
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for open duration
        thread::sleep(Duration::from_millis(60));

        assert_eq!(cb.state(), CircuitState::HalfOpen);
        assert!(cb.allow_request().is_ok());
    }

    #[test]
    fn test_half_open_closes_on_success() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration: Duration::from_millis(10),
            half_open_max_requests: 1,
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        thread::sleep(Duration::from_millis(20));
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_half_open_reopens_on_failure() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration: Duration::from_millis(10),
            half_open_max_requests: 1,
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        thread::sleep(Duration::from_millis(20));
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_metrics() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            open_duration: Duration::from_secs(60),
            half_open_max_requests: 1,
        };
        let cb = CircuitBreaker::new(config);

        cb.record_success();
        cb.record_success();
        cb.record_failure();
        cb.record_failure();

        let metrics = cb.metrics();
        assert_eq!(metrics.state, CircuitState::Open);
        assert_eq!(metrics.consecutive_failures, 2);
        assert_eq!(metrics.total_failures, 2);
        assert_eq!(metrics.total_successes, 2);
        assert_eq!(metrics.open_count, 1);
        assert!(metrics.time_until_half_open.is_some());
    }

    #[test]
    fn test_thread_safety() {
        let config = CircuitBreakerConfig {
            failure_threshold: 100,
            open_duration: Duration::from_secs(60),
            half_open_max_requests: 1,
        };
        let cb = Arc::new(CircuitBreaker::new(config));

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let cb = Arc::clone(&cb);
                thread::spawn(move || {
                    for _ in 0..10 {
                        if i % 2 == 0 {
                            cb.record_success();
                        } else {
                            cb.record_failure();
                        }
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let metrics = cb.metrics();
        assert_eq!(metrics.total_successes + metrics.total_failures, 100);
    }

    #[test]
    fn test_force_state() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());

        cb.force_state(CircuitState::Open);
        assert_eq!(cb.state(), CircuitState::Open);

        cb.force_state(CircuitState::HalfOpen);
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        cb.force_state(CircuitState::Closed);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_reset() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration: Duration::from_secs(60),
            half_open_max_requests: 1,
        };
        let cb = CircuitBreaker::new(config);

        cb.record_failure();
        cb.record_success();
        cb.record_failure();

        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();

        assert_eq!(cb.state(), CircuitState::Closed);
        let metrics = cb.metrics();
        assert_eq!(metrics.total_failures, 0);
        assert_eq!(metrics.total_successes, 0);
        assert_eq!(metrics.open_count, 0);
    }

    #[test]
    fn test_state_change_callback() {
        use std::sync::atomic::AtomicBool;

        let callback_called = Arc::new(AtomicBool::new(false));
        let callback_called_clone = Arc::clone(&callback_called);

        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration: Duration::from_secs(60),
            half_open_max_requests: 1,
        };

        let cb = CircuitBreaker::new(config)
            .with_state_change_callback(move |_old, _new| {
                callback_called_clone.store(true, Ordering::Relaxed);
            });

        cb.record_failure();

        assert!(callback_called.load(Ordering::Relaxed));
    }

    #[test]
    fn test_half_open_requires_multiple_successes() {
        // Test that with half_open_max_requests > 1, multiple successes are required
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration: Duration::from_millis(10),
            half_open_max_requests: 3, // Require 3 successful requests
        };
        let cb = CircuitBreaker::new(config);

        // Open the circuit
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Wait for half-open
        thread::sleep(Duration::from_millis(20));
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // First success - still half-open
        assert!(cb.allow_request().is_ok());
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Second success - still half-open
        assert!(cb.allow_request().is_ok());
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Third success - should close
        assert!(cb.allow_request().is_ok());
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);

        // Verify metrics
        let metrics = cb.metrics();
        assert_eq!(metrics.total_successes, 3);
        assert_eq!(metrics.consecutive_failures, 0);
    }

    #[test]
    fn test_half_open_limits_concurrent_requests() {
        // Test that half-open state limits concurrent requests
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration: Duration::from_millis(10),
            half_open_max_requests: 2,
        };
        let cb = CircuitBreaker::new(config);

        // Open and transition to half-open
        cb.record_failure();
        thread::sleep(Duration::from_millis(20));
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // First 2 requests should be allowed
        assert!(cb.allow_request().is_ok());
        assert!(cb.allow_request().is_ok());

        // Third request should be rejected (at max)
        assert!(cb.allow_request().is_err());

        // After recording successes, more requests can be allowed
        cb.record_success();
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_callback_can_safely_read_state() {
        // Test that callbacks don't deadlock when reading circuit state
        use std::sync::atomic::AtomicU32;

        let state_in_callback = Arc::new(AtomicU32::new(0));
        let state_clone = Arc::clone(&state_in_callback);

        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration: Duration::from_secs(60),
            half_open_max_requests: 1,
        };

        let cb = Arc::new(CircuitBreaker::new(config));
        let cb_for_callback = Arc::clone(&cb);

        // Replace with a new circuit breaker that has a callback
        let cb_with_callback = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 1,
            open_duration: Duration::from_secs(60),
            half_open_max_requests: 1,
        })
        .with_state_change_callback(move |_old, new| {
            // This should NOT deadlock - callback is called without lock held
            state_clone.store(new as u32, Ordering::Relaxed);
        });

        cb_with_callback.record_failure();

        // Callback should have been called and able to read new state
        assert_eq!(
            state_in_callback.load(Ordering::Relaxed),
            CircuitState::Open as u32
        );
    }
}
