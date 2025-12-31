//! Repository ports - Interfaces for data persistence
//!
//! These traits define the contracts that infrastructure repositories must implement.
//! Application services depend on these traits, not concrete implementations.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use wrldbldr_domain::entities::WorkflowSlot;
use wrldbldr_domain::entities::{
    Act, CharacterSheetTemplate, GalleryAsset, GenerationBatch, Goal, InteractionRequirement,
    InteractionTargetType, InteractionTemplate, Item, NpcObservation, SheetTemplateId, Skill, Want,
    WorkflowConfiguration, World,
};
use wrldbldr_domain::value_objects::Relationship;
use wrldbldr_domain::{
    AssetId, BatchId, CharacterId, GoalId, InteractionId, ItemId, PlayerCharacterId, RegionId,
    RelationshipId, SceneId, SkillId, WantId, WorldId,
};

// =============================================================================
// Social Network DTOs
// =============================================================================

/// Representation of the social network graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialNetwork {
    pub characters: Vec<CharacterNode>,
    pub relationships: Vec<RelationshipEdge>,
}

/// A node in the social network (character)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterNode {
    pub id: String,
    pub name: String,
    pub archetype: String,
}

/// An edge in the social network (relationship)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEdge {
    pub from_id: String,
    pub to_id: String,
    pub relationship_type: String,
    pub sentiment: f32,
}

// =============================================================================
// World Repository Port
// =============================================================================

/// Repository port for World aggregate operations
#[async_trait]
pub trait WorldRepositoryPort: Send + Sync {
    /// Create a new world
    async fn create(&self, world: &World) -> Result<()>;

    /// Get a world by ID
    async fn get(&self, id: WorldId) -> Result<Option<World>>;

    /// List all worlds
    async fn list(&self) -> Result<Vec<World>>;

    /// Update a world
    async fn update(&self, world: &World) -> Result<()>;

    /// Delete a world and all its contents (cascading)
    async fn delete(&self, id: WorldId) -> Result<()>;

    /// Create an act within a world
    async fn create_act(&self, act: &Act) -> Result<()>;

    /// Get acts for a world
    async fn get_acts(&self, world_id: WorldId) -> Result<Vec<Act>>;
}

// =============================================================================
// Player Character Repository Port - REMOVED (use ISP traits instead)
// =============================================================================
//
// PlayerCharacterRepositoryPort has been split into 4 focused traits following ISP:
// - PlayerCharacterCrudPort: Core CRUD operations (5 methods)
// - PlayerCharacterQueryPort: Query/lookup operations (4 methods)
// - PlayerCharacterPositionPort: Position/movement operations (3 methods)
// - PlayerCharacterInventoryPort: Inventory management (5 methods)
//
// See: crate::outbound::player_character_repository

// =============================================================================
// Location Repository Port - REMOVED (use ISP traits instead)
// =============================================================================
//
// LocationRepositoryPort has been split into 4 focused traits following ISP:
// - LocationCrudPort: Core CRUD operations (5 methods)
// - LocationHierarchyPort: Parent-child relationships (4 methods)
// - LocationConnectionPort: Navigation connections (5 methods)
// - LocationMapPort: Grid maps and regions (5 methods)
//
// See: crate::outbound::location_repository

// =============================================================================
// Scene Repository Port - REMOVED (use ISP traits instead)
// =============================================================================
//
// SceneRepositoryPort has been split into 5 focused traits following ISP:
// - SceneCrudPort: Core CRUD operations (5 methods)
// - SceneQueryPort: Query by act/location (2 methods)
// - SceneLocationPort: AT_LOCATION edge management (2 methods)
// - SceneFeaturedCharacterPort: FEATURES_CHARACTER edges (5 methods)
// - SceneCompletionPort: COMPLETED_SCENE tracking (3 methods)
//
// See: crate::outbound::scene_repository

// =============================================================================
// Game Flag Repository Port
// =============================================================================

