//! Connection Use Case
//!
//! Handles connection lifecycle: join/leave world, spectator management.
//!
//! # Responsibilities
//!
//! - Join a world with a specific role (DM, Player, Spectator)
//! - Leave a world
//! - Set spectate target for spectators
//! - Broadcast connection events
//!
//! # Architecture Note
//!
//! Connection operations affect the world state and notify other participants.
//! The use case coordinates between connection management and player data.

use std::sync::Arc;
use tracing::{debug, info, warn};

use async_trait::async_trait;
use uuid::Uuid;
use wrldbldr_domain::WorldId;
use wrldbldr_engine_ports::inbound::ConnectionUseCasePort;
use wrldbldr_engine_ports::outbound::{BroadcastPort, GameEvent};

use super::errors::ConnectionError;

// Import services
use crate::application::services::{JoinValidation, WorldSessionPolicy};

// Import internal service ports
use crate::application::services::internal::{PlayerCharacterServicePort, WorldServicePort};

// Import repository port for directorial context
use wrldbldr_engine_ports::outbound::DirectorialContextRepositoryPort;

// Import domain types for conversions
use wrldbldr_domain::value_objects::{DirectorialNotes, PacingGuidance};

pub use wrldbldr_engine_ports::outbound::ConnectionManagerPort;

// Import types from engine-ports
pub use wrldbldr_engine_ports::outbound::{
    ConnectedUser, ConnectionInfo, DirectorialContextData, JoinWorldInput, JoinWorldResult,
    LeaveWorldResult, NpcMotivation, PcData, SetSpectateTargetInput, SpectateTargetResult,
    UserJoinedEvent, WorldRole,
};

pub use wrldbldr_engine_ports::outbound::WorldStateUpdatePort as WorldStatePort;

// =============================================================================
// Connection Use Case
// =============================================================================

/// Use case for connection operations
pub struct ConnectionUseCase {
    connection_manager: Arc<dyn ConnectionManagerPort>,
    world_service: Arc<dyn WorldServicePort>,
    pc_service: Arc<dyn PlayerCharacterServicePort>,
    directorial_repo: Arc<dyn DirectorialContextRepositoryPort>,
    world_state: Arc<dyn WorldStatePort>,
    broadcast: Arc<dyn BroadcastPort>,
    session_policy: WorldSessionPolicy,
}

impl ConnectionUseCase {
    /// Create a new ConnectionUseCase with all dependencies
    pub fn new(
        connection_manager: Arc<dyn ConnectionManagerPort>,
        world_service: Arc<dyn WorldServicePort>,
        pc_service: Arc<dyn PlayerCharacterServicePort>,
        directorial_repo: Arc<dyn DirectorialContextRepositoryPort>,
        world_state: Arc<dyn WorldStatePort>,
        broadcast: Arc<dyn BroadcastPort>,
    ) -> Self {
        Self {
            connection_manager,
            world_service,
            pc_service,
            directorial_repo,
            world_state,
            broadcast,
            session_policy: WorldSessionPolicy::new(),
        }
    }

    /// Convert PlayerCharacter to PcData DTO
    fn player_character_to_pc_data(pc: wrldbldr_domain::entities::PlayerCharacter) -> PcData {
        PcData {
            id: pc.id.to_string(),
            name: pc.name,
            user_id: pc.user_id,
            world_id: pc.world_id.to_string(),
            current_location_id: pc.current_location_id.to_string(),
            current_region_id: pc.current_region_id.map(|id| id.to_string()),
            description: pc.description,
            sprite_asset: pc.sprite_asset,
            portrait_asset: pc.portrait_asset,
        }
    }

