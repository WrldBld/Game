//! Resilient LLM client wrapper with exponential backoff retry and circuit breaker
//!
//! Wraps any LlmPort implementation with retry logic and circuit breaker pattern
//! to handle transient failures and prevent cascading failures.
//! See `docs/designs/LLM_RESILIENCE_AND_CUSTOM_EVALUATION.md` for design details.

use async_trait::async_trait;
use rand::Rng;
use std::sync::Arc;
use std::time::Duration;

use crate::infrastructure::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerMetrics, CircuitState};
use crate::infrastructure::ports::{LlmError, LlmPort, LlmRequest, LlmResponse, ToolDefinition};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (0 = no retries, just the initial attempt)
    pub max_retries: u32,
    /// Base delay in milliseconds before first retry
    pub base_delay_ms: u64,
    /// Maximum delay in milliseconds (caps exponential growth)
    pub max_delay_ms: u64,
    /// Jitter factor (0.0-1.0) for randomizing delays to prevent thundering herd
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
            jitter_factor: 0.2,
        }
    }
}

/// Wrapper that adds retry logic and circuit breaker to any LLM client
pub struct ResilientLlmClient {
    inner: Arc<dyn LlmPort>,
    retry_config: RetryConfig,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl ResilientLlmClient {
    /// Create a new resilient wrapper around an existing LLM client
    pub fn new(inner: Arc<dyn LlmPort>, retry_config: RetryConfig) -> Self {
        Self {
            inner,
            retry_config,
            circuit_breaker: Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default())),
        }
    }

    /// Create a new resilient wrapper with custom circuit breaker configuration
    pub fn with_circuit_breaker(
        inner: Arc<dyn LlmPort>,
        retry_config: RetryConfig,
        circuit_breaker_config: CircuitBreakerConfig,
    ) -> Self {
        Self {
            inner,
            retry_config,
            circuit_breaker: Arc::new(CircuitBreaker::new(circuit_breaker_config)),
        }
    }

    /// Create a new resilient wrapper with a shared circuit breaker
    ///
    /// This allows multiple ResilientLlmClient instances to share the same
    /// circuit breaker state (useful when you have multiple LLM clients
    /// pointing to the same service).
    pub fn with_shared_circuit_breaker(
        inner: Arc<dyn LlmPort>,
        retry_config: RetryConfig,
        circuit_breaker: Arc<CircuitBreaker>,
    ) -> Self {
        Self {
            inner,
            retry_config,
            circuit_breaker,
        }
    }

    /// Get the current circuit breaker state
    pub fn circuit_state(&self) -> CircuitState {
        self.circuit_breaker.state()
    }

    /// Get circuit breaker metrics
    pub fn circuit_metrics(&self) -> CircuitBreakerMetrics {
        self.circuit_breaker.metrics()
    }

    /// Force the circuit breaker to a specific state (for testing/admin)
    pub fn force_circuit_state(&self, state: CircuitState) {
        self.circuit_breaker.force_state(state);
    }

    /// Reset the circuit breaker
    pub fn reset_circuit_breaker(&self) {
        self.circuit_breaker.reset();
    }

    /// Get a reference to the underlying circuit breaker
    pub fn circuit_breaker(&self) -> &Arc<CircuitBreaker> {
        &self.circuit_breaker
    }

    /// Calculate delay for a given attempt number using exponential backoff with jitter
    fn calculate_delay(&self, attempt: u32) -> u64 {
        let base = self.retry_config.base_delay_ms;
        // Exponential: base * 2^(attempt-1)
        let exponential = base.saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1)));
        let capped = exponential.min(self.retry_config.max_delay_ms);

        // Add jitter: ±jitter_factor around the delay
        let jitter_range = (capped as f64 * self.retry_config.jitter_factor) as i64;
        if jitter_range > 0 {
            let jitter = rand::thread_rng().gen_range(-jitter_range..=jitter_range);
            (capped as i64 + jitter).max(0) as u64
        } else {
            capped
        }
    }

    /// Determine if an error is retryable
    fn is_retryable(error: &LlmError) -> bool {
        match error {
            // Network/request failures are typically transient
            LlmError::RequestFailed(msg) => {
                // Don't retry on auth errors or bad requests
                !msg.contains("401")
                    && !msg.contains("403")
                    && !msg.contains("400")
                    && !msg.contains("Invalid")
            }
            // Invalid response could be transient (malformed response due to network issues)
            LlmError::InvalidResponse(_) => true,
        }
    }

    async fn execute_with_retry<F, Fut>(&self, operation_name: &str, operation: F) -> Result<LlmResponse, LlmError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<LlmResponse, LlmError>>,
    {
        // Check circuit breaker before attempting
        if let Err(circuit_error) = self.circuit_breaker.allow_request() {
            tracing::warn!(
                operation = operation_name,
                retry_after_secs = circuit_error.retry_after.as_secs(),
                "LLM request rejected by circuit breaker"
            );
            return Err(LlmError::RequestFailed(format!(
                "Circuit breaker is open: {}",
                circuit_error
            )));
        }

        let mut last_error = None;

        for attempt in 0..=self.retry_config.max_retries {
            match operation().await {
                Ok(response) => {
                    // Record success with circuit breaker
                    self.circuit_breaker.record_success();

                    if attempt > 0 {
                        tracing::info!(
                            attempt = attempt + 1,
                            operation = operation_name,
                            "LLM request succeeded after retry"
                        );
                    }
                    return Ok(response);
                }
                Err(e) => {
                    let is_retryable = Self::is_retryable(&e);

                    if attempt < self.retry_config.max_retries && is_retryable {
                        let delay = self.calculate_delay(attempt + 1);
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_retries = self.retry_config.max_retries,
                            delay_ms = delay,
                            error = %e,
                            operation = operation_name,
                            "LLM request failed, retrying..."
                        );
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                    } else if !is_retryable {
                        // Record failure with circuit breaker for non-retryable errors
                        self.circuit_breaker.record_failure();
                        tracing::error!(
                            error = %e,
                            operation = operation_name,
                            "LLM request failed with non-retryable error"
                        );
                        return Err(e);
                    }

                    last_error = Some(e);
                }
            }
        }

        // Record failure with circuit breaker after all retries exhausted
        self.circuit_breaker.record_failure();

        let error = last_error.unwrap_or_else(|| LlmError::RequestFailed("Unknown error".to_string()));
        tracing::error!(
            attempts = self.retry_config.max_retries + 1,
            error = %error,
            operation = operation_name,
            circuit_state = %self.circuit_breaker.state(),
            "LLM request failed after all retry attempts"
        );
        Err(error)
    }
}

