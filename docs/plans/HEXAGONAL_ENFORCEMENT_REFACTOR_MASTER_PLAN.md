# Hexagonal Architecture Enforcement Master Plan

**Status**: In Progress  
**Created**: 2025-12-28  
**Last Validated**: 2025-12-28 (Final validation with sub-agent analysis)  
**Validation Status**: All phases validated. Minor discrepancy: RequestPayload variants = 134 (not 114)

## Progress Tracking

| Phase | Status | Commit |
|-------|--------|--------|
| Pre-Implementation Cleanup | Complete | 6e51a1d |
| Phase E6 (arch-check) | Complete | dcd376c |
| Phase E1 (DomainEvent) | Complete | 7f76736 |
| Phase E2 (Approval DTOs) | Complete | 7c6faa9 |
| Phase E3 (ErrorCode) | Complete | f0f470f |
| Phase E4 (Ports Layer) | Complete | 2ed181b |
| Phase E5 (Split Handler) | Partial | 6957955 |
| arch-check exemptions | Complete | c0ffe75 |
| Build fix (actantial) | Complete | ee8e313 |
| Phase P6 (Player arch-check) | Complete | f1c2298 |
| Phase P1 (Player Domain Types) | Partial | f1c2298 |
| Phase P2 (GameConnectionPort) | Pending | - |
| Phase P3 (Message Translation) | Pending | - |
| Phase P4 (player-app Services) | Pending | - |
| Phase P5 (player-ui Components) | Pending | - |  

## Executive Summary

This plan enforces strict hexagonal architecture across the entire WrldBldr codebase. It addresses:

- **Engine-side**: 14 files with protocol coupling in engine-app/engine-ports
- **Player-side**: 49 files with protocol coupling across all player crates
- **arch-check**: Extended validation to catch all violations

### Goals

1. **Zero protocol imports** in application/domain layers (engine-app, player-app, domain)
2. **Ports use domain types only** (not protocol wire types)
3. **All protocol <-> domain mapping** happens in adapters layer
4. **arch-check validates** all rules automatically

### Architecture Principle

```
+---------------------------------------------------------------------+
|                         ADAPTERS LAYER                               |
|   (engine-adapters, player-adapters, player-ui)                      |
|   - HTTP handlers, WebSocket handlers, UI components                 |
|   - ONLY layer that imports wrldbldr_protocol                        |
|   - Maps protocol <-> domain/app types                               |
+---------------------------------------------------------------------+
|                        APPLICATION LAYER                             |
|   (engine-app, player-app)                                           |
|   - Services, use cases, DTOs                                        |
|   - NO protocol imports                                              |
|   - Uses domain types + app-local DTOs                               |
+---------------------------------------------------------------------+
|                          PORTS LAYER                                 |
|   (engine-ports, player-ports)                                       |
|   - Trait definitions only                                           |
|   - NO protocol imports (one documented exception)                   |
|   - Uses domain types only                                           |
+---------------------------------------------------------------------+
|                         DOMAIN LAYER                                 |
|   (domain)                                                           |
|   - Entities, value objects, domain events                           |
|   - Zero external dependencies (except serde, uuid, chrono)          |
+---------------------------------------------------------------------+
```

---

## Documented Exemptions

### engine-ports: RequestHandler trait

**File**: `crates/engine-ports/src/inbound/request_handler.rs`

**Exemption**: This trait uses `RequestPayload` and `ResponseResult` from protocol.

**Justification**: 
- The `RequestPayload` enum is domain-focused (entity operations like `CreateWorld`, `GetCharacter`)
- Creating ~97 equivalent `DomainCommand` variants would add 8-12 hours with minimal practical benefit
- The protocol crate is internal and controlled, not an external dependency
- This is an API boundary - the protocol types ARE the contract

**arch-check handling**: This file is explicitly exempted in the arch-check validation.

---

## Pre-Implementation Cleanup

Before starting the phases, these cleanup tasks must be completed:

### Cleanup 1: Delete Duplicate AppEvent

**Problem**: There are TWO `AppEvent` definitions:
- `crates/protocol/src/app_events.rs` (canonical)
- `crates/engine-app/src/application/dto/app_events.rs` (duplicate - should not exist)

**Action**: Delete `crates/engine-app/src/application/dto/app_events.rs`

### Cleanup 2: Remove pub use from actantial_service

**Problem**: `crates/player-app/src/application/services/actantial_service.rs` has `pub use wrldbldr_protocol::{...}` re-exporting protocol types.

**Action**: Remove the `pub use` re-export.

---

## Part 1: Engine-Side Cleanup

### Phase E1: Create DomainEvent Type

**Priority**: High  

#### Problem

`AppEvent` from `wrldbldr_protocol` is used in 5 files across engine-app and engine-ports:
- `story_event_service.rs`
- `narrative_event_service.rs`
- `generation_event_publisher.rs`
- `generation_queue_projection_service.rs`
- `app_event_repository_port.rs`

#### Validation Notes

- **SessionId does NOT exist** in domain - use `Option<String>` instead
- `EventBusPort` is currently generic (`EventBusPort<E>`) and does NOT directly import AppEvent - it will be updated to use `DomainEvent`
- SQLite adapter serializes events to JSON - mapper approach is compatible
- There is a duplicate `AppEvent` in engine-app that must be deleted first (see Pre-Implementation Cleanup)

#### Solution

Create a domain-layer event type and map to `AppEvent` at the adapter boundary.

#### Implementation

**Step 1: Create DomainEvent in domain layer**

File: `crates/domain/src/events/mod.rs`

