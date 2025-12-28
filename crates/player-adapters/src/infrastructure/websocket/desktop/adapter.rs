//! Desktop GameConnectionPort adapter

use std::future::Future;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};

use anyhow::Result;

use wrldbldr_player_ports::outbound::{ConnectionState as PortConnectionState, GameConnectionPort};
use wrldbldr_protocol::{
    AdHocOutcomes, ApprovalDecision, ApprovedNpcInfo, ChallengeOutcomeDecisionData, ClientMessage,
    DiceInputType, DirectorialContext, ParticipantRole, RequestError, RequestPayload, ResponseResult,
};

use super::client::EngineClient;
use crate::infrastructure::websocket::protocol::{map_state, state_to_u8, u8_to_state};

/// Desktop game connection adapter
#[derive(Clone)]
pub struct DesktopGameConnection {
    client: EngineClient,
    state: Arc<AtomicU8>,
}

impl DesktopGameConnection {
    pub fn new(client: EngineClient) -> Self {
        Self {
            client,
            state: Arc::new(AtomicU8::new(state_to_u8(PortConnectionState::Disconnected))),
        }
    }
}

impl GameConnectionPort for DesktopGameConnection {
    fn state(&self) -> PortConnectionState {
        u8_to_state(self.state.load(Ordering::SeqCst))
    }

    fn url(&self) -> &str {
        self.client.url()
    }