    /// Convert DirectorialNotes to DirectorialContextData DTO
    fn directorial_notes_to_context_data(notes: DirectorialNotes) -> DirectorialContextData {
        let pacing_str = match notes.pacing {
            PacingGuidance::Natural => None,
            PacingGuidance::Fast => Some("fast".to_string()),
            PacingGuidance::Slow => Some("slow".to_string()),
            PacingGuidance::Building => Some("building".to_string()),
            PacingGuidance::Urgent => Some("urgent".to_string()),
        };
        let tone_str = notes.tone.description().to_string();
        DirectorialContextData {
            npc_motivations: notes
                .npc_motivations
                .into_iter()
                .map(|(char_id, m)| NpcMotivation {
                    character_id: char_id,
                    motivation: m.immediate_goal,
                    emotional_state: if m.current_mood.is_empty() {
                        None
                    } else {
                        Some(m.current_mood)
                    },
                })
                .collect(),
            scene_mood: if tone_str.is_empty() || tone_str == "Neutral - balanced, conversational" {
                None
            } else {
                Some(tone_str)
            },
            pacing: pacing_str,
            dm_notes: if notes.general_notes.is_empty() {
                None
            } else {
                Some(notes.general_notes)
            },
        }
    }

    /// Join a world
    pub async fn join_world(
        &self,
        connection_id: uuid::Uuid,
        user_id: String,
        input: JoinWorldInput,
    ) -> Result<JoinWorldResult, ConnectionError> {
        info!(
            world_id = %input.world_id,
            role = ?input.role,
            pc_id = ?input.pc_id,
            "User joining world"
        );

        // Validate join request using policy (business rules)
        let current_dm_user_id = self
            .connection_manager
            .get_dm_user_id(*input.world_id.as_uuid())
            .await;

        let validation = self.session_policy.validate_join(
            input.role,
            &user_id,
            input.pc_id.map(|id| *id.as_uuid()),
            input.spectate_pc_id.map(|id| *id.as_uuid()),
            current_dm_user_id.as_deref(),
        );

        if let JoinValidation::Denied(err) = validation {
            warn!(
                world_id = %input.world_id,
                role = ?input.role,
                error = ?err,
                "Join request denied by policy"
            );
            return Err(ConnectionError::from(err));
        }

        // Register connection
        self.connection_manager
            .register_connection(connection_id, connection_id.to_string(), user_id.clone())
            .await;

        // Join the world (validation already done, this is just state management)
        let connected_users = self
            .connection_manager
            .join_world(
                connection_id,
                *input.world_id.as_uuid(),
                input.role,
                input.pc_id.map(|id| *id.as_uuid()),
                input.spectate_pc_id.map(|id| *id.as_uuid()),
            )
            .await
            .map_err(ConnectionError::ConnectionFailed)?;

        // Get world snapshot (call service, then serialize to JSON)
        let snapshot = match self.world_service.export_world_snapshot(input.world_id).await {
            Ok(world_snapshot) => {
                serde_json::to_value(&world_snapshot).unwrap_or_else(|e| {
                    warn!(error = %e, "Failed to serialize world snapshot");
                    serde_json::json!({})
                })
            }
            Err(e) => {
                warn!(error = %e, "Failed to get world snapshot");
                serde_json::json!({})
            }
        };

        // Load persisted directorial context (convert domain to DTO for state)
        if let Ok(Some(notes)) = self.directorial_repo.get(&input.world_id).await {
            let context_data = Self::directorial_notes_to_context_data(notes);
            self.world_state
                .set_directorial_context(&input.world_id, context_data);
            debug!(world_id = %input.world_id, "Loaded persisted directorial context");
        }

        // Get PC data if Player role (convert domain entity to DTO)
        let pc_data = if input.role == WorldRole::Player {
            if let Some(pc_id) = input.pc_id {
                match self.pc_service.get_pc(pc_id).await {
                    Ok(Some(pc)) => Some(Self::player_character_to_pc_data(pc)),
                    Ok(None) => None,
                    Err(_) => None,
                }
            } else {
                debug!("Player joined without PC ID");
                None
            }
        } else {
            None
        };

        // Broadcast UserJoined to other connections
        let user_joined = UserJoinedEvent {
            user_id: user_id.clone(),
            role: input.role,
            pc: pc_data.clone(),
        };

        let world_connections = self
            .connection_manager
            .get_world_connections(*input.world_id.as_uuid())
            .await;

        for other_conn_id in world_connections {
            if other_conn_id != connection_id {
                self.connection_manager
                    .send_to_connection(other_conn_id, user_joined.clone())
                    .await;
            }
        }

        info!(
            world_id = %input.world_id,
            connected_users = connected_users.len(),
            "User joined world successfully"
        );

        Ok(JoinWorldResult {
            world_id: input.world_id,
            snapshot,
            connected_users,
            your_role: input.role,
            your_pc: pc_data,
        })
    }