```rust
//! Domain Events
//!
//! Coarse-grained events representing significant state changes in the domain.
//! These are the domain's internal events - they get mapped to protocol AppEvent
//! at the adapter boundary for persistence and cross-system communication.

use crate::{
    WorldId, StoryEventId, NarrativeEventId, ChallengeId, CharacterId,
};

/// Domain event for significant state changes
#[derive(Debug, Clone)]
pub enum DomainEvent {
    // === Story & Narrative ===
    
    /// A story event was created
    StoryEventCreated {
        story_event_id: StoryEventId,
        world_id: WorldId,
        event_type: String,
    },
    
    /// A narrative event was triggered
    NarrativeEventTriggered {
        event_id: NarrativeEventId,
        world_id: WorldId,
        event_name: String,
        outcome_name: String,
        session_id: Option<String>,  // String, not SessionId (type doesn't exist)
    },
    
    // === Challenge ===
    
    /// A challenge was resolved
    ChallengeResolved {
        challenge_id: Option<ChallengeId>,
        challenge_name: String,
        world_id: WorldId,
        character_id: CharacterId,
        success: bool,
        roll: Option<i32>,
        total: Option<i32>,
        session_id: Option<String>,
    },
    
    // === Generation (Asset/Image) ===
    
    /// Generation batch was queued
    GenerationBatchQueued {
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        position: u32,
        session_id: Option<String>,
    },
    
    /// Generation batch progress update
    GenerationBatchProgress {
        batch_id: String,
        progress: f32,
        session_id: Option<String>,
    },
    
    /// Generation batch completed
    GenerationBatchCompleted {
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        asset_count: u32,
        session_id: Option<String>,
    },
    
    /// Generation batch failed
    GenerationBatchFailed {
        batch_id: String,
        entity_type: String,
        entity_id: String,
        asset_type: String,
        error: String,
        session_id: Option<String>,
    },
    
    // === Suggestion (LLM Text) ===
    
    /// Suggestion was queued
    SuggestionQueued {
        request_id: String,
        field_type: String,
        entity_id: Option<String>,
        world_id: Option<WorldId>,
    },
    
    /// Suggestion progress update
    SuggestionProgress {
        request_id: String,
        status: String,
        world_id: Option<WorldId>,
    },
    
    /// Suggestion completed
    SuggestionCompleted {
        request_id: String,
        field_type: String,
        suggestions: Vec<String>,
        world_id: Option<WorldId>,
    },
    
    /// Suggestion failed
    SuggestionFailed {
        request_id: String,
        field_type: String,
        error: String,
        world_id: Option<WorldId>,
    },
}

impl DomainEvent {
    /// Get the event type as a string for logging/filtering
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::StoryEventCreated { .. } => "story_event_created",
            Self::NarrativeEventTriggered { .. } => "narrative_event_triggered",
            Self::ChallengeResolved { .. } => "challenge_resolved",
            Self::GenerationBatchQueued { .. } => "generation_batch_queued",
            Self::GenerationBatchProgress { .. } => "generation_batch_progress",
            Self::GenerationBatchCompleted { .. } => "generation_batch_completed",
            Self::GenerationBatchFailed { .. } => "generation_batch_failed",
            Self::SuggestionQueued { .. } => "suggestion_queued",
            Self::SuggestionProgress { .. } => "suggestion_progress",
            Self::SuggestionCompleted { .. } => "suggestion_completed",
            Self::SuggestionFailed { .. } => "suggestion_failed",
        }
    }
    
    /// Extract world_id if present
    pub fn world_id(&self) -> Option<WorldId> {
        match self {
            Self::StoryEventCreated { world_id, .. } => Some(*world_id),
            Self::NarrativeEventTriggered { world_id, .. } => Some(*world_id),
            Self::ChallengeResolved { world_id, .. } => Some(*world_id),
            Self::SuggestionQueued { world_id, .. } => *world_id,
            Self::SuggestionProgress { world_id, .. } => *world_id,
            Self::SuggestionCompleted { world_id, .. } => *world_id,
            Self::SuggestionFailed { world_id, .. } => *world_id,
            _ => None,
        }
    }
}
```

**Step 2: Export from domain crate**

File: `crates/domain/src/lib.rs` - add:
```rust
pub mod events;
pub use events::DomainEvent;
```

**Step 3: Update EventBusPort**

File: `crates/engine-ports/src/outbound/event_bus_port.rs`

The port is currently generic `EventBusPort<E>`. Change to use `DomainEvent` directly:
```rust
use wrldbldr_domain::DomainEvent;

#[async_trait]
pub trait EventBusPort: Send + Sync {
    async fn publish(&self, event: DomainEvent) -> Result<(), EventBusError>;
    fn subscribe(&self) -> broadcast::Receiver<DomainEvent>;
}
```

**Step 4: Rename and Update Repository Port**

Rename `crates/engine-ports/src/outbound/app_event_repository_port.rs` to `domain_event_repository_port.rs`:
```rust
use wrldbldr_domain::DomainEvent;

#[async_trait]
pub trait DomainEventRepositoryPort: Send + Sync {
    async fn insert(&self, event: &DomainEvent) -> Result<i64, DomainEventRepositoryError>;
    async fn fetch_since(&self, last_id: i64, limit: u32) 
        -> Result<Vec<(i64, DomainEvent, DateTime<Utc>)>, DomainEventRepositoryError>;
}
```

**Step 5: Create adapter mapper**

File: `crates/engine-adapters/src/infrastructure/event_bus/domain_event_mapper.rs`

```rust
//! Maps DomainEvent to AppEvent for persistence and wire transmission

use wrldbldr_domain::DomainEvent;
use wrldbldr_protocol::AppEvent;

impl From<DomainEvent> for AppEvent {
    fn from(event: DomainEvent) -> Self {
        match event {
            DomainEvent::StoryEventCreated { story_event_id, world_id, event_type } => {
                AppEvent::StoryEventCreated {
                    story_event_id: story_event_id.to_string(),
                    world_id: world_id.to_string(),
                    event_type,
                }
            }
            DomainEvent::NarrativeEventTriggered { event_id, world_id, event_name, outcome_name, session_id } => {
                AppEvent::NarrativeEventTriggered {
                    event_id: event_id.to_string(),
                    world_id: world_id.to_string(),
                    event_name,
                    outcome_name,
                    session_id,
                }
            }
            // ... implement all variants
        }
    }
}

impl TryFrom<AppEvent> for DomainEvent {
    type Error = String;
    
    fn try_from(event: AppEvent) -> Result<Self, Self::Error> {
        // Implement reverse mapping for event replay
        // Parse string IDs back to domain ID types
    }
}
```

**Step 6: Update services**

Update these files to use `DomainEvent` instead of `AppEvent`:
- `crates/engine-app/src/application/services/story_event_service.rs`
- `crates/engine-app/src/application/services/narrative_event_service.rs`
- `crates/engine-app/src/application/services/generation_event_publisher.rs`
- `crates/engine-app/src/application/services/generation_queue_projection_service.rs`

**Step 7: Update SQLite adapter**

File: `crates/engine-adapters/src/infrastructure/event_bus/sqlite_event_bus.rs`

Convert `DomainEvent` to `AppEvent` before persisting (AppEvent is the serialization format):
```rust
async fn publish(&self, event: DomainEvent) -> Result<(), EventBusError> {
    let app_event: AppEvent = event.clone().into();
    self.repository.insert(&app_event).await?;
    // ...
}
```

Note: The repository implementation continues to store `AppEvent` as JSON - this is the wire/storage format.

