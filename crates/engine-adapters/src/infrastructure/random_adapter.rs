//! Thread-safe random number generation adapter.
//!
//! Implements `RandomPort` using `rand::thread_rng()`.

use rand::Rng;
use wrldbldr_engine_ports::outbound::RandomPort;

/// Production random number generator using thread-local RNG.
///
/// This adapter wraps `rand::thread_rng()` to implement the `RandomPort` trait,
/// enabling clean hexagonal architecture where the domain layer doesn't directly
/// depend on `rand`.
#[derive(Debug, Clone, Default)]
pub struct ThreadRngAdapter;

impl ThreadRngAdapter {
    /// Create a new ThreadRngAdapter.
    pub fn new() -> Self {
        Self
    }
}

impl RandomPort for ThreadRngAdapter {
    fn random_f64(&self) -> f64 {
        rand::thread_rng().gen()
    }

    fn random_range(&self, min: i32, max: i32) -> i32 {
        rand::thread_rng().gen_range(min..=max)
    }

    fn random_i64(&self) -> i64 {
        rand::thread_rng().gen()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_range_bounds() {
        let rng = ThreadRngAdapter::new();
        for _ in 0..100 {
            let value = rng.random_range(1, 20);
            assert!(value >= 1 && value <= 20, "Value {} out of range", value);
        }
    }

    #[test]
    fn test_random_f64_bounds() {
        let rng = ThreadRngAdapter::new();
        for _ in 0..100 {
            let value = rng.random_f64();
            assert!(value >= 0.0 && value < 1.0, "Value {} out of range", value);
        }
    }
}
