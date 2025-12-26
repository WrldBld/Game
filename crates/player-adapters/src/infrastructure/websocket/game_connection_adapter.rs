//! Adapter implementing the application `GameConnectionPort` for `EngineClient`.
//!
//! This allows higher layers (presentation/application) to depend on the port
//! rather than the concrete WebSocket client type.

use anyhow::Result;
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

use wrldbldr_player_ports::outbound::{ConnectionState as PortConnectionState, GameConnectionPort};
use wrldbldr_protocol::{
    AdHocOutcomes, ApprovedNpcInfo, ApprovalDecision, ChallengeOutcomeDecisionData, ClientMessage, DiceInputType, DirectorialContext, ParticipantRole,
};
use super::{ConnectionState as InfraConnectionState, EngineClient};

fn map_state(state: InfraConnectionState) -> PortConnectionState {
    match state {
        InfraConnectionState::Disconnected => PortConnectionState::Disconnected,
        InfraConnectionState::Connecting => PortConnectionState::Connecting,
        InfraConnectionState::Connected => PortConnectionState::Connected,
        InfraConnectionState::Reconnecting => PortConnectionState::Reconnecting,
        InfraConnectionState::Failed => PortConnectionState::Failed,
    }
}

fn state_to_u8(state: PortConnectionState) -> u8 {
    match state {
        PortConnectionState::Disconnected => 0,
        PortConnectionState::Connecting => 1,
        PortConnectionState::Connected => 2,
        PortConnectionState::Reconnecting => 3,
        PortConnectionState::Failed => 4,
    }
}

fn u8_to_state(v: u8) -> PortConnectionState {
    match v {
        1 => PortConnectionState::Connecting,
        2 => PortConnectionState::Connected,
        3 => PortConnectionState::Reconnecting,
        4 => PortConnectionState::Failed,
        _ => PortConnectionState::Disconnected,
    }
}

/// Concrete adapter wrapping an `EngineClient`.
#[derive(Clone)]
pub struct EngineGameConnection {
    client: EngineClient,
    state: Arc<AtomicU8>,
}

impl EngineGameConnection {
    pub fn new(client: EngineClient) -> Self {
        let initial = {
            #[cfg(target_arch = "wasm32")]
            {
                state_to_u8(map_state(client.state()))
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                state_to_u8(PortConnectionState::Disconnected)
            }
        };

        Self {
            client,
            state: Arc::new(AtomicU8::new(initial)),
        }
    }
}

impl GameConnectionPort for EngineGameConnection {
    fn state(&self) -> PortConnectionState {
        u8_to_state(self.state.load(Ordering::SeqCst))
    }

    fn url(&self) -> &str {
        self.client.url()
    }

