//! Item Service - Application service for item management and inventory operations
//!
//! This service provides use case implementations for managing items, including:
//! - Creating items in a world
//! - Adding items to character/PC inventories
//! - Item transfers between entities
//! - Container item management

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use wrldbldr_domain::entities::{AcquisitionMethod, InventoryItem, Item};
use wrldbldr_domain::error::DomainError;
use wrldbldr_domain::{ItemId, PlayerCharacterId, RegionId, WorldId};
use wrldbldr_engine_ports::outbound::{
    ItemRepositoryPort, ItemServicePort, PlayerCharacterRepositoryPort, RegionItemPort,
};

/// Request to create a new item
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct CreateItemRequest {
    pub name: String,
    pub description: Option<String>,
    pub item_type: Option<String>,
    pub is_unique: bool,
    pub properties: Option<String>,
    pub can_contain_items: bool,
    pub container_limit: Option<u32>,
}


/// Request to give an item to a player character
#[derive(Debug, Clone)]
pub struct GiveItemRequest {
    /// The item to give (will be created if None)
    pub item_id: Option<ItemId>,
    /// Item name (used if creating new item)
    pub item_name: String,
    /// Item description (used if creating new item)
    pub item_description: Option<String>,
    /// Recipient PC ID
    pub recipient_pc_id: PlayerCharacterId,
    /// Quantity to give
    pub quantity: u32,
    /// How the item was acquired
    pub acquisition_method: AcquisitionMethod,
}

/// Result of giving an item
#[derive(Debug, Clone)]
pub struct GiveItemResult {
    /// The item that was given
    pub item: Item,
    /// Whether a new item was created
    pub was_created: bool,
    /// The recipient PC ID
    pub recipient_pc_id: PlayerCharacterId,
    /// Quantity given
    pub quantity: u32,
}

/// Item service trait defining the application use cases
#[async_trait]
pub trait ItemService: Send + Sync {
    // -------------------------------------------------------------------------
    // Item CRUD
    // -------------------------------------------------------------------------

    /// Create a new item in a world
    async fn create_item(&self, world_id: WorldId, request: CreateItemRequest) -> Result<Item>;

    /// Get an item by ID
    async fn get_item(&self, item_id: ItemId) -> Result<Option<Item>>;

    /// List all items in a world
    async fn list_items(&self, world_id: WorldId) -> Result<Vec<Item>>;

    /// Update an item
    async fn update_item(&self, item: &Item) -> Result<()>;

    /// Delete an item
    async fn delete_item(&self, item_id: ItemId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Item Giving (LLM/DM workflow)
    // -------------------------------------------------------------------------

    /// Give an item to a player character
    ///
    /// This is the primary method for the LLM→DM approval→item creation flow.
    /// If `item_id` is None, creates a new item with the given name/description.
    /// If `item_id` is Some, uses the existing item.
    async fn give_item_to_pc(
        &self,
        world_id: WorldId,
        request: GiveItemRequest,
    ) -> Result<GiveItemResult>;

    /// Give an item to multiple PCs at once
    ///
    /// Creates one item per recipient (each gets their own instance).
    async fn give_item_to_multiple_pcs(
        &self,
        world_id: WorldId,
        item_name: String,
        item_description: Option<String>,
        recipient_pc_ids: Vec<PlayerCharacterId>,
        acquisition_method: AcquisitionMethod,
    ) -> Result<Vec<GiveItemResult>>;

    // -------------------------------------------------------------------------
    // Inventory Queries
    // -------------------------------------------------------------------------

    /// Get a PC's inventory
    async fn get_pc_inventory(&self, pc_id: PlayerCharacterId) -> Result<Vec<InventoryItem>>;

    /// Check if a PC has a specific item
    async fn pc_has_item(&self, pc_id: PlayerCharacterId, item_id: ItemId) -> Result<bool>;

    // -------------------------------------------------------------------------
    // Region Item Placement (DM workflow)
    // -------------------------------------------------------------------------

    /// Place an existing item into a region
    async fn place_item_in_region(&self, region_id: RegionId, item_id: ItemId) -> Result<()>;

    /// Create a new item and place it in a region
    async fn create_and_place_item(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        request: CreateItemRequest,
    ) -> Result<Item>;

    // -------------------------------------------------------------------------
    // Container Operations
    // -------------------------------------------------------------------------

    /// Add an item to a container with capacity validation
    ///
    /// Returns DomainError::ContainerFull if the container is at capacity,
    /// or DomainError::Constraint if the item cannot contain other items.
    async fn add_item_to_container(
        &self,
        container_id: ItemId,
        item_id: ItemId,
        quantity: u32,
    ) -> Result<(), DomainError>;
}

/// Default implementation of ItemService using port abstractions
#[derive(Clone)]
pub struct ItemServiceImpl {
    item_repository: Arc<dyn ItemRepositoryPort>,
    pc_repository: Arc<dyn PlayerCharacterRepositoryPort>,
    region_item: Arc<dyn RegionItemPort>,
}

impl ItemServiceImpl {
    /// Create a new ItemServiceImpl with the given repositories
    pub fn new(
        item_repository: Arc<dyn ItemRepositoryPort>,
        pc_repository: Arc<dyn PlayerCharacterRepositoryPort>,
        region_item: Arc<dyn RegionItemPort>,
    ) -> Self {
        Self {
            item_repository,
            pc_repository,
            region_item,
        }
    }

