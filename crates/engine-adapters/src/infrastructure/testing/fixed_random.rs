//! Fixed random port for deterministic testing.
//!
//! Returns values from a provided sequence, cycling if needed.
//! Thread-safe via atomic operations.

use wrldbldr_engine_ports::outbound::RandomPort;

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

    fn random_i64(&self) -> i64 {
        let idx = self
            .index
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let value = self.values[idx % self.values.len()];
        value as i64
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
