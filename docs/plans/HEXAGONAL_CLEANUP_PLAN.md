# Hexagonal Architecture Cleanup Plan

**Status**: SUPERSEDED  
**Created**: 2025-12-28  
**Superseded By**: [HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md](./HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md)  

---

> **Note**: This plan has been completed and superseded. All phases (1-6) were implemented.
> A comprehensive validation found additional violations requiring a new master plan.
> See `HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md` for the current work.

---

## Original Plan (Completed)

**Original Estimated Effort**: 7-9 hours  

## Executive Summary

This plan addresses remaining hexagonal architecture violations discovered during the validation of the challenge approval refactoring. The main issues are:

| Priority | Issue | Effort |
|----------|-------|--------|
| **P1** | Handler helpers duplicated across 5 files (blocks arch-check) | 30 min |
| **P1** | `narrative.rs` handler bypasses use case layer | 1-2 hrs |
| **P1** | `ChallengeOutcomeApprovalService` has 8 direct `ServerMessage` constructions | 4-5 hrs |
| **P2** | Orphaned fields in `ChallengeResolutionService` | 15 min |
| **P2** | `NpcDispositionStateDto` defined in domain layer | 30 min |
| **P2** | `inventory.rs` handler accesses repository directly | 30 min |

---

## Phase 1: Extract Shared Handler Helpers

**Priority**: P1  
**Effort**: 30 minutes  
**Goal**: Pass `arch-check` by reducing `challenge.rs` below 400 lines.

### Problem

Duplicated helper functions across handler files:

| Function | Files |
|----------|-------|
| `extract_dm_context` | challenge.rs, misc.rs, scene.rs, staging.rs |
| `error_msg` | challenge.rs, inventory.rs, misc.rs, scene.rs, staging.rs |
| `extract_player_context` | challenge.rs (unique but reusable) |

### Solution

Create `crates/engine-adapters/src/infrastructure/websocket/handlers/common.rs`:

```rust
//! Common handler utilities
//!
//! Shared helper functions for WebSocket message handlers.

use uuid::Uuid;
use wrldbldr_domain::{PlayerCharacterId, WorldId};
use wrldbldr_engine_ports::inbound::UseCaseContext;
use wrldbldr_protocol::ServerMessage;

use crate::infrastructure::state::AppState;

/// Create a ServerMessage::Error with the given code and message
pub fn error_msg(code: &str, message: &str) -> ServerMessage {
    ServerMessage::Error {
        code: code.to_string(),
        message: message.to_string(),
    }
}

/// Extract DM context (world_id + authorization check)
///
/// Returns Err(ServerMessage) if not connected or not a DM.
pub async fn extract_dm_context(
    state: &AppState,
    client_id: Uuid,
) -> Result<UseCaseContext, ServerMessage> {
    let client_id_str = client_id.to_string();
    let connection = state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
        .ok_or_else(|| error_msg("NOT_CONNECTED", "Connection not found"))?;

    let world_id = connection
        .world_id
        .ok_or_else(|| error_msg("NO_WORLD", "Not connected to a world"))?;

    if !connection.is_dm() {
        return Err(error_msg("NOT_AUTHORIZED", "DM privileges required"));
    }

    Ok(UseCaseContext {
        world_id: WorldId::from_uuid(world_id),
        user_id: connection.user_id.clone(),
        is_dm: true,
        pc_id: None,
    })
}

/// Extract player context (world_id + pc_id)
///
/// Returns Err(ServerMessage) if not connected or no PC.
pub async fn extract_player_context(
    state: &AppState,
    client_id: Uuid,
) -> Result<(WorldId, PlayerCharacterId), ServerMessage> {
    let client_id_str = client_id.to_string();
    let connection = state
        .world_connection_manager
        .get_connection_by_client_id(&client_id_str)
        .await
        .ok_or_else(|| error_msg("NOT_CONNECTED", "Connection not found"))?;

    let world_id = connection
        .world_id
        .ok_or_else(|| error_msg("NO_WORLD", "Not connected to a world"))?;

    let pc_id = connection
        .player_character_id
        .ok_or_else(|| error_msg("NO_CHARACTER", "No character selected"))?;

    Ok((WorldId::from_uuid(world_id), PlayerCharacterId::from_uuid(pc_id)))
}
```

### Changes Required