#[async_trait]
impl LlmPort for ResilientLlmClient {
    async fn generate(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // Clone the inner Arc and request for the retry closure
        let inner = Arc::clone(&self.inner);
        self.execute_with_retry("generate", || {
            let inner = Arc::clone(&inner);
            let request = request.clone();
            async move { inner.generate(request).await }
        })
        .await
    }

    async fn generate_with_tools(
        &self,
        request: LlmRequest,
        tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, LlmError> {
        let inner = Arc::clone(&self.inner);
        self.execute_with_retry("generate_with_tools", || {
            let inner = Arc::clone(&inner);
            let request = request.clone();
            let tools = tools.clone();
            async move { inner.generate_with_tools(request, tools).await }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Mock LLM that fails a configurable number of times before succeeding
    struct FailingMockLlm {
        failures_remaining: AtomicU32,
        error_type: LlmError,
    }

    impl FailingMockLlm {
        fn new(failure_count: u32, error: LlmError) -> Self {
            Self {
                failures_remaining: AtomicU32::new(failure_count),
                error_type: error,
            }
        }
    }

    #[async_trait]
    impl LlmPort for FailingMockLlm {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            let remaining = self.failures_remaining.fetch_sub(1, Ordering::SeqCst);
            if remaining > 0 {
                Err(self.error_type.clone())
            } else {
                Ok(LlmResponse {
                    content: "Success!".to_string(),
                    tool_calls: vec![],
                    finish_reason: crate::infrastructure::ports::FinishReason::Stop,
                    usage: None,
                })
            }
        }

        async fn generate_with_tools(
            &self,
            request: LlmRequest,
            _tools: Vec<ToolDefinition>,
        ) -> Result<LlmResponse, LlmError> {
            self.generate(request).await
        }
    }

    #[tokio::test]
    async fn test_succeeds_without_retry() {
        let mock = Arc::new(FailingMockLlm::new(0, LlmError::RequestFailed("test".into())));
        let client = ResilientLlmClient::new(mock, RetryConfig::default());

        let request = LlmRequest::new(vec![]);
        let result = client.generate(request).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "Success!");
    }

    #[tokio::test]
    async fn test_succeeds_after_retry() {
        let mock = Arc::new(FailingMockLlm::new(2, LlmError::RequestFailed("transient".into())));
        let config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 1, // Fast for tests
            max_delay_ms: 10,
            jitter_factor: 0.0,
        };
        let client = ResilientLlmClient::new(mock, config);

        let request = LlmRequest::new(vec![]);
        let result = client.generate(request).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fails_after_max_retries() {
        let mock = Arc::new(FailingMockLlm::new(10, LlmError::RequestFailed("persistent".into())));
        let config = RetryConfig {
            max_retries: 2,
            base_delay_ms: 1,
            max_delay_ms: 10,
            jitter_factor: 0.0,
        };
        let client = ResilientLlmClient::new(mock, config);

        let request = LlmRequest::new(vec![]);
        let result = client.generate(request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_no_retry_on_auth_error() {
        let mock = Arc::new(FailingMockLlm::new(10, LlmError::RequestFailed("401 Unauthorized".into())));
        let mock_ref = Arc::clone(&mock);
        let config = RetryConfig {
            max_retries: 3,
            base_delay_ms: 1,
            max_delay_ms: 10,
            jitter_factor: 0.0,
        };
        let client = ResilientLlmClient::new(mock, config);

        let request = LlmRequest::new(vec![]);
        let result = client.generate(request).await;

        // Should fail immediately without retrying
        assert!(result.is_err());
        // Verify only 1 attempt was made (10 - 1 = 9 remaining)
        assert_eq!(
            mock_ref.failures_remaining.load(Ordering::SeqCst),
            9,
            "Auth error should not retry - expected 9 remaining failures after single attempt"
        );
    }

    #[test]
    fn test_exponential_backoff() {
        let config = RetryConfig {
            max_retries: 5,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
            jitter_factor: 0.0, // No jitter for predictable test
        };
        let client = ResilientLlmClient::new(
            Arc::new(FailingMockLlm::new(0, LlmError::RequestFailed("".into()))),
            config,
        );

        // Attempt 1: 1000 * 2^0 = 1000
        assert_eq!(client.calculate_delay(1), 1000);
        // Attempt 2: 1000 * 2^1 = 2000
        assert_eq!(client.calculate_delay(2), 2000);
        // Attempt 3: 1000 * 2^2 = 4000
        assert_eq!(client.calculate_delay(3), 4000);
        // Attempt 4: 1000 * 2^3 = 8000
        assert_eq!(client.calculate_delay(4), 8000);
        // Attempt 5: 1000 * 2^4 = 16000
        assert_eq!(client.calculate_delay(5), 16000);
        // Attempt 6: 1000 * 2^5 = 32000, but capped at 30000
        assert_eq!(client.calculate_delay(6), 30000);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_after_failures() {
        let mock = Arc::new(FailingMockLlm::new(100, LlmError::RequestFailed("persistent".into())));
        let retry_config = RetryConfig {
            max_retries: 0, // No retries - each call is one failure
            base_delay_ms: 1,
            max_delay_ms: 10,
            jitter_factor: 0.0,
        };
        let circuit_config = CircuitBreakerConfig {
            failure_threshold: 3,
            open_duration: Duration::from_secs(60),
            half_open_max_requests: 1,
        };
        let client = ResilientLlmClient::with_circuit_breaker(mock, retry_config, circuit_config);

        // First 3 failures should open the circuit
        let request = LlmRequest::new(vec![]);
        let _ = client.generate(request.clone()).await;
        assert_eq!(client.circuit_state(), CircuitState::Closed);
        let _ = client.generate(request.clone()).await;
        assert_eq!(client.circuit_state(), CircuitState::Closed);
        let _ = client.generate(request.clone()).await;
        assert_eq!(client.circuit_state(), CircuitState::Open);

        // Next request should be rejected by circuit breaker
        let result = client.generate(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circuit breaker is open"));
    }

    #[tokio::test]
    async fn test_circuit_breaker_success_resets_failures() {
        let mock = Arc::new(FailingMockLlm::new(2, LlmError::RequestFailed("transient".into())));
        let retry_config = RetryConfig {
            max_retries: 0,
            base_delay_ms: 1,
            max_delay_ms: 10,
            jitter_factor: 0.0,
        };
        let circuit_config = CircuitBreakerConfig {
            failure_threshold: 3,
            open_duration: Duration::from_secs(60),
            half_open_max_requests: 1,
        };
        let client = ResilientLlmClient::with_circuit_breaker(mock, retry_config, circuit_config);

        let request = LlmRequest::new(vec![]);

        // 2 failures
        let _ = client.generate(request.clone()).await;
        let _ = client.generate(request.clone()).await;

        // 1 success - should reset failure count
        let result = client.generate(request.clone()).await;
        assert!(result.is_ok());
        assert_eq!(client.circuit_state(), CircuitState::Closed);

        // Metrics should show reset
        let metrics = client.circuit_metrics();
        assert_eq!(metrics.consecutive_failures, 0);
    }

    #[tokio::test]
    async fn test_circuit_metrics() {
        let mock = Arc::new(FailingMockLlm::new(2, LlmError::RequestFailed("test".into())));
        let retry_config = RetryConfig {
            max_retries: 0,
            base_delay_ms: 1,
            max_delay_ms: 10,
            jitter_factor: 0.0,
        };
        let client = ResilientLlmClient::new(mock, retry_config);

        let request = LlmRequest::new(vec![]);

        let _ = client.generate(request.clone()).await; // fail
        let _ = client.generate(request.clone()).await; // fail
        let _ = client.generate(request.clone()).await; // success

        let metrics = client.circuit_metrics();
        assert_eq!(metrics.total_failures, 2);
        assert_eq!(metrics.total_successes, 1);
        assert_eq!(metrics.state, CircuitState::Closed);
    }
}
