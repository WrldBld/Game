//! Quantity value object for item management

use serde::{Deserialize, Serialize};

/// Result of a quantity subtraction operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuantityChangeResult {
    /// New quantity after subtraction
    Updated(u32),
    /// Item is fully depleted (quantity reached zero or below)
    Depleted,
}

impl QuantityChangeResult {
    /// Subtract an amount from a quantity
    pub fn subtract(current: u32, amount: u32) -> Self {
        if amount >= current {
            Self::Depleted
        } else {
            Self::Updated(current - amount)
        }
    }

    /// Check if this result indicates the item should be removed
    pub fn should_remove(&self) -> bool {
        matches!(self, Self::Depleted)
    }

    /// Get the new quantity, if not depleted
    pub fn new_quantity(&self) -> Option<u32> {
        match self {
            Self::Updated(qty) => Some(*qty),
            Self::Depleted => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subtract_partial() {
        assert_eq!(
            QuantityChangeResult::subtract(5, 3),
            QuantityChangeResult::Updated(2)
        );
    }

    #[test]
    fn test_subtract_exact() {
        assert_eq!(
            QuantityChangeResult::subtract(5, 5),
            QuantityChangeResult::Depleted
        );
    }

    #[test]
    fn test_subtract_overflow() {
        assert_eq!(
            QuantityChangeResult::subtract(5, 7),
            QuantityChangeResult::Depleted
        );
    }

    #[test]
    fn test_should_remove() {
        assert!(!QuantityChangeResult::Updated(2).should_remove());
        assert!(QuantityChangeResult::Depleted.should_remove());
    }

    #[test]
    fn test_new_quantity() {
        assert_eq!(QuantityChangeResult::Updated(2).new_quantity(), Some(2));
        assert_eq!(QuantityChangeResult::Depleted.new_quantity(), None);
    }
}