| File | Action |
|------|--------|
| `handlers/mod.rs` | Add `pub mod common;` |
| `handlers/challenge.rs` | Remove lines 48-103, add `use super::common::*;` |
| `handlers/misc.rs` | Remove extract_dm_context and error_msg, add `use super::common::{extract_dm_context, error_msg};` |
| `handlers/scene.rs` | Remove extract_dm_context and error_msg, add `use super::common::{extract_dm_context, error_msg};` |
| `handlers/staging.rs` | Remove extract_dm_context and error_msg, add `use super::common::{extract_dm_context, error_msg};` |
| `handlers/inventory.rs` | Remove error_msg, add `use super::common::error_msg;` |

### Verification

```bash
cargo check -p wrldbldr-engine-adapters
cargo run -p xtask -- arch-check  # Should pass now
```

---

## Phase 2: Create NarrativeEventUseCase

**Priority**: P1  
**Effort**: 1-2 hours  
**Goal**: Move `narrative.rs` handler to use case pattern, eliminating direct broadcasting.

### Problem

`handlers/narrative.rs:62-82` directly:
1. Calls `narrative_event_approval_service` (service, not use case)
2. Constructs `ServerMessage::NarrativeEventTriggered`
3. Broadcasts via `world_connection_manager`

### Solution

#### 2.1 Add GameEvent::NarrativeEventTriggered

File: `crates/engine-ports/src/outbound/game_events.rs`

Add to `GameEvent` enum:

```rust
// === Narrative Events ===
/// Narrative event triggered (broadcast to all players)
NarrativeEventTriggered {
    event_id: NarrativeEventId,
    event_name: String,
    outcome_description: String,
    scene_direction: Option<String>,
},
```

#### 2.2 Add NarrativeEventError

File: `crates/engine-app/src/application/use_cases/errors.rs`

```rust
/// Narrative event operation errors
#[derive(Debug, Error)]
pub enum NarrativeEventError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Approval failed: {0}")]
    ApprovalFailed(String),
}

impl ErrorCode for NarrativeEventError {
    fn code(&self) -> &'static str {
        match self {
            Self::Unauthorized(_) => "NOT_AUTHORIZED",
            Self::ApprovalFailed(_) => "NARRATIVE_EVENT_ERROR",
        }
    }
}
```

#### 2.3 Create NarrativeEventUseCase

File: `crates/engine-app/src/application/use_cases/narrative_event.rs`

```rust
//! Narrative Event Use Case
//!
//! Handles DM approval of narrative event suggestions.

use std::sync::Arc;

use wrldbldr_engine_ports::inbound::UseCaseContext;
use wrldbldr_engine_ports::outbound::{BroadcastPort, GameEvent};

use crate::application::services::{
    NarrativeEventApprovalService, NarrativeEventService,
};

use super::errors::NarrativeEventError;

// =============================================================================
// Input/Output Types
// =============================================================================

/// Input for narrative event suggestion decision
#[derive(Debug, Clone)]
pub struct SuggestionDecisionInput {
    pub request_id: String,
    pub event_id: String,
    pub approved: bool,
    pub selected_outcome: Option<String>,
}

/// Result of a narrative event decision
#[derive(Debug, Clone)]
pub struct DecisionResult {
    /// Whether the event was triggered
    pub triggered: bool,
}

// =============================================================================
// Use Case
// =============================================================================

pub struct NarrativeEventUseCase<N: NarrativeEventService> {
    approval_service: Arc<NarrativeEventApprovalService<N>>,
    broadcast_port: Arc<dyn BroadcastPort>,
}

impl<N: NarrativeEventService> NarrativeEventUseCase<N> {
    pub fn new(
        approval_service: Arc<NarrativeEventApprovalService<N>>,
        broadcast_port: Arc<dyn BroadcastPort>,
    ) -> Self {
        Self {
            approval_service,
            broadcast_port,
        }
    }

    /// Handle DM's decision on a narrative event suggestion
    pub async fn handle_suggestion_decision(
        &self,
        ctx: UseCaseContext,
        input: SuggestionDecisionInput,
    ) -> Result<DecisionResult, NarrativeEventError> {
        // Verify DM authorization
        if !ctx.is_dm {
            return Err(NarrativeEventError::Unauthorized(
                "Only DM can approve narrative events".to_string(),
            ));
        }

        // Delegate to approval service
        let result = self
            .approval_service
            .handle_decision(
                ctx.world_id,
                input.request_id,
                input.event_id.clone(),
                input.approved,
                input.selected_outcome,
            )
            .await
            .map_err(|e| NarrativeEventError::ApprovalFailed(e.to_string()))?;

        // If approved, broadcast the event
        if let Some(trigger_result) = result {
            self.broadcast_port
                .broadcast(
                    ctx.world_id,
                    GameEvent::NarrativeEventTriggered {
                        event_id: trigger_result.event_id,
                        event_name: trigger_result.event_name,
                        outcome_description: trigger_result.outcome_description,
                        scene_direction: trigger_result.scene_direction,
                    },
                )
                .await;

            Ok(DecisionResult { triggered: true })
        } else {
            Ok(DecisionResult { triggered: false })
        }
    }
}
```