/// Repository port for Game Flag operations
///
/// Flags are persistent boolean values that track game state.
/// They can be world-scoped (shared) or PC-scoped (per-character).
#[async_trait]
pub trait FlagRepositoryPort: Send + Sync {
    // -------------------------------------------------------------------------
    // World-scoped Flags
    // -------------------------------------------------------------------------

    /// Set a world-scoped flag
    async fn set_world_flag(&self, world_id: WorldId, flag_name: &str, value: bool) -> Result<()>;

    /// Get a world-scoped flag value (returns false if not set)
    async fn get_world_flag(&self, world_id: WorldId, flag_name: &str) -> Result<bool>;

    /// Get all flags for a world
    async fn get_world_flags(&self, world_id: WorldId) -> Result<Vec<(String, bool)>>;

    // -------------------------------------------------------------------------
    // PC-scoped Flags
    // -------------------------------------------------------------------------

    /// Set a PC-scoped flag
    async fn set_pc_flag(
        &self,
        pc_id: PlayerCharacterId,
        flag_name: &str,
        value: bool,
    ) -> Result<()>;

    /// Get a PC-scoped flag value (returns false if not set)
    async fn get_pc_flag(&self, pc_id: PlayerCharacterId, flag_name: &str) -> Result<bool>;

    /// Get all flags for a player character
    async fn get_pc_flags(&self, pc_id: PlayerCharacterId) -> Result<Vec<(String, bool)>>;
}

// =============================================================================
// Interaction Repository Port
// =============================================================================

/// Repository port for InteractionTemplate operations
#[async_trait]
pub trait InteractionRepositoryPort: Send + Sync {
    // -------------------------------------------------------------------------
    // Core CRUD
    // -------------------------------------------------------------------------

    /// Create a new interaction template
    async fn create(&self, interaction: &InteractionTemplate) -> Result<()>;

    /// Get an interaction template by ID
    async fn get(&self, id: InteractionId) -> Result<Option<InteractionTemplate>>;

    /// List interaction templates in a scene
    async fn list_by_scene(&self, scene_id: SceneId) -> Result<Vec<InteractionTemplate>>;

    /// Update an interaction template
    async fn update(&self, interaction: &InteractionTemplate) -> Result<()>;

    /// Delete an interaction template
    async fn delete(&self, id: InteractionId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Target Edges (TARGETS_CHARACTER, TARGETS_ITEM, TARGETS_REGION)
    // -------------------------------------------------------------------------

    /// Set interaction target to a character
    async fn set_target_character(
        &self,
        interaction_id: InteractionId,
        character_id: CharacterId,
    ) -> Result<()>;

    /// Set interaction target to an item
    async fn set_target_item(&self, interaction_id: InteractionId, item_id: ItemId) -> Result<()>;

    /// Set interaction target to a backdrop region
    async fn set_target_region(
        &self,
        interaction_id: InteractionId,
        region_id: RegionId,
    ) -> Result<()>;

    /// Remove any target from the interaction
    async fn remove_target(&self, interaction_id: InteractionId) -> Result<()>;

    /// Get the target type and ID for an interaction
    async fn get_target(
        &self,
        interaction_id: InteractionId,
    ) -> Result<Option<(InteractionTargetType, String)>>;

    // -------------------------------------------------------------------------
    // Requirement Edges (REQUIRES_ITEM, REQUIRES_CHARACTER_PRESENT)
    // -------------------------------------------------------------------------

    /// Add a required item for the interaction
    async fn add_required_item(
        &self,
        interaction_id: InteractionId,
        item_id: ItemId,
        requirement: &InteractionRequirement,
    ) -> Result<()>;

    /// Remove a required item
    async fn remove_required_item(
        &self,
        interaction_id: InteractionId,
        item_id: ItemId,
    ) -> Result<()>;

    /// Add a required character presence
    async fn add_required_character(
        &self,
        interaction_id: InteractionId,
        character_id: CharacterId,
    ) -> Result<()>;

    /// Remove a required character presence
    async fn remove_required_character(
        &self,
        interaction_id: InteractionId,
        character_id: CharacterId,
    ) -> Result<()>;
}

// =============================================================================
// Relationship Repository Port
// =============================================================================

/// Repository port for Relationship (graph edge) operations
#[async_trait]
pub trait RelationshipRepositoryPort: Send + Sync {
    /// Create a relationship between characters
    async fn create(&self, relationship: &Relationship) -> Result<()>;