    /// Leave a world
    pub async fn leave_world(
        &self,
        connection_id: uuid::Uuid,
    ) -> Result<LeaveWorldResult, ConnectionError> {
        info!(connection_id = %connection_id, "User leaving world");

        // Get connection info before leaving
        let conn_info = self.connection_manager.get_connection(connection_id).await;

        // Leave the world
        if let Some((world_id, _role)) = self.connection_manager.leave_world(connection_id).await {
            // Broadcast UserLeft to remaining users
            if let Some(info) = conn_info {
                self.broadcast
                    .broadcast(
                        WorldId::from_uuid(world_id),
                        GameEvent::PlayerLeft {
                            user_id: info.user_id,
                        },
                    )
                    .await;
            }
            info!(world_id = %world_id, "User left world");
        }

        Ok(LeaveWorldResult { left: true })
    }

    /// Set spectate target
    pub async fn set_spectate_target(
        &self,
        connection_id: uuid::Uuid,
        input: SetSpectateTargetInput,
    ) -> Result<SpectateTargetResult, ConnectionError> {
        info!(
            connection_id = %connection_id,
            pc_id = %input.pc_id,
            "Setting spectate target"
        );

        // Verify connection is a spectator
        let conn_info = self
            .connection_manager
            .get_connection(connection_id)
            .await
            .ok_or(ConnectionError::NotConnected)?;

        if !conn_info.is_spectator() {
            return Err(ConnectionError::InvalidSpectateTarget(
                "Only spectators can change spectate target".to_string(),
            ));
        }

        // Get PC and extract name
        let pc = self
            .pc_service
            .get_pc(input.pc_id)
            .await
            .map_err(|e| ConnectionError::Database(e.to_string()))?
            .ok_or(ConnectionError::PcNotFound(input.pc_id))?;

        // Set spectate target
        self.connection_manager
            .set_spectate_target(connection_id, Some(*input.pc_id.as_uuid()))
            .await;

        info!(
            pc_id = %input.pc_id,
            pc_name = %pc.name,
            "Spectate target changed"
        );

        Ok(SpectateTargetResult {
            pc_id: input.pc_id,
            pc_name: pc.name,
        })
    }
}

// =============================================================================
// ConnectionUseCasePort Implementation
// =============================================================================

#[async_trait]
impl ConnectionUseCasePort for ConnectionUseCase {
    async fn join_world(
        &self,
        connection_id: Uuid,
        user_id: String,
        input: JoinWorldInput,
    ) -> Result<JoinWorldResult, ConnectionError> {
        self.join_world(connection_id, user_id, input).await
    }

    async fn leave_world(&self, connection_id: Uuid) -> Result<LeaveWorldResult, ConnectionError> {
        self.leave_world(connection_id).await
    }

    async fn set_spectate_target(
        &self,
        connection_id: Uuid,
        input: SetSpectateTargetInput,
    ) -> Result<SpectateTargetResult, ConnectionError> {
        self.set_spectate_target(connection_id, input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_role_variants() {
        assert_eq!(WorldRole::DM, WorldRole::DM);
        assert_eq!(WorldRole::Player, WorldRole::Player);
        assert_eq!(WorldRole::Spectator, WorldRole::Spectator);
        assert_ne!(WorldRole::DM, WorldRole::Player);
    }

    #[test]
    fn test_connection_info_is_spectator() {
        let spectator = ConnectionInfo {
            connection_id: uuid::Uuid::new_v4(),
            client_id: "test".to_string(),
            user_id: "user".to_string(),
            world_id: None,
            role: Some(WorldRole::Spectator),
            pc_id: None,
            spectate_pc_id: None,
        };

        let player = ConnectionInfo {
            connection_id: uuid::Uuid::new_v4(),
            client_id: "test".to_string(),
            user_id: "user".to_string(),
            world_id: None,
            role: Some(WorldRole::Player),
            pc_id: None,
            spectate_pc_id: None,
        };

        assert!(spectator.is_spectator());
        assert!(!player.is_spectator());
    }
}
