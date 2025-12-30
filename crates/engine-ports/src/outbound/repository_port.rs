//! Repository ports - Interfaces for data persistence
//!
//! These traits define the contracts that infrastructure repositories must implement.
//! Application services depend on these traits, not concrete implementations.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use wrldbldr_domain::entities::WorkflowSlot;
use wrldbldr_domain::entities::{
    AcquisitionMethod, Act, ChainStatus, Character, CharacterSheetTemplate,
    EventChain, GalleryAsset, GenerationBatch, Goal, InteractionRequirement,
    InteractionTargetType, InteractionTemplate, InventoryItem, Item, Location, LocationConnection,
    NpcObservation, PlayerCharacter, Region, RegionConnection, RegionExit, Scene, SceneCharacter,
    SheetTemplateId, Skill, Want, WorkflowConfiguration, World,
};
use wrldbldr_domain::value_objects::{RegionRelationshipType, Relationship};
use wrldbldr_domain::{
    ActId, AssetId, BatchId, CharacterId, EventChainId, GoalId, GridMapId,
    InteractionId, ItemId, LocationId, NarrativeEventId, PlayerCharacterId, RegionId,
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
// Player Character Repository Port
// =============================================================================

/// Repository port for PlayerCharacter operations
#[async_trait]
pub trait PlayerCharacterRepositoryPort: Send + Sync {
    /// Create a new player character
    async fn create(&self, pc: &PlayerCharacter) -> Result<()>;

    /// Get a player character by ID
    async fn get(&self, id: PlayerCharacterId) -> Result<Option<PlayerCharacter>>;

    /// Get all player characters at a specific location
    async fn get_by_location(&self, location_id: LocationId) -> Result<Vec<PlayerCharacter>>;

    /// Get all player characters for a user in a world (for PC selection)
    async fn get_by_user_and_world(
        &self,
        user_id: &str,
        world_id: WorldId,
    ) -> Result<Vec<PlayerCharacter>>;

    /// Get all player characters in a world
    async fn get_all_by_world(&self, world_id: WorldId) -> Result<Vec<PlayerCharacter>>;

    /// Get all unbound player characters for a user (no session)
    async fn get_unbound_by_user(&self, user_id: &str) -> Result<Vec<PlayerCharacter>>;

    /// Update a player character
    async fn update(&self, pc: &PlayerCharacter) -> Result<()>;

    /// Update a player character's location (clears region)
    async fn update_location(&self, id: PlayerCharacterId, location_id: LocationId) -> Result<()>;

    /// Update a player character's region (within current location)
    async fn update_region(&self, id: PlayerCharacterId, region_id: RegionId) -> Result<()>;

    /// Update both location and region at once
    async fn update_position(
        &self,
        id: PlayerCharacterId,
        location_id: LocationId,
        region_id: Option<RegionId>,
    ) -> Result<()>;

    /// Unbind a player character from its session
    async fn unbind_from_session(&self, id: PlayerCharacterId) -> Result<()>;

    /// Delete a player character
    async fn delete(&self, id: PlayerCharacterId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Inventory (POSSESSES edges to Items)
    // -------------------------------------------------------------------------

    /// Add an item to PC's inventory (creates POSSESSES edge)
    async fn add_inventory_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
        quantity: u32,
        is_equipped: bool,
        acquisition_method: Option<AcquisitionMethod>,
    ) -> Result<()>;

    /// Get all items in PC's inventory
    async fn get_inventory(&self, pc_id: PlayerCharacterId) -> Result<Vec<InventoryItem>>;

    /// Get a specific item from PC's inventory
    async fn get_inventory_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
    ) -> Result<Option<InventoryItem>>;

    /// Update quantity/equipped status of item in PC's inventory
    async fn update_inventory_item(
        &self,
        pc_id: PlayerCharacterId,
        item_id: ItemId,
        quantity: u32,
        is_equipped: bool,
    ) -> Result<()>;

    /// Remove an item from PC's inventory (deletes POSSESSES edge)
    async fn remove_inventory_item(&self, pc_id: PlayerCharacterId, item_id: ItemId) -> Result<()>;
}

// =============================================================================
// Location Repository Port
// =============================================================================

/// Repository port for Location operations
#[async_trait]
pub trait LocationRepositoryPort: Send + Sync {
    // -------------------------------------------------------------------------
    // Core CRUD
    // -------------------------------------------------------------------------

    /// Create a new location
    async fn create(&self, location: &Location) -> Result<()>;

    /// Get a location by ID
    async fn get(&self, id: LocationId) -> Result<Option<Location>>;

    /// List all locations in a world
    async fn list(&self, world_id: WorldId) -> Result<Vec<Location>>;

    /// Update a location
    async fn update(&self, location: &Location) -> Result<()>;