    /// Get a relationship by ID
    async fn get(&self, id: RelationshipId) -> Result<Option<Relationship>>;

    /// Get all relationships for a character (outgoing)
    async fn get_for_character(&self, character_id: CharacterId) -> Result<Vec<Relationship>>;

    /// Update a relationship
    async fn update(&self, relationship: &Relationship) -> Result<()>;

    /// Delete a relationship by ID
    async fn delete(&self, id: RelationshipId) -> Result<()>;

    /// Get the social network graph for a world
    async fn get_social_network(&self, world_id: WorldId) -> Result<SocialNetwork>;
}

// =============================================================================
// Skill Repository Port
// =============================================================================

/// Repository port for Skill operations
#[async_trait]
pub trait SkillRepositoryPort: Send + Sync {
    /// Create a skill
    async fn create(&self, skill: &Skill) -> Result<()>;

    /// Get a skill by ID
    async fn get(&self, id: SkillId) -> Result<Option<Skill>>;

    /// List skills for a world
    async fn list(&self, world_id: WorldId) -> Result<Vec<Skill>>;

    /// Update a skill
    async fn update(&self, skill: &Skill) -> Result<()>;

    /// Delete a skill
    async fn delete(&self, id: SkillId) -> Result<()>;
}

// =============================================================================
// Item Repository Port
// =============================================================================

/// Repository port for Item operations
#[async_trait]
pub trait ItemRepositoryPort: Send + Sync {
    // -------------------------------------------------------------------------
    // Core CRUD
    // -------------------------------------------------------------------------

    /// Create a new item
    async fn create(&self, item: &Item) -> Result<()>;

    /// Get an item by ID
    async fn get(&self, id: ItemId) -> Result<Option<Item>>;

    /// List all items in a world
    async fn list(&self, world_id: WorldId) -> Result<Vec<Item>>;

    /// Update an item
    async fn update(&self, item: &Item) -> Result<()>;

    /// Delete an item
    async fn delete(&self, id: ItemId) -> Result<()>;

    /// Get items by type
    async fn get_by_type(&self, world_id: WorldId, item_type: &str) -> Result<Vec<Item>>;

    // -------------------------------------------------------------------------
    // Container Operations (CONTAINS edge between Items)
    // -------------------------------------------------------------------------

    /// Add an item to a container
    ///
    /// Returns error if container is full or cannot hold items.
    async fn add_item_to_container(
        &self,
        container_id: ItemId,
        item_id: ItemId,
        quantity: u32,
    ) -> Result<()>;

    /// Get all items contained in a container
    async fn get_container_contents(&self, container_id: ItemId) -> Result<Vec<(Item, u32)>>;

    /// Remove an item (or quantity) from a container
    async fn remove_item_from_container(
        &self,
        container_id: ItemId,
        item_id: ItemId,
        quantity: u32,
    ) -> Result<()>;

    /// Get container information including capacity
    ///
    /// Returns container info with current count, max limit, and whether it can contain items.
    async fn get_container_info(&self, container_id: ItemId) -> Result<ContainerInfo>;
}

/// Information about a container's capacity and state
#[derive(Debug, Clone)]
pub struct ContainerInfo {
    /// Whether the item can contain other items
    pub can_contain_items: bool,
    /// Current number of items in the container
    pub current_count: u32,
    /// Maximum number of items (None = unlimited)
    pub max_limit: Option<u32>,
}

// =============================================================================
// Goal Repository Port
// =============================================================================

/// Repository port for Goal operations
#[async_trait]
pub trait GoalRepositoryPort: Send + Sync {
    /// Create a new goal
    async fn create(&self, goal: &Goal) -> Result<()>;

    /// Get a goal by ID
    async fn get(&self, id: GoalId) -> Result<Option<Goal>>;

    /// List all goals in a world
    async fn list(&self, world_id: WorldId) -> Result<Vec<Goal>>;