#### Files to Modify

| File | Action |
|------|--------|
| `crates/engine-app/src/application/dto/app_events.rs` | DELETE (duplicate) |
| `crates/domain/src/events/mod.rs` | Create |
| `crates/domain/src/lib.rs` | Add export |
| `crates/engine-ports/src/outbound/event_bus_port.rs` | Update trait (remove generic) |
| `crates/engine-ports/src/outbound/app_event_repository_port.rs` | Rename to domain_event_repository_port.rs |
| `crates/engine-ports/src/outbound/mod.rs` | Update exports |
| `crates/engine-adapters/src/infrastructure/event_bus/domain_event_mapper.rs` | Create |
| `crates/engine-adapters/src/infrastructure/event_bus/sqlite_event_bus.rs` | Update |
| `crates/engine-app/src/application/services/story_event_service.rs` | Update |
| `crates/engine-app/src/application/services/narrative_event_service.rs` | Update |
| `crates/engine-app/src/application/services/generation_event_publisher.rs` | Update |
| `crates/engine-app/src/application/services/generation_queue_projection_service.rs` | Update |

---

### Phase E2: Create Approval DTOs

**Priority**: High  

#### Problem

Protocol types used directly in engine-app:
- `ProposedToolInfo` - in queue_items.rs, llm_queue_service.rs, challenge.rs (dto), challenge_outcome_approval_service.rs
- `ChallengeSuggestionInfo` - in queue_items.rs, llm_queue_service.rs
- `NarrativeEventSuggestionInfo` - in queue_items.rs, llm_queue_service.rs
- `ApprovalDecision` - in queue_items.rs, dm_approval_queue_service.rs

#### Validation Notes

- `ChallengeSuggestionOutcomes` is a nested type that also needs a DTO
- There are TWO different `ApprovalDecision` types:
  - `wrldbldr_protocol::ApprovalDecision` (5 variants for wire protocol)
  - `engine-app::use_cases::scene::ApprovalDecision` (3 variants, already app-local)
- The scene handler already converts between them - this is the correct pattern
- `AcceptWithModification.modified_dialogue` is `String` in protocol, not `Option<String>`

#### Solution

Create app-layer equivalents and map at adapter boundary.

#### Implementation

**Step 1: Create approval DTOs**

File: `crates/engine-app/src/application/dto/approval.rs`

```rust
//! Approval-related DTOs for engine-app layer
//!
//! These types mirror wrldbldr_protocol approval types but are owned
//! by the application layer. Mapping to/from protocol types happens
//! in the adapters layer.

use std::collections::HashMap;

/// Proposed tool call information (app-layer version)
#[derive(Debug, Clone)]
pub struct ProposedToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub arguments: serde_json::Value,
}

/// Challenge suggestion information for DM approval
#[derive(Debug, Clone)]
pub struct ChallengeSuggestionInfo {
    pub challenge_id: String,
    pub challenge_name: String,
    pub skill_name: String,
    pub difficulty_display: String,
    pub confidence: String,
    pub reasoning: String,
    pub target_pc_id: Option<String>,
    pub outcomes: Option<ChallengeSuggestionOutcomes>,
}

/// Challenge suggestion outcomes
#[derive(Debug, Clone, Default)]
pub struct ChallengeSuggestionOutcomes {
    pub success: Option<String>,
    pub failure: Option<String>,
    pub critical_success: Option<String>,
    pub critical_failure: Option<String>,
}

/// Narrative event suggestion information
#[derive(Debug, Clone)]
pub struct NarrativeEventSuggestionInfo {
    pub event_id: String,
    pub event_name: String,
    pub description: String,
    pub scene_direction: String,
    pub confidence: String,
    pub reasoning: String,
    pub matched_triggers: Vec<String>,
    pub suggested_outcome: Option<String>,
}

/// DM's decision on an approval request (for DM approval queue)
///
/// Note: This is different from `use_cases::scene::ApprovalDecision` which
/// is a simpler type for scene-specific approvals. Both are valid app-layer
/// types for their respective use cases.
#[derive(Debug, Clone)]
pub enum DmApprovalDecision {
    /// Accept as-is
    Accept,
    
    /// Accept with item distribution
    AcceptWithRecipients {
        item_recipients: HashMap<String, Vec<String>>,
    },
    
    /// Accept with modifications
    AcceptWithModification {
        modified_dialogue: String,  // Not Option - matches protocol
        approved_tools: Vec<String>,
        rejected_tools: Vec<String>,
        item_recipients: HashMap<String, Vec<String>>,
    },
    
    /// Reject with feedback
    Reject {
        feedback: String,
    },
    
    /// DM takes over response
    TakeOver {
        dm_response: String,
    },
}
```

**Step 2: Create conversion traits in adapters**

File: `crates/engine-adapters/src/infrastructure/websocket/approval_converters.rs`

```rust
//! Converters between app-layer approval DTOs and protocol types

use wrldbldr_engine_app::application::dto::approval as app;
use wrldbldr_protocol as proto;

impl From<app::ProposedToolInfo> for proto::ProposedToolInfo {
    fn from(info: app::ProposedToolInfo) -> Self {
        Self {
            id: info.id,
            name: info.name,
            description: info.description,
            arguments: info.arguments,
        }
    }
}

impl From<proto::ProposedToolInfo> for app::ProposedToolInfo {
    fn from(info: proto::ProposedToolInfo) -> Self {
        Self {
            id: info.id,
            name: info.name,
            description: info.description,
            arguments: info.arguments,
        }
    }
}

impl From<app::DmApprovalDecision> for proto::ApprovalDecision {
    fn from(decision: app::DmApprovalDecision) -> Self {
        match decision {
            app::DmApprovalDecision::Accept => proto::ApprovalDecision::Accept,
            app::DmApprovalDecision::AcceptWithRecipients { item_recipients } => {
                proto::ApprovalDecision::AcceptWithRecipients { item_recipients }
            }
            app::DmApprovalDecision::AcceptWithModification { 
                modified_dialogue, approved_tools, rejected_tools, item_recipients 
            } => {
                proto::ApprovalDecision::AcceptWithModification {
                    modified_dialogue,
                    approved_tools,
                    rejected_tools,
                    item_recipients,
                }
            }
            app::DmApprovalDecision::Reject { feedback } => {
                proto::ApprovalDecision::Reject { feedback }
            }
            app::DmApprovalDecision::TakeOver { dm_response } => {
                proto::ApprovalDecision::TakeOver { dm_response }
            }
        }
    }
}

impl From<proto::ApprovalDecision> for app::DmApprovalDecision {
    fn from(decision: proto::ApprovalDecision) -> Self {
        match decision {
            proto::ApprovalDecision::Accept => app::DmApprovalDecision::Accept,
            proto::ApprovalDecision::AcceptWithRecipients { item_recipients } => {
                app::DmApprovalDecision::AcceptWithRecipients { item_recipients }
            }
            proto::ApprovalDecision::AcceptWithModification { 
                modified_dialogue, approved_tools, rejected_tools, item_recipients 
            } => {
                app::DmApprovalDecision::AcceptWithModification {
                    modified_dialogue,
                    approved_tools,
                    rejected_tools,
                    item_recipients,
                }
            }
            proto::ApprovalDecision::Reject { feedback } => {
                app::DmApprovalDecision::Reject { feedback }
            }
            proto::ApprovalDecision::TakeOver { dm_response } => {
                app::DmApprovalDecision::TakeOver { dm_response }
            }
        }
    }
}

// Implement for ChallengeSuggestionInfo, ChallengeSuggestionOutcomes, 
// NarrativeEventSuggestionInfo similarly
```

