//! Inventory operation result types.

/// Result of an inventory operation.
#[derive(Debug, Clone)]
pub struct InventoryActionResult {
    pub item_name: String,
    pub quantity: u32,
}