    /// Validate an item creation request
    fn validate_create_request(request: &CreateItemRequest) -> Result<()> {
        if request.name.trim().is_empty() {
            anyhow::bail!("Item name cannot be empty");
        }
        Ok(())
    }
}

#[async_trait]
impl ItemService for ItemServiceImpl {
    #[instrument(skip(self))]
    async fn create_item(&self, world_id: WorldId, request: CreateItemRequest) -> Result<Item> {
        Self::validate_create_request(&request)?;

        let item = Item {
            id: ItemId::new(),
            world_id,
            name: request.name,
            description: request.description,
            item_type: request.item_type,
            is_unique: request.is_unique,
            properties: request.properties,
            can_contain_items: request.can_contain_items,
            container_limit: request.container_limit,
        };

        self.item_repository
            .create(&item)
            .await
            .context("Failed to create item")?;

        info!(item_id = %item.id, item_name = %item.name, "Created item");
        Ok(item)
    }

    async fn get_item(&self, item_id: ItemId) -> Result<Option<Item>> {
        self.item_repository
            .get(item_id)
            .await
            .context("Failed to get item")
    }

    async fn list_items(&self, world_id: WorldId) -> Result<Vec<Item>> {
        self.item_repository
            .list(world_id)
            .await
            .context("Failed to list items")
    }

    async fn update_item(&self, item: &Item) -> Result<()> {
        self.item_repository
            .update(item)
            .await
            .context("Failed to update item")
    }

    async fn delete_item(&self, item_id: ItemId) -> Result<()> {
        self.item_repository
            .delete(item_id)
            .await
            .context("Failed to delete item")
    }

    #[instrument(skip(self))]
    async fn give_item_to_pc(
        &self,
        world_id: WorldId,
        request: GiveItemRequest,
    ) -> Result<GiveItemResult> {
        let (item, was_created) = if let Some(item_id) = request.item_id {
            // Use existing item
            let item = self
                .item_repository
                .get(item_id)
                .await
                .context("Failed to fetch item")?
                .ok_or_else(|| anyhow::anyhow!("Item not found: {}", item_id))?;
            (item, false)
        } else {
            // Create new item
            let item = self
                .create_item(
                    world_id,
                    CreateItemRequest {
                        name: request.item_name.clone(),
                        description: request.item_description.clone(),
                        ..Default::default()
                    },
                )
                .await?;
            (item, true)
        };

        // Add to PC inventory
        self.pc_repository
            .add_inventory_item(
                request.recipient_pc_id,
                item.id,
                request.quantity,
                false, // not equipped by default
                Some(request.acquisition_method),
            )
            .await
            .context("Failed to add item to PC inventory")?;

        info!(
            item_id = %item.id,
            item_name = %item.name,
            pc_id = %request.recipient_pc_id,
            quantity = request.quantity,
            "Gave item to PC"
        );

        Ok(GiveItemResult {
            item,
            was_created,
            recipient_pc_id: request.recipient_pc_id,
            quantity: request.quantity,
        })
    }