**Step 3: Update engine-app files**

Replace protocol imports with app DTO imports in:
- `crates/engine-app/src/application/dto/queue_items.rs`
- `crates/engine-app/src/application/dto/challenge.rs`
- `crates/engine-app/src/application/services/llm_queue_service.rs`
- `crates/engine-app/src/application/services/dm_approval_queue_service.rs`
- `crates/engine-app/src/application/services/challenge_outcome_approval_service.rs`

#### Files to Modify

| File | Action |
|------|--------|
| `crates/engine-app/src/application/dto/approval.rs` | Create |
| `crates/engine-app/src/application/dto/mod.rs` | Add export |
| `crates/engine-adapters/src/infrastructure/websocket/approval_converters.rs` | Create |
| `crates/engine-adapters/src/infrastructure/websocket/mod.rs` | Add export |
| `crates/engine-app/src/application/dto/queue_items.rs` | Update imports |
| `crates/engine-app/src/application/dto/challenge.rs` | Update imports |
| `crates/engine-app/src/application/services/llm_queue_service.rs` | Update imports |
| `crates/engine-app/src/application/services/dm_approval_queue_service.rs` | Update imports |
| `crates/engine-app/src/application/services/challenge_outcome_approval_service.rs` | Update imports |

---

### Phase E3: Move ErrorCode::into_server_error() to Adapters

**Priority**: Medium  

#### Problem

The `ErrorCode` trait in engine-app imports `wrldbldr_protocol::ServerMessage` to provide `into_server_error()` convenience method.

#### Validation Notes

- Actual count: 26 call sites across 7 handler files (not 28 across 8)
- Files using it: challenge.rs (11), inventory.rs (4), staging.rs (3), scene.rs (3), misc.rs (3), connection.rs (1), player_action.rs (1)
- narrative.rs does NOT use it (error message constructed differently)
- There is a test `test_into_server_error` at line 569-581 in errors.rs that needs to move

#### Solution

Move the `into_server_error()` implementation to the adapters layer as an extension trait.

#### Implementation

**Step 1: Create extension trait in adapters**

File: `crates/engine-adapters/src/infrastructure/websocket/error_conversion.rs`

```rust
//! Error to ServerMessage conversion for WebSocket handlers
//!
//! Provides the IntoServerError trait that converts any ErrorCode
//! implementing type into a ServerMessage::Error.

use std::fmt::Display;
use wrldbldr_engine_app::application::use_cases::ErrorCode;
use wrldbldr_protocol::ServerMessage;

/// Extension trait for converting use case errors to ServerMessage
pub trait IntoServerError {
    /// Convert this error into a ServerMessage::Error
    fn into_server_error(&self) -> ServerMessage;
}

impl<T: ErrorCode + Display + ?Sized> IntoServerError for T {
    fn into_server_error(&self) -> ServerMessage {
        ServerMessage::Error {
            code: self.code().to_string(),
            message: self.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_engine_app::application::use_cases::MovementError;

    #[test]
    fn test_into_server_error() {
        let err = MovementError::NotConnected;
        let server_msg = err.into_server_error();

        match server_msg {
            ServerMessage::Error { code, message } => {
                assert_eq!(code, "NOT_CONNECTED");
                assert_eq!(message, "Not connected to a world");
            }
            _ => panic!("Expected Error message"),
        }
    }
}
```

**Step 2: Export from websocket module**

File: `crates/engine-adapters/src/infrastructure/websocket/mod.rs`

Add:
```rust
pub mod error_conversion;
pub use error_conversion::IntoServerError;
```

**Step 3: Update ErrorCode trait**

File: `crates/engine-app/src/application/use_cases/errors.rs`

Remove the protocol import and the `into_server_error()` method:
```rust
// Remove: use wrldbldr_protocol::ServerMessage;

pub trait ErrorCode: Display {
    /// Get the error code string (e.g., "PC_NOT_FOUND")
    fn code(&self) -> &'static str;
    
    // Remove the into_server_error() method entirely
}
```

Also remove the `test_into_server_error` test (moved to adapters).

**Step 4: Update handler imports**

Add this import to each handler file:
```rust
use crate::infrastructure::websocket::error_conversion::IntoServerError;
```

Files to update (7 files, 26 call sites):
- `handlers/challenge.rs` (11 call sites)
- `handlers/inventory.rs` (4 call sites)
- `handlers/staging.rs` (3 call sites)
- `handlers/scene.rs` (3 call sites)
- `handlers/misc.rs` (3 call sites)
- `handlers/connection.rs` (1 call site)
- `handlers/player_action.rs` (1 call site)

#### Files to Modify

| File | Action |
|------|--------|
| `crates/engine-adapters/src/infrastructure/websocket/error_conversion.rs` | Create |
| `crates/engine-adapters/src/infrastructure/websocket/mod.rs` | Add export |
| `crates/engine-app/src/application/use_cases/errors.rs` | Remove method + import + test |
| `crates/engine-adapters/src/infrastructure/websocket/handlers/challenge.rs` | Add import |
| `crates/engine-adapters/src/infrastructure/websocket/handlers/inventory.rs` | Add import |
| `crates/engine-adapters/src/infrastructure/websocket/handlers/staging.rs` | Add import |
| `crates/engine-adapters/src/infrastructure/websocket/handlers/scene.rs` | Add import |
| `crates/engine-adapters/src/infrastructure/websocket/handlers/misc.rs` | Add import |
| `crates/engine-adapters/src/infrastructure/websocket/handlers/connection.rs` | Add import |
| `crates/engine-adapters/src/infrastructure/websocket/handlers/player_action.rs` | Add import |