#### 2.4 Add to BroadcastAdapter

File: `crates/engine-adapters/src/infrastructure/websocket/broadcast_adapter.rs`

Add match arm for `GameEvent::NarrativeEventTriggered`:

```rust
GameEvent::NarrativeEventTriggered { event_id, event_name, outcome_description, scene_direction } => {
    let message = ServerMessage::NarrativeEventTriggered {
        event_id: event_id.into(),
        event_name,
        outcome_description,
        scene_direction: scene_direction.unwrap_or_default(),
    };
    self.world_connection_manager
        .broadcast_to_world(world_id.into(), message)
        .await;
}
```

#### 2.5 Update Handler

File: `crates/engine-adapters/src/infrastructure/websocket/handlers/narrative.rs`

```rust
//! Narrative event handlers

use uuid::Uuid;

use crate::infrastructure::state::AppState;
use super::common::{extract_dm_context, error_msg};
use wrldbldr_engine_app::application::use_cases::narrative_event::SuggestionDecisionInput;
use wrldbldr_protocol::ServerMessage;

pub async fn handle_narrative_event_suggestion_decision(
    state: &AppState,
    client_id: Uuid,
    request_id: String,
    event_id: String,
    approved: bool,
    selected_outcome: Option<String>,
) -> Option<ServerMessage> {
    let ctx = match extract_dm_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(e) => return Some(e),
    };

    let input = SuggestionDecisionInput {
        request_id,
        event_id,
        approved,
        selected_outcome,
    };

    match state.use_cases.narrative_event.handle_suggestion_decision(ctx, input).await {
        Ok(_) => None, // Use case broadcasts via BroadcastPort
        Err(e) => Some(e.into_server_error()),
    }
}
```

#### 2.6 Wire Up

1. Add `NarrativeEventUseCase` to `UseCases` struct in `state/mod.rs`
2. Register in `use_cases/mod.rs`
3. Update dispatch to pass `client_id` to narrative handler

---

## Phase 3: Refactor ChallengeOutcomeApprovalService

**Priority**: P1  
**Effort**: 4-5 hours  
**Goal**: Remove all 8 `ServerMessage` constructions using Channel + Publisher pattern.

### Problem

The service directly constructs and sends protocol messages at lines:
- 191-196 (`ProposedToolInfo`)
- 325-328, 377-380 (`OutcomeSuggestionReady`)
- 415-426 (`ChallengeResolved`)
- 487-500 (`ChallengeOutcomePending`)
- 516-524 (`ChallengeRollSubmitted`)
- 650-664 (`OutcomeBranchesReady`)
- 891-899 (`CharacterStatUpdated`)

### Solution: Channel + Publisher Pattern

This is the most hexagonally pure pattern, following the existing `GenerationEventPublisher`.

#### 3.1 Define ChallengeApprovalEvent

File: `crates/engine-app/src/application/services/challenge_approval_events.rs`

```rust
//! Challenge approval events for async notification
//!
//! These events are sent through a channel and processed by
//! ChallengeApprovalEventPublisher, which converts them to
//! GameEvent and broadcasts via BroadcastPort.

use wrldbldr_domain::WorldId;

/// Events emitted by ChallengeOutcomeApprovalService
#[derive(Debug, Clone)]
pub enum ChallengeApprovalEvent {
    /// Roll submitted, awaiting DM approval
    RollSubmitted {
        world_id: WorldId,
        resolution_id: String,
        challenge_id: String,
        challenge_name: String,
        character_id: String,
        character_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome_type: String,
        outcome_description: String,
        roll_breakdown: Option<String>,
        outcome_triggers: Vec<OutcomeTriggerInfo>,
    },
    
    /// Pending outcome sent to DM
    OutcomePending {
        world_id: WorldId,
        resolution_id: String,
        challenge_id: String,
        challenge_name: String,
        character_id: String,
        character_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome_type: String,
        outcome_description: String,
        roll_breakdown: Option<String>,
        outcome_triggers: Vec<OutcomeTriggerInfo>,
    },
    
    /// Challenge resolved and approved
    Resolved {
        world_id: WorldId,
        challenge_id: String,
        challenge_name: String,
        character_name: String,
        roll: i32,
        modifier: i32,
        total: i32,
        outcome: String,
        outcome_description: String,
        roll_breakdown: Option<String>,
        individual_rolls: Option<Vec<i32>>,
    },
    
    /// LLM suggestions ready
    SuggestionsReady {
        world_id: WorldId,
        resolution_id: String,
        suggestions: Vec<String>,
    },
    
    /// Outcome branches ready
    BranchesReady {
        world_id: WorldId,
        resolution_id: String,
        outcome_type: String,
        branches: Vec<OutcomeBranchInfo>,
    },
    
    /// Character stat updated
    StatUpdated {
        world_id: WorldId,
        character_id: String,
        stat_name: String,
        old_value: i32,
        new_value: i32,
    },
}

/// Outcome trigger info (mirrors domain type for event)
#[derive(Debug, Clone)]
pub struct OutcomeTriggerInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub arguments: serde_json::Value,
}

/// Outcome branch info
#[derive(Debug, Clone)]
pub struct OutcomeBranchInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub effects: Vec<String>,
}
```

