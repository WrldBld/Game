//! World seeder for loading Thornhaven test fixtures.
//!
//! Provides utilities for loading the Thornhaven village test world from JSON fixtures
//! and creating a fully-populated test environment for LLM integration testing.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::test_fixtures::world_seeder;
//!
//! #[test]
//! fn test_with_thornhaven() {
//!     let world = world_seeder::load_thornhaven();
//!     let marta = world.npc_by_name("Marta Hearthwood").unwrap();
//!     // ... test logic
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wrldbldr_domain::{
    ActId, CampbellArchetype, ChallengeId, CharacterId, DispositionLevel, LocationId,
    LocationStateId, MonomythStage, MoodState, NarrativeEventId, RegionId, SceneId, TimeOfDay,
    WantId, WorldId,
};

use super::load_fixture;

// =============================================================================
// TestWorld - Main container for loaded test world
// =============================================================================

/// A fully loaded test world with all Thornhaven fixtures.
///
/// Provides convenient access to all entities by name or ID.
#[derive(Debug, Clone)]
pub struct TestWorld {
    /// The world ID (fixed for Thornhaven)
    pub world_id: WorldId,

    // Raw data from JSON files
    pub locations: Vec<LocationData>,
    pub regions: Vec<RegionData>,
    pub location_connections: Vec<LocationConnectionData>,
    pub region_connections: Vec<RegionConnectionData>,
    pub npcs: Vec<NpcData>,
    pub home_locations: Vec<HomeLocationData>,
    pub works_at: Vec<WorksAtData>,
    pub frequents: Vec<FrequentsData>,
    pub avoids: Vec<AvoidsData>,
    pub relationships: Vec<RelationshipData>,
    pub wants: Vec<WantData>,
    pub actantial_views: Vec<ActantialViewData>,
    pub acts: Vec<ActData>,
    pub scenes: Vec<SceneData>,
    pub featured_characters: Vec<FeaturedCharacterData>,
    pub challenges: Vec<ChallengeData>,
    pub narrative_events: Vec<NarrativeEventData>,
    pub location_states: Vec<LocationStateData>,

    // Quick lookup maps
    location_by_name: HashMap<String, LocationId>,
    region_by_name: HashMap<String, RegionId>,
    npc_by_name: HashMap<String, CharacterId>,
    scene_by_name: HashMap<String, SceneId>,
    challenge_by_name: HashMap<String, ChallengeId>,
    event_by_name: HashMap<String, NarrativeEventId>,
}

impl TestWorld {
    /// Get a location ID by name.
    pub fn location(&self, name: &str) -> Option<LocationId> {
        self.location_by_name.get(name).copied()
    }

    /// Get a region ID by name.
    pub fn region(&self, name: &str) -> Option<RegionId> {
        self.region_by_name.get(name).copied()
    }

    /// Get an NPC's character ID by name.
    pub fn npc(&self, name: &str) -> Option<CharacterId> {
        self.npc_by_name.get(name).copied()
    }

    /// Get NPC data by name.
    pub fn npc_data(&self, name: &str) -> Option<&NpcData> {
        self.npcs.iter().find(|n| n.name == name)
    }

    /// Get a scene ID by name.
    pub fn scene(&self, name: &str) -> Option<SceneId> {
        self.scene_by_name.get(name).copied()
    }

    /// Get a challenge ID by name.
    pub fn challenge(&self, name: &str) -> Option<ChallengeId> {
        self.challenge_by_name.get(name).copied()
    }

    /// Get a narrative event ID by name.
    pub fn event(&self, name: &str) -> Option<NarrativeEventId> {
        self.event_by_name.get(name).copied()
    }

    /// Get location data by ID.
    pub fn location_data(&self, id: LocationId) -> Option<&LocationData> {
        self.locations.iter().find(|l| l.id == id)
    }

    /// Get scene data by ID.
    pub fn scene_data(&self, id: SceneId) -> Option<&SceneData> {
        self.scenes.iter().find(|s| s.id == id)
    }

    /// Get all NPCs at a specific location (via works_at).
    pub fn npcs_working_at(&self, location_id: LocationId) -> Vec<CharacterId> {
        self.works_at
            .iter()
            .filter(|w| w.location_id == location_id)
            .map(|w| w.character_id)
            .collect()
    }

