 //! Shared helpers for conversation use cases.
 //!
 //! Provides utilities for validating NPC visibility, PlayerActionData construction,
 //! and shared DTO types to avoid code duplication.

 use chrono::{DateTime, Utc};
 use wrldbldr_domain::{
     CharacterId, ConversationId, LocationId, PlayerCharacterId, SceneId, WorldId,
 };

 use crate::queue_types::PlayerActionData;

 use crate::infrastructure::ports::{
     ClockPort, StagingRepo, WorldRepo,
 };

 // =============================================================================
 // Shared DTO Types
 // =============================================================================

 /// Location context for conversation summaries.
 #[derive(Debug, Clone)]
 pub struct LocationSummary {
     pub location_id: LocationId,
     pub location_name: String,
     pub region_name: String,
 }

 /// Scene context for conversation summaries.
 #[derive(Debug, Clone)]
 pub struct SceneSummary {
     pub scene_id: SceneId,
     pub scene_name: String,
 }

 /// Active conversation summary (use-case DTO, not infrastructure type).
 ///
 /// This is a domain projection of ActiveConversationRecord.
 /// Mapping from repo types to use case DTOs preserves architecture boundaries.
 #[derive(Debug, Clone)]
 pub struct ActiveConversationSummary {
     pub id: ConversationId,
     pub pc_id: PlayerCharacterId,
     pub npc_id: CharacterId,
     pub pc_name: String,
     pub npc_name: String,
     /// Optional topic hint (from recent dialogue or summary)
     pub topic_hint: Option<String>,
     pub started_at: DateTime<Utc>,
     pub last_updated_at: DateTime<Utc>,
     pub is_active: bool,
     pub turn_count: u32,
     pub pending_approval: bool,
     pub location: Option<LocationSummary>,
     pub scene: Option<SceneSummary>,
 }

 impl ActiveConversationSummary {
     /// Map from infrastructure ActiveConversationRecord to use case DTO.
     ///
     /// This conversion keeps use cases decoupled from infrastructure types.
     pub fn from_record(record: crate::infrastructure::ports::ActiveConversationRecord) -> Self {
         Self {
             id: record.id,
             pc_id: record.pc_id,
             npc_id: record.npc_id,
             pc_name: record.pc_name,
             npc_name: record.npc_name,
             topic_hint: record.topic_hint,
             started_at: record.started_at,
             last_updated_at: record.last_updated_at,
             is_active: record.is_active,
             turn_count: record.turn_count,
             pending_approval: record.pending_approval,
             location: record.location.map(|l| LocationSummary {
                 location_id: l.location_id,
                 location_name: l.location_name,
                 region_name: l.region_name,
             }),
             scene: record.scene.map(|s| SceneSummary {
                 scene_id: s.scene_id,
                 scene_name: s.scene_name,
             }),
         }
     }
 }

 /// Type of participant (PC or NPC).
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum ParticipantType {
     Pc,
     Npc,
 }

 /// Detailed participant info for conversation view (use-case DTO).
 #[derive(Debug, Clone)]
 pub struct ParticipantDetail {
     pub character_id: CharacterId,
     pub name: String,
     pub participant_type: ParticipantType,
     pub turn_count: u32,
     pub last_spoke_at: Option<DateTime<Utc>>,
     pub last_spoke: Option<String>,
     /// NPC's want (for NPCs only)
     pub want: Option<String>,
     /// Relationship to other participant (for NPCs only)
     pub relationship: Option<String>,
 }

 impl ParticipantDetail {
     /// Map from infrastructure ConversationParticipantDetail to use case DTO.
     pub fn from_infrastructure(
         detail: crate::infrastructure::ports::ConversationParticipantDetail,
     ) -> Self {
         Self {
             character_id: detail.character_id,
             name: detail.name,
             participant_type: match detail.participant_type {
                 crate::infrastructure::ports::ParticipantType::Pc => ParticipantType::Pc,
                 crate::infrastructure::ports::ParticipantType::Npc => ParticipantType::Npc,
             },
             turn_count: detail.turn_count,
             last_spoke_at: detail.last_spoke_at,
             last_spoke: detail.last_spoke,
             want: detail.want,
             relationship: detail.relationship,
         }
     }
 }

 /// Dialogue turn detail for conversation history (use-case DTO).
 #[derive(Debug, Clone)]
 pub struct DialogueTurnDetail {
     pub speaker_name: String,
     pub text: String,
     pub timestamp: DateTime<Utc>,
     pub is_dm_override: bool,
 }

 impl DialogueTurnDetail {
     /// Map from infrastructure DialogueTurnDetail to use case DTO.
     pub fn from_infrastructure(
         turn: crate::infrastructure::ports::DialogueTurnDetail,
     ) -> Self {
         Self {
             speaker_name: turn.speaker_name,
             text: turn.text,
             timestamp: turn.timestamp,
             is_dm_override: turn.is_dm_override,
         }
     }
 }

 /// Full conversation details for DM view (use-case DTO, not infrastructure type).
 ///
 /// This is a domain projection of infrastructure ConversationDetails.
 /// Mapping from repo types to use case DTOs preserves architecture boundaries.
 #[derive(Debug, Clone)]
 pub struct ConversationDetailResult {
     pub conversation: ActiveConversationSummary,
     pub participants: Vec<ParticipantDetail>,
     pub recent_turns: Vec<DialogueTurnDetail>,
 }

 impl ConversationDetailResult {
     /// Map from infrastructure ConversationDetails to use case DTO.
     pub fn from_infrastructure(
         details: crate::infrastructure::ports::ConversationDetails,
     ) -> Self {
         Self {
             conversation: ActiveConversationSummary::from_record(details.conversation),
             participants: details
                 .participants
                 .into_iter()
                 .map(ParticipantDetail::from_infrastructure)
                 .collect(),
             recent_turns: details
                 .recent_turns
                 .into_iter()
                 .map(DialogueTurnDetail::from_infrastructure)
                 .collect(),
         }
     }
 }

 /// Result of listing active conversations.
 #[derive(Debug, Clone)]
 pub struct ListActiveConversationsResult {
     pub conversations: Vec<ActiveConversationSummary>,
 }

 // =============================================================================
 // Validation Helpers
 // =============================================================================

 /// Validate that an NPC is staged and visible in the PC's region.
 ///
 /// This helper checks:
 /// 1. PC is in a region
 /// 2. World exists and current game time is available
 /// 3. NPC is staged in the region with TTL check
 /// 4. NPC is present and not hidden from players
 ///
 /// # Arguments
 /// * `staging` - Staging repository for checking active staging
 /// * `world` - World repository for getting current game time
 /// * `pc_region_id` - The region the PC is currently in
 /// * `npc_id` - The NPC to validate
 /// * `world_id` - The world context for TTL check
 ///
 /// # Returns
 /// * `Ok(())` - NPC is staged and visible
 /// * `Err(ConversationError::PlayerNotInRegion)` - PC has no region
 /// * `Err(ConversationError::WorldNotFound(_))` - World doesn't exist
 /// * `Err(ConversationError::NpcNotInRegion)` - NPC not staged or not visible
 ///
 /// # Errors
 /// Returns conversation error variants from start::ConversationError.
 pub async fn validate_npc_staging_visibility(
     staging: &dyn StagingRepo,
     world: &dyn WorldRepo,
     pc_region_id: Option<wrldbldr_domain::RegionId>,
     npc_id: CharacterId,
     world_id: WorldId,
 ) -> Result<(), super::start::ConversationError> {
     // 1. Verify PC has a region
     let region_id = pc_region_id.ok_or(super::start::ConversationError::PlayerNotInRegion)?;

     // 2. Get current game time for staging TTL check
     let world_data = world
         .get(world_id)
         .await?
         .ok_or(super::start::ConversationError::WorldNotFound(world_id))?;
     let current_game_time_seconds = world_data.game_time().total_seconds();

     // 3. Get active staging and filter to visible NPCs
     let active_staging = staging
         .get_active_staging(region_id, current_game_time_seconds)
         .await?;
     let staged_npcs = active_staging
         .map(|s| {
             s.npcs()
                 .iter()
                 .filter(|npc| npc.is_present() && !npc.is_hidden_from_players())
                 .cloned()
                 .collect::<Vec<_>>()
         })
         .unwrap_or_default();
     let npc_in_region = staged_npcs
         .iter()
         .any(|staged| staged.character_id == npc_id);

     if !npc_in_region {
         return Err(super::start::ConversationError::NpcNotInRegion);
     }

     Ok(())
 }

 // =============================================================================
 // PlayerActionData Builder
 // =============================================================================

 /// Build PlayerActionData for a conversation action.
 ///
 /// This helper constructs the queue data payload for player dialogue actions.
 /// Centralized here to ensure consistent structure across use cases.
 ///
 /// # Arguments
 /// * `world_id` - The world context
 /// * `player_id` - The player's user ID
 /// * `pc_id` - The player character in the conversation
 /// * `npc_id` - The NPC being spoken to
 /// * `dialogue` - The dialogue text
 /// * `clock` - Clock for timestamp
 /// * `conversation_id` - Optional conversation ID (for continuing conversations)
 ///
 /// # Returns
 /// PlayerActionData ready for queuing
 pub fn build_player_action_data(
     world_id: WorldId,
     player_id: String,
     pc_id: PlayerCharacterId,
     npc_id: CharacterId,
     dialogue: String,
     clock: &dyn ClockPort,
     conversation_id: Option<ConversationId>,
 ) -> PlayerActionData {
     // Note: target is the NPC ID (as string) so it can be parsed in build_prompt
     PlayerActionData {
         world_id,
         player_id,
         pc_id: Some(pc_id),
         action_type: "talk".to_string(),
         target: Some(npc_id.to_string()),
         dialogue: Some(dialogue),
         timestamp: clock.now(),
         conversation_id: conversation_id.map(|id| id.to_uuid()),
     }
 }