    /// Delete a location
    async fn delete(&self, id: LocationId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Location Hierarchy (CONTAINS_LOCATION edges)
    // -------------------------------------------------------------------------

    /// Set a location's parent (creates CONTAINS_LOCATION edge)
    async fn set_parent(&self, child_id: LocationId, parent_id: LocationId) -> Result<()>;

    /// Remove a location's parent (deletes CONTAINS_LOCATION edge)
    async fn remove_parent(&self, child_id: LocationId) -> Result<()>;

    /// Get a location's parent
    async fn get_parent(&self, location_id: LocationId) -> Result<Option<Location>>;

    /// Get a location's children
    async fn get_children(&self, location_id: LocationId) -> Result<Vec<Location>>;

    // -------------------------------------------------------------------------
    // Location Connections (CONNECTED_TO edges)
    // -------------------------------------------------------------------------

    /// Create a connection between locations
    async fn create_connection(&self, connection: &LocationConnection) -> Result<()>;

    /// Get all connections from a location
    async fn get_connections(&self, location_id: LocationId) -> Result<Vec<LocationConnection>>;

    /// Update a connection's properties
    async fn update_connection(&self, connection: &LocationConnection) -> Result<()>;

    /// Delete a connection between locations
    async fn delete_connection(&self, from: LocationId, to: LocationId) -> Result<()>;

    /// Unlock a connection (set is_locked = false)
    async fn unlock_connection(&self, from: LocationId, to: LocationId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Grid Map (HAS_TACTICAL_MAP edge)
    // -------------------------------------------------------------------------

    /// Set a location's tactical map
    async fn set_grid_map(&self, location_id: LocationId, grid_map_id: GridMapId) -> Result<()>;

    /// Remove a location's tactical map
    async fn remove_grid_map(&self, location_id: LocationId) -> Result<()>;

    /// Get a location's tactical map ID
    async fn get_grid_map_id(&self, location_id: LocationId) -> Result<Option<GridMapId>>;

    // -------------------------------------------------------------------------
    // Regions (HAS_REGION edges to Region nodes)
    // -------------------------------------------------------------------------

    /// Create a region within a location
    async fn create_region(&self, location_id: LocationId, region: &Region) -> Result<()>;

    /// Get all regions in a location
    async fn get_regions(&self, location_id: LocationId) -> Result<Vec<Region>>;
}

// =============================================================================
// Scene Repository Port
// =============================================================================

/// Repository port for Scene operations
#[async_trait]
pub trait SceneRepositoryPort: Send + Sync {
    // -------------------------------------------------------------------------
    // Core CRUD
    // -------------------------------------------------------------------------

    /// Create a new scene
    async fn create(&self, scene: &Scene) -> Result<()>;

    /// Get a scene by ID
    async fn get(&self, id: SceneId) -> Result<Option<Scene>>;

    /// List scenes by act
    async fn list_by_act(&self, act_id: ActId) -> Result<Vec<Scene>>;

    /// List scenes by location (via AT_LOCATION edge)
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Scene>>;

    /// Update a scene
    async fn update(&self, scene: &Scene) -> Result<()>;

    /// Delete a scene
    async fn delete(&self, id: SceneId) -> Result<()>;

    /// Update directorial notes for a scene
    async fn update_directorial_notes(&self, id: SceneId, notes: &str) -> Result<()>;

    // -------------------------------------------------------------------------
    // Location (AT_LOCATION edge)
    // -------------------------------------------------------------------------

    /// Set scene's location (creates AT_LOCATION edge)
    async fn set_location(&self, scene_id: SceneId, location_id: LocationId) -> Result<()>;

    /// Get scene's location
    async fn get_location(&self, scene_id: SceneId) -> Result<Option<LocationId>>;

    // -------------------------------------------------------------------------
    // Featured Characters (FEATURES_CHARACTER edges)
    // -------------------------------------------------------------------------

    /// Add a featured character to the scene
    async fn add_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
        scene_char: &SceneCharacter,
    ) -> Result<()>;

    /// Get all featured characters for a scene
    async fn get_featured_characters(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<(CharacterId, SceneCharacter)>>;

    /// Update a featured character's role/cue
    async fn update_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
        scene_char: &SceneCharacter,
    ) -> Result<()>;

    /// Remove a featured character from the scene
    async fn remove_featured_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
    ) -> Result<()>;

    /// Get scenes featuring a specific character
    async fn get_scenes_for_character(&self, character_id: CharacterId) -> Result<Vec<Scene>>;

    // -------------------------------------------------------------------------
    // Scene Completion Tracking (COMPLETED_SCENE edge)
    // -------------------------------------------------------------------------

    /// Mark a scene as completed by a player character
    async fn mark_scene_completed(&self, pc_id: PlayerCharacterId, scene_id: SceneId)
        -> Result<()>;

    /// Check if a player character has completed a scene
    async fn is_scene_completed(&self, pc_id: PlayerCharacterId, scene_id: SceneId)
        -> Result<bool>;

    /// Get all scenes completed by a player character
    async fn get_completed_scenes(&self, pc_id: PlayerCharacterId) -> Result<Vec<SceneId>>;
}

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

    /// Get container capacity (current count, max limit)
    ///
    /// Returns (current_count, Some(max)) if limited, (current_count, None) if unlimited.
    async fn get_container_capacity(&self, container_id: ItemId) -> Result<(u32, Option<u32>)>;
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
// EventChain Repository Port
// =============================================================================