    /// Get all NPCs that frequent a location.
    pub fn npcs_frequenting(&self, location_id: LocationId) -> Vec<CharacterId> {
        self.frequents
            .iter()
            .filter(|f| f.location_id == location_id)
            .map(|f| f.character_id)
            .collect()
    }

    /// Get all NPCs that avoid a location.
    pub fn npcs_avoiding(&self, location_id: LocationId) -> Vec<CharacterId> {
        self.avoids
            .iter()
            .filter(|a| a.location_id == location_id)
            .map(|a| a.character_id)
            .collect()
    }

    /// Get relationships from a specific character.
    pub fn relationships_from(&self, character_id: CharacterId) -> Vec<&RelationshipData> {
        self.relationships
            .iter()
            .filter(|r| r.from_character_id == character_id)
            .collect()
    }

    /// Get wants for a specific character.
    pub fn wants_for(&self, character_id: CharacterId) -> Vec<&WantData> {
        self.wants
            .iter()
            .filter(|w| w.character_id == character_id)
            .collect()
    }

    /// Get featured characters for a scene.
    pub fn featured_in_scene(&self, scene_id: SceneId) -> Vec<&FeaturedCharacterData> {
        self.featured_characters
            .iter()
            .filter(|f| f.scene_id == scene_id)
            .collect()
    }

    /// Get location states for a location.
    pub fn states_for_location(&self, location_id: LocationId) -> Vec<&LocationStateData> {
        self.location_states
            .iter()
            .filter(|s| s.location_id == location_id)
            .collect()
    }
}

