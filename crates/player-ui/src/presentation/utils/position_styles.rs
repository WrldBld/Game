//! Character position styling utilities
//!
//! Extension trait for converting CharacterPosition to Tailwind CSS classes.
//! This keeps presentation/styling logic in the UI layer, following hexagonal
//! architecture principles.

use wrldbldr_player_ports::outbound::player_events::CharacterPosition;

/// Extension trait for CharacterPosition styling
pub trait CharacterPositionStyle {
    /// Get Tailwind CSS classes for positioning
    fn as_tailwind_classes(&self) -> &'static str;
}

impl CharacterPositionStyle for CharacterPosition {
    fn as_tailwind_classes(&self) -> &'static str {
        match self {
            CharacterPosition::Left => "left-[10%]",
            CharacterPosition::Center => "left-1/2 -translate-x-1/2",
            CharacterPosition::Right => "right-[10%]",
            // Both OffScreen and Unknown variants are hidden
            // Unknown handles forward compatibility when protocol adds new positions
            CharacterPosition::OffScreen | CharacterPosition::Unknown => "hidden",
        }
    }
}