#### 3.2 Create ChallengeApprovalEventPublisher

File: `crates/engine-app/src/application/services/challenge_approval_event_publisher.rs`

```rust
//! Challenge Approval Event Publisher
//!
//! Background task that receives ChallengeApprovalEvent from a channel
//! and broadcasts them via BroadcastPort as GameEvent.

use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
use wrldbldr_engine_ports::outbound::{BroadcastPort, GameEvent};

use super::challenge_approval_events::ChallengeApprovalEvent;

pub struct ChallengeApprovalEventPublisher {
    broadcast_port: Arc<dyn BroadcastPort>,
}

impl ChallengeApprovalEventPublisher {
    pub fn new(broadcast_port: Arc<dyn BroadcastPort>) -> Self {
        Self { broadcast_port }
    }

    /// Run the publisher, consuming events from the channel
    pub async fn run(self, mut rx: UnboundedReceiver<ChallengeApprovalEvent>) {
        tracing::info!("ChallengeApprovalEventPublisher started");
        
        while let Some(event) = rx.recv().await {
            let world_id = self.extract_world_id(&event);
            if let Some(game_event) = self.map_to_game_event(event) {
                self.broadcast_port.broadcast(world_id, game_event).await;
            }
        }
        
        tracing::info!("ChallengeApprovalEventPublisher stopped");
    }

    fn extract_world_id(&self, event: &ChallengeApprovalEvent) -> wrldbldr_domain::WorldId {
        match event {
            ChallengeApprovalEvent::RollSubmitted { world_id, .. } => *world_id,
            ChallengeApprovalEvent::OutcomePending { world_id, .. } => *world_id,
            ChallengeApprovalEvent::Resolved { world_id, .. } => *world_id,
            ChallengeApprovalEvent::SuggestionsReady { world_id, .. } => *world_id,
            ChallengeApprovalEvent::BranchesReady { world_id, .. } => *world_id,
            ChallengeApprovalEvent::StatUpdated { world_id, .. } => *world_id,
        }
    }

    fn map_to_game_event(&self, event: ChallengeApprovalEvent) -> Option<GameEvent> {
        match event {
            ChallengeApprovalEvent::RollSubmitted {
                world_id,
                resolution_id,
                challenge_id,
                challenge_name,
                character_id,
                character_name,
                roll,
                modifier,
                total,
                outcome_type,
                outcome_description,
                roll_breakdown,
                outcome_triggers,
            } => Some(GameEvent::ChallengeRollSubmitted {
                world_id,
                resolution_id,
                challenge_id,
                challenge_name,
                character_id,
                character_name,
                roll,
                modifier,
                total,
                outcome_type,
                outcome_description,
                roll_breakdown,
                individual_rolls: None,
                outcome_triggers: outcome_triggers
                    .into_iter()
                    .map(|t| wrldbldr_engine_ports::outbound::OutcomeTriggerInfo {
                        id: t.id,
                        name: t.name,
                        description: t.description,
                        arguments: t.arguments,
                    })
                    .collect(),
            }),
            
            ChallengeApprovalEvent::Resolved {
                world_id,
                challenge_id,
                challenge_name,
                character_name,
                roll,
                modifier,
                total,
                outcome,
                outcome_description,
                roll_breakdown,
                individual_rolls,
            } => Some(GameEvent::ChallengeResolved {
                world_id,
                challenge_id,
                challenge_name,
                character_name,
                roll,
                modifier,
                total,
                outcome,
                outcome_description,
                roll_breakdown,
                individual_rolls,
                state_changes: vec![], // State changes are broadcast separately
            }),
            
            ChallengeApprovalEvent::SuggestionsReady {
                resolution_id,
                suggestions,
                ..
            } => Some(GameEvent::ChallengeSuggestionsReady {
                resolution_id,
                suggestions,
            }),
            
            ChallengeApprovalEvent::BranchesReady {
                resolution_id,
                branches,
                ..
            } => Some(GameEvent::ChallengeBranchesReady {
                resolution_id,
                branches: branches
                    .into_iter()
                    .map(|b| wrldbldr_engine_ports::outbound::OutcomeBranchInfo {
                        branch_id: b.id,
                        title: b.title,
                        description: b.description,
                        effects: b.effects,
                    })
                    .collect(),
            }),
            
            ChallengeApprovalEvent::StatUpdated {
                world_id,
                character_id,
                stat_name,
                old_value,
                new_value,
            } => Some(GameEvent::CharacterStatUpdated {
                world_id,
                character_id,
                stat_name,
                old_value,
                new_value,
            }),
            
            // OutcomePending is sent to DM only, handled differently
            ChallengeApprovalEvent::OutcomePending { .. } => {
                // This should use send_to_dm, not broadcast
                // We'll handle this via a separate DM-specific event
                None
            }
        }
    }
}
```