    fn connect(&self) -> Result<()> {
        let client = self.client.clone();
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            if let Err(e) = client.connect().await {
                tracing::error!("Failed to connect to Engine: {}", e);
                state.store(state_to_u8(PortConnectionState::Failed), Ordering::SeqCst);
            }
        });
        Ok(())
    }

    fn disconnect(&self) {
        let client = self.client.clone();
        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            client.disconnect().await;
            state.store(state_to_u8(PortConnectionState::Disconnected), Ordering::SeqCst);
        });
    }

    fn join_world(
        &self,
        world_id: &str,
        user_id: &str,
        role: ParticipantRole,
    ) -> Result<()> {
        let client = self.client.clone();
        let world_id = world_id.to_string();
        let user_id = user_id.to_string();
        tokio::spawn(async move {
            if let Err(e) = client.join_world(&world_id, &user_id, role).await {
                tracing::error!("Failed to join world: {}", e);
            }
        });
        Ok(())
    }

    fn send_action(&self, action_type: &str, target: Option<&str>, dialogue: Option<&str>) -> Result<()> {
        let client = self.client.clone();
        let action_type = action_type.to_string();
        let target = target.map(|s| s.to_string());
        let dialogue = dialogue.map(|s| s.to_string());
        tokio::spawn(async move {
            if let Err(e) = client.send_action(&action_type, target.as_deref(), dialogue.as_deref()).await {
                tracing::error!("Failed to send action: {}", e);
            }
        });
        Ok(())
    }

    fn request_scene_change(&self, scene_id: &str) -> Result<()> {
        let msg = ClientMessage::RequestSceneChange { scene_id: scene_id.to_string() };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to request scene change: {}", e);
            }
        });
        Ok(())
    }

    fn send_directorial_update(&self, context: DirectorialContext) -> Result<()> {
        let msg = ClientMessage::DirectorialUpdate { context };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to send directorial update: {}", e);
            }
        });
        Ok(())
    }

    fn send_approval_decision(&self, request_id: &str, decision: ApprovalDecision) -> Result<()> {
        let msg = ClientMessage::ApprovalDecision {
            request_id: request_id.to_string(),
            decision,
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to send approval decision: {}", e);
            }
        });
        Ok(())
    }

    fn send_challenge_outcome_decision(&self, resolution_id: &str, decision: ChallengeOutcomeDecisionData) -> Result<()> {
        let msg = ClientMessage::ChallengeOutcomeDecision {
            resolution_id: resolution_id.to_string(),
            decision,
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to send challenge outcome decision: {}", e);
            }
        });
        Ok(())
    }

    fn trigger_challenge(&self, challenge_id: &str, target_character_id: &str) -> Result<()> {
        let msg = ClientMessage::TriggerChallenge {
            challenge_id: challenge_id.to_string(),
            target_character_id: target_character_id.to_string(),
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to trigger challenge: {}", e);
            }
        });
        Ok(())
    }

    fn submit_challenge_roll(&self, challenge_id: &str, roll: i32) -> Result<()> {
        let msg = ClientMessage::ChallengeRoll {
            challenge_id: challenge_id.to_string(),
            roll,
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to submit challenge roll: {}", e);
            }
        });
        Ok(())
    }

    fn submit_challenge_roll_input(&self, challenge_id: &str, input: DiceInputType) -> Result<()> {
        let msg = ClientMessage::ChallengeRollInput {
            challenge_id: challenge_id.to_string(),
            input_type: input,
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to submit challenge roll input: {}", e);
            }
        });
        Ok(())
    }

    fn heartbeat(&self) -> Result<()> {
        let msg = ClientMessage::Heartbeat;
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to send heartbeat: {}", e);
            }
        });
        Ok(())
    }

    fn move_to_region(&self, pc_id: &str, region_id: &str) -> Result<()> {
        let msg = ClientMessage::MoveToRegion {
            pc_id: pc_id.to_string(),
            region_id: region_id.to_string(),
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to send move to region: {}", e);
            }
        });
        Ok(())
    }

    fn exit_to_location(&self, pc_id: &str, location_id: &str, arrival_region_id: Option<&str>) -> Result<()> {
        let msg = ClientMessage::ExitToLocation {
            pc_id: pc_id.to_string(),
            location_id: location_id.to_string(),
            arrival_region_id: arrival_region_id.map(|s| s.to_string()),
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to send exit to location: {}", e);
            }
        });
        Ok(())
    }

    fn send_staging_approval(
        &self,
        request_id: &str,
        approved_npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
        source: &str,
    ) -> Result<()> {
        let msg = ClientMessage::StagingApprovalResponse {
            request_id: request_id.to_string(),
            approved_npcs,
            ttl_hours,
            source: source.to_string(),
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to send staging approval: {}", e);
            }
        });
        Ok(())
    }

    fn request_staging_regenerate(&self, request_id: &str, guidance: &str) -> Result<()> {
        let msg = ClientMessage::StagingRegenerateRequest {
            request_id: request_id.to_string(),
            guidance: guidance.to_string(),
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to send staging regenerate request: {}", e);
            }
        });
        Ok(())
    }

    fn pre_stage_region(
        &self,
        region_id: &str,
        npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
    ) -> Result<()> {
        let msg = ClientMessage::PreStageRegion {
            region_id: region_id.to_string(),
            npcs,
            ttl_hours,
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to send pre-stage region: {}", e);
            }
        });
        Ok(())
    }

    fn create_adhoc_challenge(
        &self,
        challenge_name: &str,
        skill_name: &str,
        difficulty: &str,
        target_pc_id: &str,
        outcomes: AdHocOutcomes,
    ) -> Result<()> {
        let msg = ClientMessage::CreateAdHocChallenge {
            challenge_name: challenge_name.to_string(),
            skill_name: skill_name.to_string(),
            difficulty: difficulty.to_string(),
            target_pc_id: target_pc_id.to_string(),
            outcomes,
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to create ad-hoc challenge: {}", e);
            }
        });
        Ok(())
    }

    fn equip_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        let msg = ClientMessage::EquipItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to equip item: {}", e);
            }
        });
        Ok(())
    }

    fn unequip_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        let msg = ClientMessage::UnequipItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to unequip item: {}", e);
            }
        });
        Ok(())
    }

    fn drop_item(&self, pc_id: &str, item_id: &str, quantity: u32) -> Result<()> {
        let msg = ClientMessage::DropItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
            quantity,
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to drop item: {}", e);
            }
        });
        Ok(())
    }

    fn pickup_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        let msg = ClientMessage::PickupItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to pick up item: {}", e);
            }
        });
        Ok(())
    }

    fn check_comfyui_health(&self) -> Result<()> {
        let msg = ClientMessage::CheckComfyUIHealth;
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to check ComfyUI health: {}", e);
            }
        });
        Ok(())
    }

    fn set_npc_disposition(&self, npc_id: &str, pc_id: &str, disposition: &str, reason: Option<&str>) -> Result<()> {
        let msg = ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload: RequestPayload::SetNpcDisposition {
                npc_id: npc_id.to_string(),
                pc_id: pc_id.to_string(),
                disposition: disposition.to_string(),
                reason: reason.map(|s| s.to_string()),
            },
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to set NPC disposition: {}", e);
            }
        });
        Ok(())
    }

    fn set_npc_relationship(&self, npc_id: &str, pc_id: &str, relationship: &str) -> Result<()> {
        let msg = ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload: RequestPayload::SetNpcRelationship {
                npc_id: npc_id.to_string(),
                pc_id: pc_id.to_string(),
                relationship: relationship.to_string(),
            },
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to set NPC relationship: {}", e);
            }
        });
        Ok(())
    }

    fn get_npc_dispositions(&self, pc_id: &str) -> Result<()> {
        let msg = ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload: RequestPayload::GetNpcDispositions {
                pc_id: pc_id.to_string(),
            },
        };
        let client = self.client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.send(msg).await {
                tracing::error!("Failed to get NPC dispositions: {}", e);
            }
        });
        Ok(())
    }

    fn on_state_change(&self, callback: Box<dyn FnMut(PortConnectionState) + Send + 'static>) {
        let state_slot = Arc::clone(&self.state);
        let cb = Arc::new(tokio::sync::Mutex::new(callback));

        let cb_for_engine = Arc::clone(&cb);
        let state_for_engine = Arc::clone(&state_slot);
        let client = self.client.clone();

        tokio::spawn(async move {
            client
                .set_on_state_change(move |infra_state| {
                    let port_state = map_state(infra_state);
                    state_for_engine.store(state_to_u8(port_state), Ordering::SeqCst);

                    let cb_for_call = Arc::clone(&cb_for_engine);
                    tokio::spawn(async move {
                        let mut cb = cb_for_call.lock().await;
                        (cb)(port_state);
                    });
                })
                .await;
        });
    }

    fn on_message(&self, callback: Box<dyn FnMut(serde_json::Value) + Send + 'static>) {
        let cb = Arc::new(tokio::sync::Mutex::new(callback));
        let cb_for_engine = Arc::clone(&cb);
        let client = self.client.clone();

        tokio::spawn(async move {
            client
                .set_on_message(move |msg| {
                    let value = serde_json::to_value(msg).unwrap_or(serde_json::Value::Null);
                    let cb_for_call = Arc::clone(&cb_for_engine);
                    tokio::spawn(async move {
                        let mut cb = cb_for_call.lock().await;
                        (cb)(value);
                    });
                })
                .await;
        });
    }

    fn request(
        &self,
        payload: RequestPayload,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseResult, RequestError>> + Send + '_>> {
        let client = self.client.clone();
        
        Box::pin(async move {
            client.request(payload).await
        })
    }

    fn request_with_timeout(
        &self,
        payload: RequestPayload,
        timeout_ms: u64,
    ) -> Pin<Box<dyn Future<Output = Result<ResponseResult, RequestError>> + Send + '_>> {
        let client = self.client.clone();
        
        Box::pin(async move {
            client.request_with_timeout(payload, timeout_ms).await
        })
    }
}