    /// Update a goal
    async fn update(&self, goal: &Goal) -> Result<()>;

    /// Delete a goal
    async fn delete(&self, id: GoalId) -> Result<()>;
}

// =============================================================================
// Want Repository Port
// =============================================================================

/// Repository port for standalone Want operations
#[async_trait]
pub trait WantRepositoryPort: Send + Sync {
    /// Get a want by ID
    async fn get(&self, id: WantId) -> Result<Option<Want>>;

    /// Get the target of a want (returns type and ID)
    async fn get_target(&self, want_id: WantId) -> Result<Option<(String, String)>>;
}

// =============================================================================
// Asset Repository Port
// =============================================================================

/// Repository port for GalleryAsset operations
#[async_trait]
pub trait AssetRepositoryPort: Send + Sync {
    /// Create an asset
    async fn create(&self, asset: &GalleryAsset) -> Result<()>;

    /// Get an asset by ID
    async fn get(&self, id: AssetId) -> Result<Option<GalleryAsset>>;

    /// List assets for an entity
    async fn list_for_entity(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Vec<GalleryAsset>>;

    /// Activate an asset (set as current for its slot)
    async fn activate(&self, id: AssetId) -> Result<()>;

    /// Update an asset's label
    async fn update_label(&self, id: AssetId, label: Option<String>) -> Result<()>;

    /// Delete an asset
    async fn delete(&self, id: AssetId) -> Result<()>;

    /// Create a generation batch
    async fn create_batch(&self, batch: &GenerationBatch) -> Result<()>;

    /// Get a batch by ID
    async fn get_batch(&self, id: BatchId) -> Result<Option<GenerationBatch>>;

    /// Update batch status
    async fn update_batch_status(
        &self,
        id: BatchId,
        status: &wrldbldr_domain::entities::BatchStatus,
    ) -> Result<()>;

    /// Update the assets associated with a batch
    async fn update_batch_assets(&self, id: BatchId, assets: &[AssetId]) -> Result<()>;

    /// List all active (queued or generating) batches for a specific world
    async fn list_active_batches_by_world(&self, world_id: WorldId)
        -> Result<Vec<GenerationBatch>>;

    /// List batches ready for selection
    async fn list_ready_batches(&self) -> Result<Vec<GenerationBatch>>;

    /// Delete a batch
    async fn delete_batch(&self, id: BatchId) -> Result<()>;
}

// =============================================================================
// StoryEvent Repository Port - MOVED to story_event_repository module
// =============================================================================
//
// StoryEventRepositoryPort has been split into 4 focused traits following ISP:
// - StoryEventCrudPort (7 methods) - Core CRUD + state management
// - StoryEventEdgePort (15 methods) - Edge relationship management
// - StoryEventQueryPort (10 methods) - Query operations
// - StoryEventDialoguePort (2 methods) - Dialogue-specific operations
//
// The super-trait StoryEventRepositoryPort is retained for backward compatibility.
// See: crate::outbound::story_event_repository

// =============================================================================
// NarrativeEvent Repository Port - MOVED to narrative_event_repository module
// =============================================================================
//
// NarrativeEventRepositoryPort has been split into 4 focused traits following ISP:
// - NarrativeEventCrudPort (12 methods) - Core CRUD + state management
// - NarrativeEventTiePort (9 methods) - Scene/Location/Act relationships
// - NarrativeEventNpcPort (5 methods) - Featured NPC management
// - NarrativeEventQueryPort (4 methods) - Query by relationships
//
// The super-trait NarrativeEventRepositoryPort is retained for backward compatibility.
// See: crate::outbound::narrative_event_repository

// =============================================================================
// EventChain Repository Port - REMOVED (use ISP traits instead)
// =============================================================================
//
// EventChainRepositoryPort has been split into 4 focused traits following ISP:
// - EventChainCrudPort: Core CRUD operations (4 methods)
// - EventChainQueryPort: Query/lookup operations (4 methods)
// - EventChainMembershipPort: Event membership management (3 methods)
// - EventChainStatePort: Status and state management (5 methods)
//
// See: crate::outbound::event_chain_repository

// =============================================================================
// SheetTemplate Repository Port
// =============================================================================

/// Repository port for CharacterSheetTemplate operations
#[async_trait]
pub trait SheetTemplateRepositoryPort: Send + Sync {
    /// Create a new sheet template
    async fn create(&self, template: &CharacterSheetTemplate) -> Result<()>;