/// Repository port for EventChain operations
#[async_trait]
pub trait EventChainRepositoryPort: Send + Sync {
    /// Create a new event chain
    async fn create(&self, chain: &EventChain) -> Result<()>;

    /// Get an event chain by ID
    async fn get(&self, id: EventChainId) -> Result<Option<EventChain>>;

    /// Update an event chain
    async fn update(&self, chain: &EventChain) -> Result<bool>;

    /// List all event chains for a world
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// List active event chains for a world
    async fn list_active(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// List favorite event chains for a world
    async fn list_favorites(&self, world_id: WorldId) -> Result<Vec<EventChain>>;

    /// Get chains containing a specific narrative event
    async fn get_chains_for_event(&self, event_id: NarrativeEventId) -> Result<Vec<EventChain>>;

    /// Add an event to a chain
    async fn add_event_to_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<bool>;

    /// Remove an event from a chain
    async fn remove_event_from_chain(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<bool>;

    /// Mark an event as completed in a chain
    async fn complete_event(
        &self,
        chain_id: EventChainId,
        event_id: NarrativeEventId,
    ) -> Result<bool>;

    /// Toggle favorite status
    async fn toggle_favorite(&self, id: EventChainId) -> Result<bool>;

    /// Set active status
    async fn set_active(&self, id: EventChainId, is_active: bool) -> Result<bool>;

    /// Reset chain progress
    async fn reset(&self, id: EventChainId) -> Result<bool>;

    /// Delete an event chain
    async fn delete(&self, id: EventChainId) -> Result<bool>;

    /// Get chain status summary
    async fn get_status(&self, id: EventChainId) -> Result<Option<ChainStatus>>;

    /// Get all chain statuses for a world
    async fn list_statuses(&self, world_id: WorldId) -> Result<Vec<ChainStatus>>;
}

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
// Region Repository Port
// =============================================================================

/// Repository port for Region operations
#[async_trait]
pub trait RegionRepositoryPort: Send + Sync {
    /// Get a region by ID
    async fn get(&self, id: RegionId) -> Result<Option<Region>>;

    /// List all regions in a location
    async fn list_by_location(&self, location_id: LocationId) -> Result<Vec<Region>>;

    /// List all spawn point regions in a world
    async fn list_spawn_points(&self, world_id: WorldId) -> Result<Vec<Region>>;

    /// Get all NPCs with relationships to a region (for presence determination)
    async fn get_npcs_related_to_region(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<(Character, RegionRelationshipType)>>;

    /// Update a region
    async fn update(&self, region: &Region) -> Result<()>;

    /// Delete a region
    async fn delete(&self, id: RegionId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Region Connections (CONNECTED_TO_REGION edges)
    // -------------------------------------------------------------------------

    /// Create a connection between two regions
    async fn create_connection(&self, connection: &RegionConnection) -> Result<()>;

    /// Get all connections from a region
    async fn get_connections(&self, region_id: RegionId) -> Result<Vec<RegionConnection>>;

    /// Delete a connection between two regions
    async fn delete_connection(&self, from: RegionId, to: RegionId) -> Result<()>;

    /// Unlock a locked connection between two regions
    async fn unlock_connection(&self, from: RegionId, to: RegionId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Region Exits (EXITS_TO_LOCATION edges)
    // -------------------------------------------------------------------------

    /// Create an exit from a region to another location
    async fn create_exit(&self, exit: &RegionExit) -> Result<()>;

    /// Get all exits from a region
    async fn get_exits(&self, region_id: RegionId) -> Result<Vec<RegionExit>>;

    /// Delete an exit from a region to a location
    async fn delete_exit(&self, from_region: RegionId, to_location: LocationId) -> Result<()>;

    // -------------------------------------------------------------------------
    // Region Item Placement (Future - US-REGION-ITEMS)
    // -------------------------------------------------------------------------

    /// Add an item to a region (stub - not yet implemented)
    ///
    /// This will create a `(Region)-[:CONTAINS_ITEM]->(Item)` edge.
    /// Future implementation should enforce region.max_items capacity.
    async fn add_item_to_region(&self, _region_id: RegionId, _item_id: ItemId) -> Result<()> {
        Err(anyhow::anyhow!(
            "Region item placement not yet implemented - see US-REGION-ITEMS"
        ))
    }

    /// Get all items in a region (stub - not yet implemented)
    ///
    /// Returns items linked via `(Region)-[:CONTAINS_ITEM]->(Item)` edge.
    async fn get_region_items(&self, _region_id: RegionId) -> Result<Vec<Item>> {
        Err(anyhow::anyhow!(
            "Region item placement not yet implemented - see US-REGION-ITEMS"
        ))
    }

    /// Remove an item from a region (stub - not yet implemented)
    ///
    /// Deletes the `(Region)-[:CONTAINS_ITEM]->(Item)` edge.
    async fn remove_item_from_region(&self, _region_id: RegionId, _item_id: ItemId) -> Result<()> {
        Err(anyhow::anyhow!(
            "Region item placement not yet implemented - see US-REGION-ITEMS"
        ))
    }
}

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
