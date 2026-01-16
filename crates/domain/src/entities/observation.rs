//! NPC Observation Entity (Phase 23D)
//!
//! Tracks a PC's knowledge of where NPCs are/were located.
//! This creates a "fog of war" where player knowledge differs from reality,
//! enabling mystery/investigation gameplay.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, RegionId};

/// How the PC learned about an NPC's location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObservationType {
    /// PC directly saw the NPC at this location
    Direct,
    /// PC heard about the NPC's location from someone else
    HeardAbout,
    /// PC deduced/investigated the NPC's location
    Deduced,
}

impl ObservationType {
    /// Get a display name for the observation type
    pub fn display_name(&self) -> &'static str {
        match self {
            ObservationType::Direct => "Saw directly",
            ObservationType::HeardAbout => "Heard about",
            ObservationType::Deduced => "Deduced",
        }
    }

    /// Get an icon identifier for the observation type
    pub fn icon(&self) -> &'static str {
        match self {
            ObservationType::Direct => "eye",
            ObservationType::HeardAbout => "ear",
            ObservationType::Deduced => "brain",
        }
    }
}

impl std::fmt::Display for ObservationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for ObservationType {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "direct" => Ok(ObservationType::Direct),
            "heard_about" | "heardabout" => Ok(ObservationType::HeardAbout),
            "deduced" => Ok(ObservationType::Deduced),
            _ => Err(DomainError::parse(format!(
                "Invalid observation type: {}",
                s
            ))),
        }
    }
}

/// A PC's observation of an NPC's location
///
/// Stored as Neo4j edge: `(PlayerCharacter)-[:OBSERVED_NPC {...}]->(Character)`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcObservation {
    /// The PC who made the observation
    pc_id: PlayerCharacterId,
    /// The NPC who was observed
    npc_id: CharacterId,
    /// The location where the NPC was observed
    location_id: LocationId,
    /// The region within the location (more specific)
    region_id: RegionId,
    /// When this observation was made (in game time)
    game_time: DateTime<Utc>,
    /// How the PC learned this information
    observation_type: ObservationType,
    /// Whether the NPC's identity is revealed to the player
    is_revealed_to_player: bool,
    /// Optional notes (e.g., "The bartender mentioned seeing them at the docks")
    notes: Option<String>,
    /// When this observation was recorded (real time)
    created_at: DateTime<Utc>,
}

impl NpcObservation {
    /// Create a new direct observation (PC saw the NPC)
    pub fn direct(
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        location_id: LocationId,
        region_id: RegionId,
        game_time: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Self {
        Self::direct_with_reveal(pc_id, npc_id, location_id, region_id, game_time, true, now)
    }

    /// Create a new direct observation but keep identity unrevealed.
    pub fn direct_unrevealed(
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        location_id: LocationId,
        region_id: RegionId,
        game_time: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Self {
        Self::direct_with_reveal(pc_id, npc_id, location_id, region_id, game_time, false, now)
    }

    fn direct_with_reveal(
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        location_id: LocationId,
        region_id: RegionId,
        game_time: DateTime<Utc>,
        is_revealed_to_player: bool,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            pc_id,
            npc_id,
            location_id,
            region_id,
            game_time,
            observation_type: ObservationType::Direct,
            is_revealed_to_player,
            notes: None,
            created_at: now,
        }
    }

    /// Create a new heard-about observation (PC was told about the NPC)
    pub fn heard_about(
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        location_id: LocationId,
        region_id: RegionId,
        game_time: DateTime<Utc>,
        notes: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            pc_id,
            npc_id,
            location_id,
            region_id,
            game_time,
            observation_type: ObservationType::HeardAbout,
            is_revealed_to_player: true,
            notes,
            created_at: now,
        }
    }