---

### Phase E4: Clean Up Ports Layer

**Priority**: High  

#### Problem

engine-ports imports protocol types in trait definitions:
- `WorldConnectionPort` uses `ServerMessage`
- `DirectorialContextRepositoryPort` uses `DirectorialContext`
- `WorldExporterPort` uses `RuleSystemConfig`

#### Validation Notes (Sub-Agent Analysis 2025-12-28)

**DirectorialContext System Analysis:**

Three overlapping types currently exist:
1. **`DirectorialContext` (Protocol)** - Wire format at `protocol/src/messages.rs:842-847`
   - Simple strings: `scene_notes`, `tone`, `npc_motivations`, `forbidden_topics`
   - `NpcMotivationData` with `character_id`, `emotional_guidance`, `immediate_goal`, `secret_agenda`

2. **`DirectorialNotes` (Domain)** - Rich typed format at `domain/src/value_objects/directorial.rs:12-230`
   - Typed enums: `ToneGuidance` (11 variants), `PacingGuidance` (5 variants)
   - `NpcMotivation` with `current_mood`, `immediate_goal`, `secret_agenda`, `attitude_to_players`, `speech_patterns`
   - Has `to_prompt()` method for LLM formatting
   - **Currently NOT wired into the LLM flow** (gap identified)

3. **`DirectorialContextData` (Use Case)** - App DTO at `engine-app/src/application/use_cases/scene.rs:241-246`
   - Simplified DTO with `npc_motivations`, `scene_mood`, `pacing`, `dm_notes`

**Key Finding:** The domain `DirectorialNotes` type exists with rich typed fields and `to_prompt()` method but is NOT actually used in the flow. The system stores `DirectorialContext` (protocol) in SQLite and WorldStateManager, but the rich `DirectorialNotes` never reaches `generate_npc_response_with_direction()`.

**Other Validation:**
- **RuleSystemConfig already exists in domain!** Protocol re-exports it from domain. Just change the import path - trivial fix.
- **WorldConnectionPort should be REMOVED**, not updated. `BroadcastPort` is the correct abstraction (takes `GameEvent`, not `ServerMessage`).

#### Solution