// =============================================================================
// JSON Data Structures - Locations
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationsFile {
    #[serde(rename = "_comment")]
    pub comment: Option<String>,
    pub locations: Vec<LocationData>,
    pub connections: Vec<LocationConnectionData>,
    pub region_connections: Vec<RegionConnectionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationData {
    pub id: LocationId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    pub location_type: String,
    pub backdrop_asset: Option<String>,
    pub map_asset: Option<String>,
    pub parent_map_bounds: Option<serde_json::Value>,
    pub default_region_id: RegionId,
    pub atmosphere: String,
    pub presence_cache_ttl_hours: u32,
    pub use_llm_presence: bool,
    pub regions: Vec<RegionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionData {
    pub id: RegionId,
    pub location_id: LocationId,
    pub name: String,
    pub description: String,
    pub backdrop_asset: Option<String>,
    pub atmosphere: String,
    pub map_bounds: Option<serde_json::Value>,
    pub is_spawn_point: bool,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationConnectionData {
    pub from_location_id: LocationId,
    pub to_location_id: LocationId,
    pub connection_type: String,
    pub description: String,
    pub bidirectional: bool,
    pub travel_time: u32,
    pub is_locked: bool,
    pub lock_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionConnectionData {
    pub from_region_id: RegionId,
    pub to_region_id: RegionId,
    pub description: String,
    pub bidirectional: bool,
    pub is_locked: bool,
    pub lock_description: Option<String>,
}

// =============================================================================
// JSON Data Structures - NPCs
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcsFile {
    #[serde(rename = "_comment")]
    pub comment: Option<String>,
    pub npcs: Vec<NpcData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcData {
    pub id: CharacterId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub base_archetype: CampbellArchetype,
    pub current_archetype: CampbellArchetype,
    pub archetype_history: Vec<serde_json::Value>,
    pub stats: NpcStatsData,
    pub is_alive: bool,
    pub is_active: bool,
    pub default_disposition: DispositionLevel,
    pub default_mood: MoodState,
    pub expression_config: ExpressionConfigData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcStatsData {
    pub stats: HashMap<String, i32>,
    pub modifiers: HashMap<String, serde_json::Value>,
    pub current_hp: i32,
    pub max_hp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpressionConfigData {
    pub expressions: Vec<String>,
    pub actions: Vec<String>,
    pub default_expression: String,
}

// =============================================================================
// JSON Data Structures - NPC Schedules
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcSchedulesFile {
    #[serde(rename = "_comment")]
    pub comment: Option<String>,
    pub home_locations: Vec<HomeLocationData>,
    pub home_regions: Vec<HomeRegionData>,
    pub works_at: Vec<WorksAtData>,
    pub works_at_region: Vec<WorksAtRegionData>,
    pub frequents: Vec<FrequentsData>,
    pub frequents_region: Vec<FrequentsRegionData>,
    pub avoids: Vec<AvoidsData>,
    pub avoids_region: Vec<AvoidsRegionData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeLocationData {
    pub character_id: CharacterId,
    pub location_id: LocationId,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeRegionData {
    pub character_id: CharacterId,
    pub region_id: RegionId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorksAtData {
    pub character_id: CharacterId,
    pub location_id: LocationId,
    pub role: String,
    pub schedule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorksAtRegionData {
    pub character_id: CharacterId,
    pub region_id: RegionId,
    pub shift: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrequentsData {
    pub character_id: CharacterId,
    pub location_id: LocationId,
    pub frequency: String,
    pub time_of_day: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrequentsRegionData {
    pub character_id: CharacterId,
    pub region_id: RegionId,
    pub frequency: String,
    pub time_of_day: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvoidsData {
    pub character_id: CharacterId,
    pub location_id: LocationId,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvoidsRegionData {
    pub character_id: CharacterId,
    pub region_id: RegionId,
    pub reason: String,
}

// =============================================================================
// JSON Data Structures - Relationships
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipsFile {
    #[serde(rename = "_comment")]
    pub comment: Option<String>,
    pub relationships: Vec<RelationshipData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipData {
    pub id: String,
    pub from_character_id: CharacterId,
    pub to_character_id: CharacterId,
    pub relationship_type: String,
    pub sentiment: f32,
    pub known_to_player: bool,
    pub history: Vec<RelationshipHistoryData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipHistoryData {
    pub event: String,
    pub sentiment_change: f32,
}

// =============================================================================
// JSON Data Structures - Wants
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WantsFile {
    #[serde(rename = "_comment")]
    pub comment: Option<String>,
    pub wants: Vec<WantData>,
    pub actantial_views: Vec<ActantialViewData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WantData {
    pub id: WantId,
    pub character_id: CharacterId,
    pub description: String,
    pub intensity: f32,
    pub visibility: String,
    pub priority: u32,
    pub deflection_behavior: Option<String>,
    pub tells: Vec<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActantialViewData {
    pub subject_id: CharacterId,
    pub target_id: CharacterId,
    pub role: String,
    pub want_id: WantId,
    pub reason: String,
}

// =============================================================================
// JSON Data Structures - Scenes
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenesFile {
    #[serde(rename = "_comment")]
    pub comment: Option<String>,
    pub acts: Vec<ActData>,
    pub scenes: Vec<SceneData>,
    pub featured_character_edges: Vec<FeaturedCharacterData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActData {
    pub id: ActId,
    pub world_id: WorldId,
    pub name: String,
    pub stage: MonomythStage,
    pub description: String,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneData {
    pub id: SceneId,
    pub act_id: ActId,
    pub name: String,
    pub location_id: LocationId,
    pub time_context: serde_json::Value,
    pub backdrop_override: Option<String>,
    pub entry_conditions: Vec<serde_json::Value>,
    pub featured_characters: Vec<CharacterId>,
    pub directorial_notes: String,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeaturedCharacterData {
    pub scene_id: SceneId,
    pub character_id: CharacterId,
    pub role: String,
    pub entrance_cue: Option<String>,
}

// =============================================================================
// JSON Data Structures - Challenges
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengesFile {
    #[serde(rename = "_comment")]
    pub comment: Option<String>,
    pub challenges: Vec<ChallengeData>,
    pub challenge_location_edges: Vec<ChallengeLocationEdgeData>,
    pub challenge_region_edges: Vec<ChallengeRegionEdgeData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeData {
    pub id: ChallengeId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    pub challenge_type: String,
    pub difficulty: serde_json::Value,
    pub check_stat: Option<String>,
    pub outcomes: ChallengeOutcomesData,
    pub trigger_conditions: Vec<serde_json::Value>,
    pub active: bool,
    pub order: u32,
    pub is_favorite: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeOutcomesData {
    pub success: OutcomeData,
    pub failure: OutcomeData,
    pub partial: Option<OutcomeData>,
    pub critical_success: Option<OutcomeData>,
    pub critical_failure: Option<OutcomeData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeData {
    pub description: String,
    pub triggers: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeLocationEdgeData {
    pub challenge_id: ChallengeId,
    pub location_id: LocationId,
    pub always_available: bool,
    pub time_restriction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeRegionEdgeData {
    pub challenge_id: ChallengeId,
    pub region_id: RegionId,
    pub always_available: bool,
    pub time_restriction: Option<String>,
}

// =============================================================================
// JSON Data Structures - Narrative Events
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeEventsFile {
    #[serde(rename = "_comment")]
    pub comment: Option<String>,
    pub narrative_events: Vec<NarrativeEventData>,
    pub event_location_edges: Vec<EventLocationEdgeData>,
    pub featured_npc_edges: Vec<FeaturedNpcEdgeData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeEventData {
    pub id: NarrativeEventId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub trigger_conditions: Vec<serde_json::Value>,
    pub trigger_logic: String,
    pub scene_direction: String,
    pub suggested_opening: Option<String>,
    pub outcomes: Vec<serde_json::Value>,
    pub default_outcome: Option<String>,
    pub is_active: bool,
    pub is_triggered: bool,
    pub triggered_at: Option<String>,
    pub selected_outcome: Option<String>,
    pub is_repeatable: bool,
    pub trigger_count: u32,
    pub delay_turns: u32,
    pub expires_after_turns: Option<u32>,
    pub priority: i32,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventLocationEdgeData {
    pub event_id: NarrativeEventId,
    pub location_id: LocationId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeaturedNpcEdgeData {
    pub event_id: NarrativeEventId,
    pub character_id: CharacterId,
    pub role: String,
}

// =============================================================================
// JSON Data Structures - Location States
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationStatesFile {
    #[serde(rename = "_comment")]
    pub comment: Option<String>,
    pub location_states: Vec<LocationStateData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationStateData {
    pub id: LocationStateId,
    pub location_id: LocationId,
    pub world_id: WorldId,
    pub name: String,
    pub description: String,
    pub backdrop_override: Option<String>,
    pub atmosphere_override: Option<String>,
    pub ambient_sound: Option<String>,
    pub map_overlay: Option<String>,
    pub activation_rules: Vec<serde_json::Value>,
    pub activation_logic: String,
    pub priority: i32,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Loading Functions
// =============================================================================

/// Load the complete Thornhaven test world from JSON fixtures.
///
/// This loads all JSON files and builds lookup maps for convenient access.
pub fn load_thornhaven() -> TestWorld {
    // Load all JSON files
    let locations_file: LocationsFile = load_fixture("dnd5e/thornhaven/locations.json");
    let npcs_file: NpcsFile = load_fixture("dnd5e/thornhaven/npcs.json");
    let schedules_file: NpcSchedulesFile = load_fixture("dnd5e/thornhaven/npc_schedules.json");
    let relationships_file: RelationshipsFile = load_fixture("dnd5e/thornhaven/relationships.json");
    let wants_file: WantsFile = load_fixture("dnd5e/thornhaven/wants.json");
    let scenes_file: ScenesFile = load_fixture("dnd5e/thornhaven/scenes.json");
    let challenges_file: ChallengesFile = load_fixture("dnd5e/thornhaven/challenges.json");
    let events_file: NarrativeEventsFile = load_fixture("dnd5e/thornhaven/narrative_events.json");
    let states_file: LocationStatesFile = load_fixture("dnd5e/thornhaven/visual_states.json");

    // Extract regions from locations
    let mut all_regions = Vec::new();
    for location in &locations_file.locations {
        all_regions.extend(location.regions.clone());
    }

    // Build lookup maps
    let mut location_by_name = HashMap::new();
    for location in &locations_file.locations {
        location_by_name.insert(location.name.clone(), location.id);
    }

    let mut region_by_name = HashMap::new();
    for region in &all_regions {
        region_by_name.insert(region.name.clone(), region.id);
    }

    let mut npc_by_name = HashMap::new();
    for npc in &npcs_file.npcs {
        npc_by_name.insert(npc.name.clone(), npc.id);
    }

    let mut scene_by_name = HashMap::new();
    for scene in &scenes_file.scenes {
        scene_by_name.insert(scene.name.clone(), scene.id);
    }

    let mut challenge_by_name = HashMap::new();
    for challenge in &challenges_file.challenges {
        challenge_by_name.insert(challenge.name.clone(), challenge.id);
    }

    let mut event_by_name = HashMap::new();
    for event in &events_file.narrative_events {
        event_by_name.insert(event.name.clone(), event.id);
    }

    // Fixed world ID for Thornhaven
    let world_id =
        WorldId::from(uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap());

    TestWorld {
        world_id,
        locations: locations_file.locations,
        regions: all_regions,
        location_connections: locations_file.connections,
        region_connections: locations_file.region_connections,
        npcs: npcs_file.npcs,
        home_locations: schedules_file.home_locations,
        works_at: schedules_file.works_at,
        frequents: schedules_file.frequents,
        avoids: schedules_file.avoids,
        relationships: relationships_file.relationships,
        wants: wants_file.wants,
        actantial_views: wants_file.actantial_views,
        acts: scenes_file.acts,
        scenes: scenes_file.scenes,
        featured_characters: scenes_file.featured_character_edges,
        challenges: challenges_file.challenges,
        narrative_events: events_file.narrative_events,
        location_states: states_file.location_states,
        location_by_name,
        region_by_name,
        npc_by_name,
        scene_by_name,
        challenge_by_name,
        event_by_name,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_thornhaven() {
        let world = load_thornhaven();

        // Verify locations loaded
        assert_eq!(world.locations.len(), 6);
        assert!(world.location("The Drowsy Dragon Inn").is_some());
        assert!(world.location("The Old Mill").is_some());

        // Verify NPCs loaded
        assert_eq!(world.npcs.len(), 8);
        assert!(world.npc("Marta Hearthwood").is_some());
        assert!(world.npc("Grom Ironhand").is_some());
        assert!(world.npc("Old Tom").is_some());

        // Verify scenes loaded
        assert!(world.scenes.len() >= 8);
        assert!(world.scene("Welcome to the Drowsy Dragon").is_some());

        // Verify challenges loaded
        assert!(world.challenges.len() >= 3);
        assert!(world.challenge("Convince Grom to Share His Past").is_some());

        // Verify narrative events loaded
        assert!(world.narrative_events.len() >= 3);
        assert!(world.event("The Stranger's Warning").is_some());
    }

    #[test]
    fn test_npc_lookup() {
        let world = load_thornhaven();

        let marta = world.npc_data("Marta Hearthwood").unwrap();
        assert_eq!(marta.base_archetype, CampbellArchetype::Mentor);
        assert_eq!(marta.default_disposition, DispositionLevel::Friendly);

        let grom = world.npc_data("Grom Ironhand").unwrap();
        assert_eq!(grom.base_archetype, CampbellArchetype::ThresholdGuardian);
    }

    #[test]
    fn test_relationships() {
        let world = load_thornhaven();

        let marta_id = world.npc("Marta Hearthwood").unwrap();
        let marta_relationships = world.relationships_from(marta_id);

        // Marta has relationships with Grom, Tom, and Pip
        assert!(marta_relationships.len() >= 3);
    }

    #[test]
    fn test_wants() {
        let world = load_thornhaven();

        let marta_id = world.npc("Marta Hearthwood").unwrap();
        let marta_wants = world.wants_for(marta_id);

        // Marta has at least 2 wants (protect village, find out about mill)
        assert!(marta_wants.len() >= 2);

        // Check one is known and one is hidden
        let visibilities: Vec<&str> = marta_wants.iter().map(|w| w.visibility.as_str()).collect();
        assert!(visibilities.contains(&"known"));
        assert!(visibilities.contains(&"hidden"));
    }

    #[test]
    fn test_location_states() {
        let world = load_thornhaven();

        let inn_id = world.location("The Drowsy Dragon Inn").unwrap();
        let inn_states = world.states_for_location(inn_id);

        // Inn has morning, evening, and default states
        assert!(inn_states.len() >= 3);
    }

    #[test]
    fn test_featured_characters() {
        let world = load_thornhaven();

        let scene_id = world.scene("Welcome to the Drowsy Dragon").unwrap();
        let featured = world.featured_in_scene(scene_id);

        // Scene features Marta
        assert!(!featured.is_empty());
        let marta_id = world.npc("Marta Hearthwood").unwrap();
        assert!(featured.iter().any(|f| f.character_id == marta_id));
    }
}