    /// Create a new deduced observation (PC figured out the NPC's location)
    pub fn deduced(
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        location_id: LocationId,
        region_id: RegionId,
        game_time: DateTime<Utc>,
        notes: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            pc_id,
            npc_id,
            location_id,
            region_id,
            game_time,
            observation_type: ObservationType::Deduced,
            is_revealed_to_player: true,
            notes,
            created_at: now,
        }
    }

    /// Reconstruct an NpcObservation from stored data (e.g., database)
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        pc_id: PlayerCharacterId,
        npc_id: CharacterId,
        location_id: LocationId,
        region_id: RegionId,
        game_time: DateTime<Utc>,
        observation_type: ObservationType,
        is_revealed_to_player: bool,
        notes: Option<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            pc_id,
            npc_id,
            location_id,
            region_id,
            game_time,
            observation_type,
            is_revealed_to_player,
            notes,
            created_at,
        }
    }

    // Read accessors
    pub fn pc_id(&self) -> PlayerCharacterId {
        self.pc_id
    }

    pub fn npc_id(&self) -> CharacterId {
        self.npc_id
    }

    pub fn location_id(&self) -> LocationId {
        self.location_id
    }

    pub fn region_id(&self) -> RegionId {
        self.region_id
    }

    pub fn game_time(&self) -> DateTime<Utc> {
        self.game_time
    }

    pub fn observation_type(&self) -> ObservationType {
        self.observation_type
    }

    pub fn is_revealed_to_player(&self) -> bool {
        self.is_revealed_to_player
    }

    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    // Builder methods
    /// Add notes to this observation
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    pub fn with_revealed(mut self, revealed: bool) -> Self {
        self.is_revealed_to_player = revealed;
        self
    }
}

/// Summary of an observation for display purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationSummary {
    /// The NPC's ID
    pub npc_id: String,
    /// The NPC's name
    pub npc_name: String,
    /// The NPC's portrait asset (if any)
    pub npc_portrait: Option<String>,
    /// Whether the NPC's identity is revealed to the player
    pub is_revealed_to_player: bool,
    /// The location name where they were observed
    pub location_name: String,
    /// The region name where they were observed
    pub region_name: String,
    /// When (game time)
    pub game_time: DateTime<Utc>,
    /// How the PC knows this
    pub observation_type: ObservationType,
    /// Any notes
    pub notes: Option<String>,
    /// How long ago in game time (for display)
    pub time_ago_description: Option<String>,
}

impl ObservationSummary {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        npc_id: impl Into<String>,
        npc_name: impl Into<String>,
        is_revealed_to_player: bool,
        location_name: impl Into<String>,
        region_name: impl Into<String>,
        game_time: DateTime<Utc>,
        observation_type: ObservationType,
    ) -> Self {
        Self {
            npc_id: npc_id.into(),
            npc_name: npc_name.into(),
            npc_portrait: None,
            is_revealed_to_player,
            location_name: location_name.into(),
            region_name: region_name.into(),
            game_time,
            observation_type,
            notes: None,
            time_ago_description: None,
        }
    }

    // Builder methods
    pub fn with_npc_portrait(mut self, portrait: impl Into<String>) -> Self {
        self.npc_portrait = Some(portrait.into());
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    pub fn with_time_ago_description(mut self, description: impl Into<String>) -> Self {
        self.time_ago_description = Some(description.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_time() -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn test_observation_type_from_str() {
        assert_eq!(
            "direct".parse::<ObservationType>().unwrap(),
            ObservationType::Direct
        );
        assert_eq!(
            "heard_about".parse::<ObservationType>().unwrap(),
            ObservationType::HeardAbout
        );
        assert_eq!(
            "deduced".parse::<ObservationType>().unwrap(),
            ObservationType::Deduced
        );
    }

    #[test]
    fn test_direct_observation() {
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let game_time = fixed_time();
        let now = fixed_time();

        let obs = NpcObservation::direct(pc_id, npc_id, location_id, region_id, game_time, now);

        assert_eq!(obs.observation_type(), ObservationType::Direct);
        assert!(obs.is_revealed_to_player());
        assert!(obs.notes().is_none());
    }

    #[test]
    fn test_direct_unrevealed_observation() {
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let game_time = fixed_time();
        let now = fixed_time();

        let obs = NpcObservation::direct_unrevealed(
            pc_id,
            npc_id,
            location_id,
            region_id,
            game_time,
            now,
        );

        assert_eq!(obs.observation_type(), ObservationType::Direct);
        assert!(!obs.is_revealed_to_player());
    }

    #[test]
    fn test_heard_about_with_notes() {
        let pc_id = PlayerCharacterId::new();
        let npc_id = CharacterId::new();
        let location_id = LocationId::new();
        let region_id = RegionId::new();
        let game_time = fixed_time();
        let now = fixed_time();

        let obs = NpcObservation::heard_about(
            pc_id,
            npc_id,
            location_id,
            region_id,
            game_time,
            Some("The bartender told me".to_string()),
            now,
        );

        assert_eq!(obs.observation_type(), ObservationType::HeardAbout);
        assert!(obs.is_revealed_to_player());
        assert_eq!(obs.notes(), Some("The bartender told me"));
    }
}