    fn connect(&self) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            self.state.store(state_to_u8(PortConnectionState::Connecting), Ordering::SeqCst);
            self.client.connect()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
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
    }

    fn disconnect(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            self.client.disconnect();
            self.state.store(state_to_u8(PortConnectionState::Disconnected), Ordering::SeqCst);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            let state = Arc::clone(&self.state);
            tokio::spawn(async move {
                client.disconnect().await;
                state.store(state_to_u8(PortConnectionState::Disconnected), Ordering::SeqCst);
            });
        }
    }

    fn join_session(
        &self,
        user_id: &str,
        role: ParticipantRole,
        world_id: Option<String>,
    ) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            self.client.join_session(user_id, role, world_id)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            let user_id = user_id.to_string();
            let world_id = world_id.clone();
            tokio::spawn(async move {
                if let Err(e) = client.join_session(&user_id, role, world_id).await {
                    tracing::error!("Failed to join session: {}", e);
                }
            });
            Ok(())
        }
    }

    fn send_action(&self, action_type: &str, target: Option<&str>, dialogue: Option<&str>) -> Result<()> {
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send_action(action_type, target, dialogue)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
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
    }

    fn request_scene_change(&self, scene_id: &str) -> Result<()> {
        let msg = ClientMessage::RequestSceneChange { scene_id: scene_id.to_string() };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to request scene change: {}", e);
                }
            });
            Ok(())
        }
    }

    fn send_directorial_update(&self, context: DirectorialContext) -> Result<()> {
        let msg = ClientMessage::DirectorialUpdate { context };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to send directorial update: {}", e);
                }
            });
            Ok(())
        }
    }

    fn send_approval_decision(&self, request_id: &str, decision: ApprovalDecision) -> Result<()> {
        let msg = ClientMessage::ApprovalDecision {
            request_id: request_id.to_string(),
            decision,
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to send approval decision: {}", e);
                }
            });
            Ok(())
        }
    }

    fn send_challenge_outcome_decision(&self, resolution_id: &str, decision: ChallengeOutcomeDecisionData) -> Result<()> {
        let msg = ClientMessage::ChallengeOutcomeDecision {
            resolution_id: resolution_id.to_string(),
            decision,
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to send challenge outcome decision: {}", e);
                }
            });
            Ok(())
        }
    }

    fn trigger_challenge(&self, challenge_id: &str, target_character_id: &str) -> Result<()> {
        let msg = ClientMessage::TriggerChallenge {
            challenge_id: challenge_id.to_string(),
            target_character_id: target_character_id.to_string(),
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to trigger challenge: {}", e);
                }
            });
            Ok(())
        }
    }

    fn submit_challenge_roll(&self, challenge_id: &str, roll: i32) -> Result<()> {
        let msg = ClientMessage::ChallengeRoll {
            challenge_id: challenge_id.to_string(),
            roll,
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to submit challenge roll: {}", e);
                }
            });
            Ok(())
        }
    }

    fn submit_challenge_roll_input(&self, challenge_id: &str, input: DiceInputType) -> Result<()> {
        let msg = ClientMessage::ChallengeRollInput {
            challenge_id: challenge_id.to_string(),
            input_type: input,
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to submit challenge roll input: {}", e);
                }
            });
            Ok(())
        }
    }

    fn heartbeat(&self) -> Result<()> {
        let msg = ClientMessage::Heartbeat;
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to send heartbeat: {}", e);
                }
            });
            Ok(())
        }
    }

    fn move_to_region(&self, pc_id: &str, region_id: &str) -> Result<()> {
        let msg = ClientMessage::MoveToRegion {
            pc_id: pc_id.to_string(),
            region_id: region_id.to_string(),
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to send move to region: {}", e);
                }
            });
            Ok(())
        }
    }

    fn exit_to_location(&self, pc_id: &str, location_id: &str, arrival_region_id: Option<&str>) -> Result<()> {
        let msg = ClientMessage::ExitToLocation {
            pc_id: pc_id.to_string(),
            location_id: location_id.to_string(),
            arrival_region_id: arrival_region_id.map(|s| s.to_string()),
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to send exit to location: {}", e);
                }
            });
            Ok(())
        }
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
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to send staging approval: {}", e);
                }
            });
            Ok(())
        }
    }

    fn request_staging_regenerate(&self, request_id: &str, guidance: &str) -> Result<()> {
        let msg = ClientMessage::StagingRegenerateRequest {
            request_id: request_id.to_string(),
            guidance: guidance.to_string(),
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to send staging regenerate request: {}", e);
                }
            });
            Ok(())
        }
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
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to send pre-stage region: {}", e);
                }
            });
            Ok(())
        }
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
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to create ad-hoc challenge: {}", e);
                }
            });
            Ok(())
        }
    }

    fn equip_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        let msg = ClientMessage::EquipItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to equip item: {}", e);
                }
            });
            Ok(())
        }
    }

    fn unequip_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        let msg = ClientMessage::UnequipItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to unequip item: {}", e);
                }
            });
            Ok(())
        }
    }

    fn drop_item(&self, pc_id: &str, item_id: &str, quantity: u32) -> Result<()> {
        let msg = ClientMessage::DropItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
            quantity,
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to drop item: {}", e);
                }
            });
            Ok(())
        }
    }

    fn pickup_item(&self, pc_id: &str, item_id: &str) -> Result<()> {
        let msg = ClientMessage::PickupItem {
            pc_id: pc_id.to_string(),
            item_id: item_id.to_string(),
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to pick up item: {}", e);
                }
            });
            Ok(())
        }
    }

    fn check_comfyui_health(&self) -> Result<()> {
        let msg = ClientMessage::CheckComfyUIHealth;
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to check ComfyUI health: {}", e);
                }
            });
            Ok(())
        }
    }

    fn set_npc_mood(&self, npc_id: &str, pc_id: &str, mood: &str, reason: Option<&str>) -> Result<()> {
        use wrldbldr_protocol::RequestPayload;
        
        let msg = ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload: RequestPayload::SetNpcMood {
                npc_id: npc_id.to_string(),
                pc_id: pc_id.to_string(),
                mood: mood.to_string(),
                reason: reason.map(|s| s.to_string()),
            },
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to set NPC mood: {}", e);
                }
            });
            Ok(())
        }
    }

    fn set_npc_relationship(&self, npc_id: &str, pc_id: &str, relationship: &str) -> Result<()> {
        use wrldbldr_protocol::RequestPayload;
        
        let msg = ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload: RequestPayload::SetNpcRelationship {
                npc_id: npc_id.to_string(),
                pc_id: pc_id.to_string(),
                relationship: relationship.to_string(),
            },
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to set NPC relationship: {}", e);
                }
            });
            Ok(())
        }
    }

    fn get_npc_moods(&self, pc_id: &str) -> Result<()> {
        use wrldbldr_protocol::RequestPayload;
        
        let msg = ClientMessage::Request {
            request_id: uuid::Uuid::new_v4().to_string(),
            payload: RequestPayload::GetNpcMoods {
                pc_id: pc_id.to_string(),
            },
        };
        #[cfg(target_arch = "wasm32")]
        {
            self.client.send(msg)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let client = self.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send(msg).await {
                    tracing::error!("Failed to get NPC moods: {}", e);
                }
            });
            Ok(())
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(target_arch = "wasm32")]
    fn on_state_change(&self, callback: Box<dyn FnMut(PortConnectionState) + 'static>) {
        use std::cell::RefCell;
        use std::rc::Rc;

        let state_slot = Arc::clone(&self.state);
        let cb = Rc::new(RefCell::new(callback));

        let cb_for_engine = Rc::clone(&cb);
        self.client.set_on_state_change(move |infra_state| {
            let port_state = map_state(infra_state);
            state_slot.store(state_to_u8(port_state), Ordering::SeqCst);
            (cb_for_engine.borrow_mut())(port_state);
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(target_arch = "wasm32")]
    fn on_message(&self, callback: Box<dyn FnMut(serde_json::Value) + 'static>) {
        use std::cell::RefCell;
        use std::rc::Rc;

        let cb = Rc::new(RefCell::new(callback));
        let cb_for_engine = Rc::clone(&cb);
        self.client.set_on_message(move |msg| {
            let value = serde_json::to_value(msg).unwrap_or(serde_json::Value::Null);
            (cb_for_engine.borrow_mut())(value);
        });
    }
}