    #[instrument(skip(self))]
    async fn give_item_to_multiple_pcs(
        &self,
        world_id: WorldId,
        item_name: String,
        item_description: Option<String>,
        recipient_pc_ids: Vec<PlayerCharacterId>,
        acquisition_method: AcquisitionMethod,
    ) -> Result<Vec<GiveItemResult>> {
        let mut results = Vec::with_capacity(recipient_pc_ids.len());

        for pc_id in recipient_pc_ids {
            let result = self
                .give_item_to_pc(
                    world_id,
                    GiveItemRequest {
                        item_id: None, // Create new item for each recipient
                        item_name: item_name.clone(),
                        item_description: item_description.clone(),
                        recipient_pc_id: pc_id,
                        quantity: 1,
                        acquisition_method,
                    },
                )
                .await?;
            results.push(result);
        }

        debug!(
            item_name = %item_name,
            recipient_count = results.len(),
            "Gave item to multiple PCs"
        );

        Ok(results)
    }

    async fn get_pc_inventory(&self, pc_id: PlayerCharacterId) -> Result<Vec<InventoryItem>> {
        self.pc_repository
            .get_inventory(pc_id)
            .await
            .context("Failed to get PC inventory")
    }

    async fn pc_has_item(&self, pc_id: PlayerCharacterId, item_id: ItemId) -> Result<bool> {
        let item = self
            .pc_repository
            .get_inventory_item(pc_id, item_id)
            .await
            .context("Failed to check PC inventory")?;
        Ok(item.is_some())
    }

    #[instrument(skip(self))]
    async fn place_item_in_region(&self, region_id: RegionId, item_id: ItemId) -> Result<()> {
        self.region_item
            .add_item_to_region(region_id, item_id)
            .await
            .context("Failed to place item in region")?;

        info!(item_id = %item_id, region_id = %region_id, "Placed item in region");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn create_and_place_item(
        &self,
        world_id: WorldId,
        region_id: RegionId,
        request: CreateItemRequest,
    ) -> Result<Item> {
        // Create the item first
        let item = self.create_item(world_id, request).await?;

        // Then place it in the region
        self.place_item_in_region(region_id, item.id).await?;

        info!(
            item_id = %item.id,
            item_name = %item.name,
            region_id = %region_id,
            "Created and placed item in region"
        );

        Ok(item)
    }

    #[instrument(skip(self))]
    async fn add_item_to_container(
        &self,
        container_id: ItemId,
        item_id: ItemId,
        quantity: u32,
    ) -> Result<(), DomainError> {
        // Get container info for validation
        let info = self
            .item_repository
            .get_container_info(container_id)
            .await
            .map_err(|e| DomainError::constraint(format!("Failed to get container info: {}", e)))?;

        // Validate container can hold items
        if !info.can_contain_items {
            return Err(DomainError::constraint("Item cannot contain other items"));
        }

        // Validate capacity
        if let Some(max) = info.max_limit {
            if info.current_count >= max {
                return Err(DomainError::container_full(info.current_count, max));
            }
        }

        // Add item (repository is now pure data access)
        self.item_repository
            .add_item_to_container(container_id, item_id, quantity)
            .await
            .map_err(|e| DomainError::constraint(format!("Failed to add item to container: {}", e)))?;

        info!(
            container_id = %container_id,
            item_id = %item_id,
            quantity = quantity,
            "Added item to container"
        );

        Ok(())
    }
}

// =============================================================================
// ItemServicePort Implementation
// =============================================================================

#[async_trait]
impl ItemServicePort for ItemServiceImpl {
    async fn get_item(&self, id: ItemId) -> Result<Option<Item>> {
        ItemService::get_item(self, id).await
    }

    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Item>> {
        ItemService::list_items(self, world_id).await
    }

    async fn list_by_region(&self, region_id: RegionId) -> Result<Vec<Item>> {
        self.region_item
            .get_region_items(region_id)
            .await
            .context("Failed to list items by region")
    }
}