#### 3.3 Update ChallengeOutcomeApprovalService

Replace `WorldConnectionPort` with channel sender:

```rust
pub struct ChallengeOutcomeApprovalService<L: LlmPort> {
    // Remove: world_connection: Arc<dyn WorldConnectionPort>,
    // Add:
    event_sender: mpsc::UnboundedSender<ChallengeApprovalEvent>,
    // ... rest unchanged
}

// In queue_for_approval(), replace:
// self.world_connection.broadcast_to_world(...)
// With:
let _ = self.event_sender.send(ChallengeApprovalEvent::RollSubmitted { ... });
```

#### 3.4 Add GameEvent Variants (if missing)

Check `game_events.rs` for missing variants and add:
- `CharacterStatUpdated` (for stat changes from triggers)

#### 3.5 Wire Up in Server

File: `crates/engine-adapters/src/run/server.rs`

```rust
// Create channel
let (challenge_approval_tx, challenge_approval_rx) = tokio::sync::mpsc::unbounded_channel();

// Start publisher
let challenge_approval_worker = {
    let broadcast_port = state.broadcast_port.clone();
    let publisher = ChallengeApprovalEventPublisher::new(broadcast_port);
    tokio::spawn(async move {
        publisher.run(challenge_approval_rx).await;
    })
};

// Add to tokio::select!
```

---

## Phase 4: Remove Orphaned Fields

**Priority**: P2  
**Effort**: 15 minutes  

### Problem

`ChallengeResolutionService` has two unused fields from the refactoring:
- `event_bus: Arc<dyn EventBusPort<AppEvent>>` (line 190)
- `outcome_trigger_service: Arc<OutcomeTriggerService>` (line 192)

### Solution

1. Remove fields from struct definition
2. Remove from `new()` constructor
3. Update call site in `state/mod.rs`

---

## Phase 5: Move NpcDispositionStateDto

**Priority**: P2  
**Effort**: 30 minutes  

### Problem

`NpcDispositionStateDto` is defined in domain layer but is a DTO for protocol serialization.

### Solution

1. Create `crates/protocol/src/dto.rs`
2. Move `NpcDispositionStateDto` and its impls
3. Update imports in affected files
4. Update `lib.rs` exports

---

## Phase 6: Fix Inventory Handler Context

**Priority**: P2  
**Effort**: 30 minutes  

### Problem

`handlers/inventory.rs:173-199` accesses repositories directly.

### Solution

Thread `client_id` through to inventory handlers (consistent with all other handlers):

1. Update `dispatch.rs` to pass `client_id` to inventory handlers
2. Update inventory handler signatures
3. Use `extract_player_context` from `common.rs`
4. Remove `extract_inventory_context` function

---

## Verification Checklist

After all phases:

```bash
# Must pass
cargo check --workspace
cargo clippy --workspace --all-targets
cargo run -p xtask -- arch-check

# Should have no ServerMessage in engine-app services
grep -r "ServerMessage::" crates/engine-app/src/application/services/ | grep -v "// "

# Check line counts
wc -l crates/engine-adapters/src/infrastructure/websocket/handlers/*.rs
```

---

## Implementation Order

1. **Phase 1** - Handler helpers (unblocks arch-check)
2. **Phase 4** - Remove orphaned fields (quick win)
3. **Phase 6** - Inventory handler fix (quick win)
4. **Phase 5** - Move DTO (quick win)
5. **Phase 2** - NarrativeEventUseCase
6. **Phase 3** - ChallengeOutcomeApprovalService (largest change)

This order maximizes early wins and unblocks arch-check first.
