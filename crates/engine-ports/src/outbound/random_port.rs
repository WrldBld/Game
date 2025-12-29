//! Random number generation port for engine-side services.
//!
//! This port abstracts random number generation, enabling:
//! - Deterministic testing with mock RNG
//! - Reproducible game scenarios
//! - Clean hexagonal architecture (no I/O in domain layer)
//!
//! # Example
//!
//! ```ignore
//! use wrldbldr_engine_ports::outbound::RandomPort;
//!
//! fn roll_dice(rng: &dyn RandomPort, dice_count: u8, die_size: u8) -> Vec<i32> {
//!     (0..dice_count)
//!         .map(|_| rng.random_range(1, die_size as i32))
//!         .collect()
//! }
//! ```

/// Random number generation abstraction for engine-side services.
///
/// Modeled after `player-ports/src/outbound/platform.rs::RandomProvider`.
///
/// # Implementations
///
/// - `ThreadRngAdapter` in engine-adapters (production, uses `rand::thread_rng()`)
/// - `MockRandomPort` via mockall (testing)
/// - `FixedRandomPort` for deterministic testing (returns fixed values)
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
pub trait RandomPort: Send + Sync {
    /// Generate a random f64 in range [0.0, 1.0)
    fn random_f64(&self) -> f64;

    /// Generate a random i32 in range [min, max] (inclusive on both ends)
    fn random_range(&self, min: i32, max: i32) -> i32;
}

/// Fixed random port for deterministic testing.
///
/// Returns values from a provided sequence, cycling if needed.
/// Thread-safe via atomic operations.
#[derive(Debug)]
pub struct FixedRandomPort {
    values: Vec<i32>,
    index: std::sync::atomic::AtomicUsize,
}

impl Clone for FixedRandomPort {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            // Reset index on clone for predictable behavior
            index: std::sync::atomic::AtomicUsize::new(
                self.index.load(std::sync::atomic::Ordering::SeqCst),
            ),
        }
    }
}

impl FixedRandomPort {
    /// Create a new FixedRandomPort with the given sequence of values.
    pub fn new(values: Vec<i32>) -> Self {
        Self {
            values,
            index: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Create a FixedRandomPort that always returns the same value.
    pub fn constant(value: i32) -> Self {
        Self::new(vec![value])
    }
}

impl RandomPort for FixedRandomPort {
    fn random_f64(&self) -> f64 {
        let idx = self
            .index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let value = self.values[idx % self.values.len()];
        // Normalize to [0.0, 1.0) based on a reasonable max
        (value as f64 / 100.0).clamp(0.0, 0.999999)
    }

    fn random_range(&self, min: i32, max: i32) -> i32 {
        let idx = self
            .index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let value = self.values[idx % self.values.len()];
        // Clamp to the requested range
        value.clamp(min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_random_port_constant() {
        let rng = FixedRandomPort::constant(10);
        assert_eq!(rng.random_range(1, 20), 10);
        assert_eq!(rng.random_range(1, 20), 10);
        assert_eq!(rng.random_range(1, 6), 6); // Clamped to max
        assert_eq!(rng.random_range(15, 20), 15); // Clamped to min
    }

    #[test]
    fn test_fixed_random_port_sequence() {
        let rng = FixedRandomPort::new(vec![1, 5, 10, 20]);
        assert_eq!(rng.random_range(1, 20), 1);
        assert_eq!(rng.random_range(1, 20), 5);
        assert_eq!(rng.random_range(1, 20), 10);
        assert_eq!(rng.random_range(1, 20), 20);
        // Cycles back
        assert_eq!(rng.random_range(1, 20), 1);
    }
}