    /// Get a sheet template by ID
    async fn get(&self, id: &SheetTemplateId) -> Result<Option<CharacterSheetTemplate>>;

    /// Get the default template for a world
    async fn get_default_for_world(
        &self,
        world_id: &WorldId,
    ) -> Result<Option<CharacterSheetTemplate>>;

    /// List all templates for a world
    async fn list_by_world(&self, world_id: &WorldId) -> Result<Vec<CharacterSheetTemplate>>;

    /// Update a sheet template
    async fn update(&self, template: &CharacterSheetTemplate) -> Result<()>;

    /// Delete a sheet template
    async fn delete(&self, id: &SheetTemplateId) -> Result<()>;

    /// Delete all templates for a world
    async fn delete_all_for_world(&self, world_id: &WorldId) -> Result<()>;

    /// Check if a world has any templates
    async fn has_templates(&self, world_id: &WorldId) -> Result<bool>;
}

// =============================================================================
// Workflow Repository Port
// =============================================================================

/// Repository port for WorkflowConfiguration operations
#[async_trait]
pub trait WorkflowRepositoryPort: Send + Sync {
    /// Save a workflow configuration (create or update)
    async fn save(&self, config: &WorkflowConfiguration) -> Result<()>;

    /// Get a workflow configuration by slot
    async fn get_by_slot(&self, slot: WorkflowSlot) -> Result<Option<WorkflowConfiguration>>;

    /// Delete a workflow configuration by slot
    async fn delete_by_slot(&self, slot: WorkflowSlot) -> Result<bool>;

    /// List all workflow configurations
    async fn list_all(&self) -> Result<Vec<WorkflowConfiguration>>;
}

// =============================================================================
// Region Repository Port - REMOVED (use ISP traits instead)
// =============================================================================
//
// RegionRepositoryPort has been split into 5 focused traits following ISP:
// - RegionCrudPort: Core CRUD operations (5 methods)
// - RegionConnectionPort: Region-to-region connections (4 methods)
// - RegionExitPort: Region-to-location exits (3 methods)
// - RegionNpcPort: NPC relationship queries (1 method)
// - RegionItemPort: Item placement in regions (3 stub methods)
//
// See: crate::outbound::region_repository

// =============================================================================
// Observation Repository Port
// =============================================================================

/// Repository port for NPC Observation operations
///
/// Observations track when a PC has seen/met/heard about an NPC.
/// Used for scene conditions (`KnowsCharacter`) and the Known NPCs panel.
#[async_trait]
pub trait ObservationRepositoryPort: Send + Sync {
    /// Create or update an observation (upsert)
    ///
    /// If the PC already has an observation for this NPC, it will be updated.
    async fn upsert(&self, observation: &NpcObservation) -> Result<()>;

    /// Get all observations for a PC
    async fn get_for_pc(&self, pc_id: PlayerCharacterId) -> Result<Vec<NpcObservation>>;

    /// Get the latest observation of a specific NPC by a PC
    async fn get_latest(
        &self,
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
    ) -> Result<Option<NpcObservation>>;

    /// Check if a PC has observed a specific NPC
    ///
    /// Returns true if any observation exists (regardless of reveal status).
    async fn has_observed(&self, pc_id: PlayerCharacterId, npc_id: CharacterId) -> Result<bool> {
        Ok(self.get_latest(pc_id, npc_id).await?.is_some())
    }

    /// Delete an observation (removes the OBSERVED edge between PC and NPC)
    async fn delete(&self, pc_id: PlayerCharacterId, npc_id: CharacterId) -> Result<()>;
}

// =============================================================================
// Repository Provider Port (Facade)
// =============================================================================
