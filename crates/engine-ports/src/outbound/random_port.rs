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
/// - `FixedRandomPort` in engine-adapters for deterministic testing (returns fixed values)
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
pub trait RandomPort: Send + Sync {
    /// Generate a random f64 in range [0.0, 1.0)
    fn random_f64(&self) -> f64;

    /// Generate a random i32 in range [min, max] (inclusive on both ends)
    fn random_range(&self, min: i32, max: i32) -> i32;
}