1. Remove `WorldConnectionPort` entirely (use `BroadcastPort`)
2. **Promote `DirectorialNotes` as the canonical domain type** (don't create a new type)
3. Add `Serialize`/`Deserialize` to `DirectorialNotes`, `ToneGuidance`, `PacingGuidance`
4. Update ports to use `DirectorialNotes` instead of protocol's `DirectorialContext`
5. Remove `DirectorialContextData` from use cases (use `DirectorialNotes` directly)
6. Wire up the flow so `DirectorialNotes` reaches the LLM
7. Fix `WorldExporterPort` import (trivial - already in domain)

#### Implementation

**Step 1: Remove WorldConnectionPort**

Delete file: `crates/engine-ports/src/outbound/world_connection_port.rs`

Update `crates/engine-ports/src/outbound/mod.rs` to remove the export.

Find and update any remaining usages to use `BroadcastPort` instead. Based on validation, the port is documented in service comments but actual code uses `BroadcastPort`.

**Step 2: Add Serialization to DirectorialNotes (Domain)**

File: `crates/domain/src/value_objects/directorial.rs`

Add `Serialize`, `Deserialize` derives to existing types:

```rust
use serde::{Deserialize, Serialize};

/// Structured directorial notes for a scene (canonical domain type)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectorialNotes {
    pub general_notes: String,
    pub tone: ToneGuidance,
    pub npc_motivations: HashMap<String, NpcMotivation>,
    pub forbidden_topics: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub suggested_beats: Vec<String>,
    pub pacing: PacingGuidance,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToneGuidance {
    #[default]
    Neutral,
    Serious,
    Lighthearted,
    Tense,
    Mysterious,
    Exciting,
    Contemplative,
    Creepy,
    Romantic,
    Comedic,
    Custom(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PacingGuidance {
    #[default]
    Natural,
    Fast,
    Slow,
    Building,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMotivation {
    pub current_mood: String,
    pub immediate_goal: String,
    pub secret_agenda: Option<String>,
    pub attitude_to_players: String,
    #[serde(default)]
    pub speech_patterns: Vec<String>,
}
```

**Step 3: Update DirectorialContextRepositoryPort**

File: `crates/engine-ports/src/outbound/directorial_context_port.rs`

```rust
use wrldbldr_domain::value_objects::DirectorialNotes;  // Changed from protocol DirectorialContext

#[async_trait]
pub trait DirectorialContextRepositoryPort: Send + Sync {
    async fn get(&self, world_id: &WorldId) -> Result<Option<DirectorialNotes>>;
    async fn save(&self, world_id: &WorldId, notes: &DirectorialNotes) -> Result<()>;
    async fn delete(&self, world_id: &WorldId) -> Result<()>;
}
```

**Step 4: Remove DirectorialContextData from Use Cases**

File: `crates/engine-app/src/application/use_cases/scene.rs`

Delete `DirectorialContextData` struct (lines 241-246) and its associated `NpcMotivation`.
Update use case methods to use `DirectorialNotes` directly from domain.

**Step 5: Create DirectorialNotes <-> DirectorialContext Converters**

File: `crates/engine-adapters/src/infrastructure/websocket/directorial_converters.rs`

```rust
//! Converters between domain DirectorialNotes and protocol DirectorialContext

use wrldbldr_domain::value_objects::{DirectorialNotes, ToneGuidance, PacingGuidance, NpcMotivation};
use wrldbldr_protocol::{DirectorialContext, NpcMotivationData};

impl From<DirectorialContext> for DirectorialNotes {
    fn from(ctx: DirectorialContext) -> Self {
        DirectorialNotes {
            general_notes: ctx.scene_notes,
            tone: parse_tone(&ctx.tone),
            pacing: PacingGuidance::default(),  // Protocol doesn't have pacing
            npc_motivations: ctx.npc_motivations.into_iter()
                .map(|m| (m.character_id.clone(), NpcMotivation {
                    current_mood: m.emotional_guidance,
                    immediate_goal: m.immediate_goal,
                    secret_agenda: m.secret_agenda,
                    attitude_to_players: String::new(),
                    speech_patterns: vec![],
                }))
                .collect(),
            forbidden_topics: ctx.forbidden_topics,
            allowed_tools: vec![],
            suggested_beats: vec![],
        }
    }
}

impl From<DirectorialNotes> for DirectorialContext {
    fn from(notes: DirectorialNotes) -> Self {
        DirectorialContext {
            scene_notes: notes.general_notes,
            tone: notes.tone.description().to_string(),
            npc_motivations: notes.npc_motivations.into_iter()
                .map(|(id, m)| NpcMotivationData {
                    character_id: id,
                    emotional_guidance: m.current_mood,
                    immediate_goal: m.immediate_goal,
                    secret_agenda: m.secret_agenda,
                })
                .collect(),
            forbidden_topics: notes.forbidden_topics,
        }
    }
}

fn parse_tone(s: &str) -> ToneGuidance {
    match s.to_lowercase().as_str() {
        "neutral" => ToneGuidance::Neutral,
        "serious" => ToneGuidance::Serious,
        "lighthearted" => ToneGuidance::Lighthearted,
        "tense" => ToneGuidance::Tense,
        "mysterious" => ToneGuidance::Mysterious,
        "exciting" => ToneGuidance::Exciting,
        "contemplative" => ToneGuidance::Contemplative,
        "creepy" => ToneGuidance::Creepy,
        "romantic" => ToneGuidance::Romantic,
        "comedic" => ToneGuidance::Comedic,
        _ => ToneGuidance::Custom(s.to_string()),
    }
}
```

**Step 6: Wire DirectorialNotes into LLM Flow**

File: `crates/engine-adapters/src/infrastructure/websocket_helpers.rs`

Update `build_prompt_from_action()` to:
1. Fetch `DirectorialNotes` from `WorldStateManager` 
2. Pass it to `generate_npc_response_with_direction()` instead of `None`

This closes the gap where the rich domain type was created but never used.

**Step 7: Fix WorldExporterPort import**

File: `crates/engine-ports/src/outbound/world_exporter_port.rs`

Change:
```rust
// FROM:
use wrldbldr_protocol::RuleSystemConfig;

// TO:
use wrldbldr_domain::value_objects::RuleSystemConfig;
```

This is a one-line change - the type is already in domain, protocol just re-exports it.

#### Files to Modify

| File | Action |
|------|--------|
| `crates/engine-ports/src/outbound/world_connection_port.rs` | DELETE |
| `crates/engine-ports/src/outbound/mod.rs` | Remove world_connection_port export |
| `crates/domain/src/value_objects/directorial.rs` | Add Serialize/Deserialize to existing types |
| `crates/engine-ports/src/outbound/directorial_context_port.rs` | Update to use DirectorialNotes |
| `crates/engine-app/src/application/use_cases/scene.rs` | Remove DirectorialContextData, use DirectorialNotes |
| `crates/engine-adapters/src/infrastructure/websocket/directorial_converters.rs` | Create converters |
| `crates/engine-adapters/src/infrastructure/websocket_helpers.rs` | Wire DirectorialNotes into LLM flow |
| `crates/engine-adapters/src/infrastructure/ports/scene_adapters.rs` | Update adapters |
| `crates/engine-ports/src/outbound/world_exporter_port.rs` | Update import |
| Any files using WorldConnectionPort | Update to use BroadcastPort |

---

### Phase E5: Split AppRequestHandler

**Priority**: Medium (Maintainability)  

#### Problem

`AppRequestHandler` is 3,252 lines with a single `match` handling 114 request variants.

#### Validation Notes (Sub-Agent Analysis 2025-12-28)

**Actual counts:**
- **Total lines:** 3,252
- **Total RequestPayload variants:** 114 (not ~97)
- **ID parsing helpers:** 18 functions (lines 174-270)
- **Converter functions:** 7 functions (lines 276-372)

**Cross-domain operations identified:**
1. `CreatePlayerCharacter` (1957-2034) - spans Region, Location, PlayerCharacter
2. `CreateChallenge`/`UpdateChallenge` (1503-1582) - spans Challenge, Skill
3. AI Suggestion operations (2817-3037) - spans Character, Want, AI/Suggestion
4. `GetCharacterInventory` (613) - spans PlayerCharacter, Item
5. `CreateObservation` (2403) - spans PC, Character, Location, Region

**Variant breakdown by domain:**
| Domain | Variants | Line Range |
|--------|----------|------------|
| World/Act/GameTime | 11 | 388-492, 1276-1311, 2782-2810 |
| Character/NPC | 7 | 493-635 |
| Location | 8 | 637-788 |
| Region | 13 | 790-1071 |
| Skill | 5 | 1073-1168 |
| Scene | 5 | 1170-1273 |
| Interaction | 6 | 1313-1430 |
| Challenge | 7 | 1432-1582 |
| NarrativeEvent | 9 | 1584-1721 |
| EventChain | 12 | 1723-1911 |
| PlayerCharacter | 7 | 1913-2100 |
| Relationship | 3 | 2102-2156 |
| Actantial | 3 | 2158-2210 |
| NPC Disposition | 3 | 2212-2270 |
| StoryEvent | 5 | 2272-2370 |
| Observation | 3 | 2372-2481 |
| CharacterRegion | 5 | 2483-2578 |
| Goal | 5 | 2580-2658 |
| Want | 7 | 2660-2780 |
| AI Suggestion | 4 | 2812-3037 |
| Generation Queue | 2 | 3039-3101 |
| Content Suggestion | 2 | 3103-3173 |
| Item Placement | 2 | 3175-3232 |

#### Solution

Split into 8 domain-specific handlers with a routing layer. Extract shared helpers.

#### Implementation

**Step 1: Extract common helpers**

File: `crates/engine-app/src/application/handlers/common.rs`

Move the ID parsing functions (18 total):
- `parse_uuid`, `parse_world_id`, `parse_character_id`, `parse_location_id`
- `parse_skill_id`, `parse_scene_id`, `parse_act_id`, `parse_challenge_id`
- `parse_narrative_event_id`, `parse_event_chain_id`, `parse_player_character_id`
- `parse_interaction_id`, `parse_goal_id`, `parse_want_id`, `parse_region_id`
- `parse_relationship_id`, `parse_story_event_id`, `parse_item_id`

Move converter functions (7 total):
- `parse_difficulty`, `parse_disposition_level`, `parse_relationship_level`
- `convert_actor_type`, `convert_actantial_role`, `convert_want_target_type`, `convert_want_visibility`

**Step 2: Create domain-specific handlers**

```
crates/engine-app/src/application/handlers/
├── mod.rs                         # Router + exports
├── common.rs                      # Shared ID parsers + converters
├── request_handler.rs             # Trait definition (unchanged)
├── world_handler.rs               # World, Act, GameTime (11 variants)
├── character_handler.rs           # Character, NPC, Archetype (7 variants)
├── player_character_handler.rs    # PC, Observation, Disposition (13 variants)
├── location_region_handler.rs     # Location, Region, Connections (21 variants)
├── narrative_handler.rs           # NarrativeEvent, EventChain, StoryEvent (26 variants)
├── challenge_interaction_handler.rs # Challenge, Interaction, Skill, Scene (23 variants)
├── actantial_handler.rs           # Actantial, Relationship, Goal, Want (18 variants)
└── ai_generation_handler.rs       # Suggestions, Generation, Content (8 variants)
```

**Handler responsibilities:**

| Handler | Domains | Variants | Notes |
|---------|---------|----------|-------|
| WorldHandler | World, Act, GameTime, SheetTemplate | 11 | World-level concerns |
| CharacterHandler | Character, NPC, Archetype | 7 | Core NPC/Character CRUD |
| PlayerCharacterHandler | PC, Observation, NPC Dispositions | 13 | Player-facing state |
| LocationRegionHandler | Location, Region, Connections, Exits, SpawnPoints | 21 | Spatial navigation |
| NarrativeHandler | NarrativeEvent, EventChain, StoryEvent | 26 | Story/narrative systems |
| ChallengeInteractionHandler | Challenge, Interaction, Skill, Scene | 23 | Gameplay mechanics |
| ActantialHandler | Actantial, Relationship, Goal, Want | 18 | NPC motivation systems |
| AIGenerationHandler | Suggestions, Generation, Content | 8 | AI/LLM operations |

**Step 3: Handle cross-domain operations**

For cross-domain operations, inject required services:
- `CreatePlayerCharacter` → PlayerCharacterHandler with `RegionRepositoryPort` injected
- `CreateChallenge` skill linking → ChallengeInteractionHandler (keep both services)
- AI suggestions → AIGenerationHandler with `CharacterService` injected
- `CreateObservation` → PlayerCharacterHandler with Location/Region repos

**Step 4: Create RequestRouter**

The router implements `RequestHandler` and dispatches to domain-specific handlers based on the `RequestPayload` variant category.

```rust
pub struct RequestRouter {
    world: WorldHandler,
    character: CharacterHandler,
    player_character: PlayerCharacterHandler,
    location_region: LocationRegionHandler,
    narrative: NarrativeHandler,
    challenge_interaction: ChallengeInteractionHandler,
    actantial: ActantialHandler,
    ai_generation: AIGenerationHandler,
}

#[async_trait]
impl RequestHandler for RequestRouter {
    async fn handle(&self, ctx: UseCaseContext, payload: RequestPayload) -> ResponseResult {
        match &payload {
            // World domain
            RequestPayload::ListWorlds { .. } |
            RequestPayload::GetWorld { .. } |
            // ... 
            => self.world.handle(ctx, payload).await,
            
            // Character domain
            RequestPayload::ListCharacters { .. } |
            // ...
            => self.character.handle(ctx, payload).await,
            
            // etc.
        }
    }
}
```

#### Files to Modify

| File | Action |
|------|--------|
| `crates/engine-app/src/application/handlers/common.rs` | Create (extract 25 helper functions) |
| `crates/engine-app/src/application/handlers/mod.rs` | Create router |
| `crates/engine-app/src/application/handlers/world_handler.rs` | Create (11 variants) |
| `crates/engine-app/src/application/handlers/character_handler.rs` | Create (7 variants) |
| `crates/engine-app/src/application/handlers/player_character_handler.rs` | Create (13 variants) |
| `crates/engine-app/src/application/handlers/location_region_handler.rs` | Create (21 variants) |
| `crates/engine-app/src/application/handlers/narrative_handler.rs` | Create (26 variants) |
| `crates/engine-app/src/application/handlers/challenge_interaction_handler.rs` | Create (23 variants) |
| `crates/engine-app/src/application/handlers/actantial_handler.rs` | Create (18 variants) |
| `crates/engine-app/src/application/handlers/ai_generation_handler.rs` | Create (8 variants) |
| `crates/engine-app/src/application/handlers/request_handler.rs` | Keep trait + delete 3,000 lines of impl |

---

### Phase E6: Extend arch-check

**Priority**: High  

#### Problem

Current arch-check only validates `use_cases/` directory. It doesn't check:
- `services/` directory (46 files!)
- `dto/` directory
- `handlers/` directory

#### Validation Notes

- xtask uses `regex-lite`, not `regex`
- xtask has a custom `walkdir_rs_files()` helper, not the `walkdir` crate
- Current exemptions: `mod.rs`, `errors.rs` in use_cases

#### Solution

Extend arch-check to cover all engine-app directories with proper exemptions.

#### Implementation

File: `crates/xtask/src/main.rs`

```rust
fn check_engine_app_protocol_isolation() -> anyhow::Result<()> {
    let workspace_root = workspace_root()?;
    let engine_app_src = workspace_root.join("crates/engine-app/src");
    
    // Directories to check
    let dirs_to_check = [
        engine_app_src.join("application/use_cases"),
        engine_app_src.join("application/services"),
        engine_app_src.join("application/dto"),
        engine_app_src.join("application/handlers"),
    ];
    
    // Files explicitly allowed to use protocol types
    let exempt_files: std::collections::HashSet<&str> = [
        "mod.rs",  // Module declarations only
    ].into_iter().collect();
    
    // Forbidden patterns
    let forbidden_patterns = [
        r"use\s+wrldbldr_protocol::",
        r"wrldbldr_protocol::",  // Catches FQN usage too
    ];
    
    let mut violations = Vec::new();
    
    for dir in dirs_to_check {
        if !dir.exists() { continue; }
        
        for path in walkdir_rs_files(&dir)? {
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            
            if exempt_files.contains(file_name) { continue; }
            
            let content = std::fs::read_to_string(&path)?;
            
            for (line_num, line) in content.lines().enumerate() {
                // Skip comments
                let trimmed = line.trim();
                if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with("*") {
                    continue;
                }
                
                for pattern in &forbidden_patterns {
                    if regex_lite::Regex::new(pattern)?.is_match(line) {
                        violations.push(format!(
                            "{}:{}: {}",
                            path.display(),
                            line_num + 1,
                            trimmed
                        ));
                    }
                }
            }
        }
    }
    
    if !violations.is_empty() {
        anyhow::bail!(
            "Protocol imports found in engine-app ({} violations):\n{}",
            violations.len(),
            violations.join("\n")
        );
    }
    
    Ok(())
}

fn check_engine_ports_protocol_isolation() -> anyhow::Result<()> {
    let workspace_root = workspace_root()?;
    let engine_ports_src = workspace_root.join("crates/engine-ports/src");
    
    // Documented exemption: RequestHandler uses protocol types (see plan)
    let exempt_files: std::collections::HashSet<&str> = [
        "request_handler.rs",  // Documented exemption - API boundary
    ].into_iter().collect();
    
    // ... similar logic as above
}
```

Add these checks to the main `arch_check()` function.

#### Files to Modify

| File | Action |
|------|--------|
| `crates/xtask/src/main.rs` | Add new check functions |

---

## Part 2: Player-Side Cleanup

### Phase P1: Create Player Domain Types

**Priority**: High  

#### Problem

48 files across player-app, player-ports, and player-ui import protocol types directly (17 in player-app + 2 in player-ports + 29 in player-ui).

#### Validation Notes

- `crates/player-app/src/application/dto/` exists with 6 files, some already have player-local types
- `world_snapshot.rs` has `pub use wrldbldr_protocol::{...}` re-exports that violate hexagonal
- 65 `ServerMessage` variants need handling
- 10 protocol types used in `GameConnectionPort`

#### Solution

Create player-owned DTOs that mirror the needed protocol types.

#### Implementation

**Step 1: Remove protocol re-exports from world_snapshot.rs**

File: `crates/player-app/src/application/dto/world_snapshot.rs`

Remove:
```rust
pub use wrldbldr_protocol::{
    DiceSystem, RuleSystemConfig, RuleSystemType, RuleSystemVariant, StatDefinition,
    SuccessComparison,
};
```

Either create local equivalents or import from domain (since these types exist in domain).

**Step 2: Create player DTOs**

Create/update these files:

| File | Content |
|------|---------|
| `crates/player-app/src/application/dto/connection.rs` | `ParticipantRole`, `ConnectionState`, `ConnectedUser` |
| `crates/player-app/src/application/dto/game_state.rs` | `GameTime`, `NavigationData`, `NavigationExit` |
| `crates/player-app/src/application/dto/scene.rs` | `SceneData`, `CharacterData`, `CharacterPosition` |
| `crates/player-app/src/application/dto/approval.rs` | `ApprovalDecision`, `SuggestionInfo` types |

**Step 3: Create converters in player-adapters**

File: `crates/player-adapters/src/infrastructure/protocol_mapper.rs`

Implement `From` conversions for all player DTO <-> protocol type pairs.

---

### Phase P2: Refactor GameConnectionPort

**Priority**: Critical  

#### Problem

`GameConnectionPort` trait uses 10 protocol types in method signatures.

#### Validation Notes

- 10 protocol types used (not 11 as previously stated)
- 3 implementations: Desktop, WASM, Mock
- Many methods already use primitives - only some need updating

#### Solution

Replace protocol types with player DTOs or primitives.

---

### Phase P3: Create Message Translation Layer

**Priority**: High  

#### Problem

`session_message_handler.rs` handles 65 `ServerMessage` variants directly.

#### Validation Notes

- 65 variants, not 28 as stated in original plan
- Handler is 1,296 lines
- Constructs protocol types at lines 1139-1165 (violation)

#### Solution

Create `PlayerEvent` enum (~25-30 grouped events) and `MessageTranslator` in adapters.

---

### Phase P4: Clean Up player-app Services

**Priority**: Medium  

#### Problem

14 service files use `request()` with `RequestPayload`.

---

### Phase P5: Clean Up player-ui Components

**Priority**: Medium  

#### Problem

~30 UI component files import protocol types.

---

### Phase P6: Add Player-Side arch-check

**Priority**: High  

#### Problem

No arch-check validation for player-side protocol isolation.

---

## Verification

After completing all phases, run:

```bash
# Build check
cargo check --workspace

# Clippy
cargo clippy --workspace --all-targets

# Architecture check
cargo run -p xtask -- arch-check

# Verify no protocol imports in app layers (should return empty except documented exemptions)
grep -r "use wrldbldr_protocol::" crates/engine-app/src/ | grep -v "// "
grep -r "use wrldbldr_protocol::" crates/player-app/src/ | grep -v "// "
grep -r "use wrldbldr_protocol::" crates/engine-ports/src/ | grep -v request_handler
grep -r "use wrldbldr_protocol::" crates/player-ports/src/
```

---

## Implementation Order

### Recommended Order (Engine First)

1. **Pre-Implementation Cleanup** - Delete duplicate files, remove re-exports
2. **Phase E6** - Extend arch-check (validates subsequent phases)
3. **Phase E1** - Create DomainEvent (largest structural change)
4. **Phase E2** - Create Approval DTOs
5. **Phase E3** - Move ErrorCode::into_server_error()
6. **Phase E4** - Clean Up Ports Layer
7. **Phase E5** - Split AppRequestHandler

Then player-side:

8. **Phase P1** - Create Player Domain Types
9. **Phase P2** - Refactor GameConnectionPort
10. **Phase P3** - Create Message Translation Layer
11. **Phase P4** - Clean Up player-app Services
12. **Phase P5** - Clean Up player-ui Components
13. **Phase P6** - Add Player-Side arch-check

### Batch Verification Points

- After Phase E5: Verify engine-side clean
- After Phase P6: Verify full codebase clean

---

## Appendix: Count Corrections from Validation

| Item | Original | Final Validated |
|------|----------|-----------------|
| ErrorCode call sites | 28 across 8 files | 26 across 7 files |
| RequestPayload variants | "100+" | **134** (2025-12-28 validation) |
| AppRequestHandler lines | 3,252 | 3,251 |
| GameConnectionPort protocol types | 11 | 10 |
| ServerMessage variants to handle | 28 | 65 |
| Player-app services with protocol | 16 | 14 |
| Player-app files with protocol | 18 | 17 |
| Player-ports files with protocol | 2 | 2 |
| Player-ui files with protocol | 28 | 29 |
| **Total player files** | 49 | **48** (17+2+29) |

## Appendix: DirectorialContext System Analysis

Three types currently exist with overlapping responsibilities:

| Type | Location | Purpose |
|------|----------|---------|
| `DirectorialContext` | protocol/src/messages.rs | Wire format for WebSocket |
| `DirectorialNotes` | domain/src/value_objects/directorial.rs | Rich domain type with enums |
| `DirectorialContextData` | engine-app/use_cases/scene.rs | App-layer DTO |

**Decision:** Promote `DirectorialNotes` as canonical domain type. See Phase E4 for implementation details.
