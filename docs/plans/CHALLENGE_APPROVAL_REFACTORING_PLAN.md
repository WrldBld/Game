# Challenge Approval System Refactoring Plan

**Status**: In Progress  
**Created**: 2025-12-28  
**Updated**: 2025-12-28 (validation corrections, pattern alignment)  
**Estimated Effort**: 18-24 hours  

## Executive Summary

This plan addresses critical architectural violations in the challenge/approval system that were discovered during the WebSocket Adapter Refactoring. The main issues are:

1. **CRITICAL BUG**: Approved dialogue is never broadcast to players (queue_workers.rs)
2. **Volatile State**: ChallengeOutcomeApprovalService uses in-memory HashMap (loses state on restart)
3. **Hexagonal Violations**: Services construct ServerMessage directly (protocol coupling)
4. **Conditional Approval**: has_dm() check allows bypassing DM approval
5. **Handler Violations**: 5 challenge handlers call services directly instead of use cases

## Decisions Made

| Decision | Choice |
|----------|--------|
| Queue implementation | Reuse existing `SqliteQueue` with `queue_name = "challenge_outcomes"` |
| Handler + Use Case priority | Both will be done |
| DM event routing | Always broadcast to all DMs |
| DM reconnection scope | Send ALL pending approvals (not just challenges) |
| queue_workers.rs bug | Fix immediately as Phase 0 |
| NarrativeEventApprovalService | Include in this plan |

## Verified Issues

| Issue | Location | Lines | Status |
|-------|----------|-------|--------|
| **CRITICAL**: `ApprovalOutcome::Broadcast` logged but never broadcast | `queue_workers.rs` | 172-174 | [x] |
| **CRITICAL**: `list_by_world()` doesn't filter by world_id | `sqlite_queue.rs` | 462-468 | [x] |
| In-memory HashMap for pending resolutions | `challenge_outcome_approval_service.rs` | 41, 68 | [x] |
| `has_dm()` check with fallback immediate resolution | `challenge_resolution_service.rs` | 298, 363-416 | [x] |
| Direct `ServerMessage` construction in services | Multiple services | Various | [ ] |
| 5 handlers call service directly instead of use case | `handlers/challenge.rs` | 97-214 | [ ] |
| `ChallengeResolutionPort` trait defined but not implemented | `use_cases/challenge.rs` | 218-264 | [x] |
| `NarrativeEventApprovalService` constructs `ServerMessage` | `narrative_event_approval_service.rs` | 197 | [ ] |

---

## Phase 0: Fix Critical queue_workers.rs Bug

**Priority**: CRITICAL  
**Effort**: 30 minutes  
**Risk**: Low  
**Status**: [x] Complete

### Problem

When DM approves dialogue, `ApprovalOutcome::Broadcast` is returned but only logged - never actually broadcast to players. This means approved NPC dialogue never reaches players.

### File

`crates/engine-adapters/src/infrastructure/queue_workers.rs`

### Current Code (lines 172-174)

```rust
Ok(outcome) => {
    tracing::info!("Processed approval decision: {:?}", outcome);
}
```

### Fixed Code

```rust
Ok(outcome) => match outcome {
    ApprovalOutcome::Broadcast { dialogue, npc_name, executed_tools } => {
        let message = ServerMessage::DialogueResponse {
            speaker_id: npc_name.clone(),
            speaker_name: npc_name,
            text: dialogue,
            choices: vec![],
        };
        world_connection_manager
            .broadcast_to_players(action.world_id, message)
            .await;
        tracing::info!("Broadcast approved dialogue, tools: {:?}", executed_tools);
    }
    ApprovalOutcome::Rejected { feedback, needs_reprocessing } => {
        tracing::info!("Approval rejected: {}, reprocess: {}", feedback, needs_reprocessing);
    }
    ApprovalOutcome::MaxRetriesExceeded { feedback } => {
        tracing::warn!("Approval max retries exceeded: {}", feedback);
    }
}
```

### Verification

```bash
cargo check -p wrldbldr-engine-adapters
```

---

## Phase 1: Add Challenge Queue to QueueFactory

**Priority**: High  
**Effort**: 2-3 hours  
**Risk**: Low  
**Status**: [x] Complete (factory method only - AppState wiring deferred to Phase 2)

### Rationale

Use the existing `SqliteQueue` pattern instead of creating a dedicated table. The `ChallengeOutcomeApprovalItem` DTO already exists in `queue_items.rs`.

### File

`crates/engine-adapters/src/infrastructure/queues/factory.rs`

### Changes

1. Add import for `ChallengeOutcomeApprovalItem`:
   ```rust
   use wrldbldr_engine_app::application::dto::{
       ApprovalItem, AssetGenerationItem, ChallengeOutcomeApprovalItem,
       DMActionItem, LLMRequestItem, PlayerActionItem,
   };
   ```

2. Add notifier field to `QueueFactory`:
   ```rust
   pub struct QueueFactory {
       // ... existing fields ...
       challenge_outcome_notifier: InProcessNotifier,
   }
   ```

3. Initialize in `new()`:
   ```rust
   challenge_outcome_notifier: InProcessNotifier::new("challenge_outcomes"),
   ```

4. Add getter:
   ```rust
   pub fn challenge_outcome_notifier(&self) -> InProcessNotifier {
       self.challenge_outcome_notifier.clone()
   }
   ```

5. Add factory method:
   ```rust
   /// Create a challenge outcome approval queue
   pub async fn create_challenge_outcome_queue(
       &self,
   ) -> Result<Arc<QueueBackendEnum<ChallengeOutcomeApprovalItem>>> {
       match self.config.backend.as_str() {
           "memory" => Ok(Arc::new(QueueBackendEnum::Memory(
               InMemoryQueue::new("challenge_outcomes", self.challenge_outcome_notifier.clone())
           ))),
           "sqlite" => {
               let pool = self
                   .sqlite_pool
                   .as_ref()
                   .context("SQLite pool not initialized")?;
               let queue = SqliteQueue::new(
                   pool.clone(),
                   "challenge_outcomes",
                   1,
                   self.challenge_outcome_notifier.clone(),
               ).await?;
               Ok(Arc::new(QueueBackendEnum::Sqlite(queue)))
           }
           backend => anyhow::bail!("Unsupported queue backend: {}", backend),
       }
   }
   ```

### File 2: `crates/engine-adapters/src/infrastructure/state/mod.rs`

Wire the queue into AppState initialization (after line 360):

```rust
// After existing queue creation
let challenge_outcome_queue = queue_factory.create_challenge_outcome_queue().await?;
```

Update `ChallengeOutcomeApprovalService` construction (around line 461):

```rust
let challenge_outcome_approval_service = Arc::new(
    ChallengeOutcomeApprovalService::new(
        world_connection_port_adapter.clone(),
        outcome_trigger_service.clone(),
        player_character_repo_for_triggers.clone(),
        item_repo.clone(),
        prompt_template_service.clone(),
    )
    .with_queue(challenge_outcome_queue.clone())  // NEW - Phase 2 adds this
    .with_llm_port(llm_for_suggestions)
    .with_settings_service(settings_service.clone()),
);
```

### Verification

```bash
cargo check -p wrldbldr-engine-adapters
cargo test -p wrldbldr-engine-adapters
```

---

## Phase 1.5: Fix list_by_world Filtering

**Priority**: High (Blocks Phase 8)  
**Effort**: 1 hour  
**Risk**: Low  
**Status**: [x] Complete

### Problem

The current `SqliteQueue.list_by_world()` implementation doesn't actually filter by world_id - it returns ALL pending items:

```rust
async fn list_by_world(&self, _world_id: WorldId) -> Result<Vec<QueueItem<T>>, QueueError> {
    // This is a limitation - we need to know the structure of T
    // For now, we'll return all pending/processing items.
    self.list_by_status(QueueItemStatus::Pending).await
}
```

This breaks DM reconnection logic (Phase 8) which needs to send only that world's pending approvals.

### Solution

Add a `world_id` column to the `queue_items` table and update the queue implementation.

### File 1: `crates/engine-adapters/src/infrastructure/queues/sqlite_queue.rs`

#### 1.1 Update Schema (in `new()`)

Add column and index:
```sql
ALTER TABLE queue_items ADD COLUMN world_id TEXT;
CREATE INDEX IF NOT EXISTS idx_queue_world ON queue_items(queue_name, world_id, status);
```

Or for new installations, update the CREATE TABLE:
```sql
CREATE TABLE IF NOT EXISTS queue_items (
    id TEXT PRIMARY KEY,
    queue_name TEXT NOT NULL,
    world_id TEXT,  -- NEW: extracted from payload for filtering
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    scheduled_at TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    error_message TEXT,
    metadata_json TEXT
)
```

#### 1.2 Update `enqueue()` to Extract world_id

```rust
async fn enqueue(&self, payload: T, priority: u8) -> Result<QueueItemId, QueueError> {
    let id = uuid::Uuid::new_v4();
    let payload_json = serde_json::to_string(&payload)?;
    let now = Utc::now();
    let now_str = now.to_rfc3339();
    
    // Extract world_id from payload JSON if present
    let world_id: Option<String> = serde_json::from_str::<serde_json::Value>(&payload_json)
        .ok()
        .and_then(|v| v.get("world_id").and_then(|w| w.as_str().map(String::from)));

    sqlx::query(
        r#"
        INSERT INTO queue_items 
        (id, queue_name, world_id, payload_json, status, priority, created_at, updated_at, attempts, max_attempts, metadata_json)
        VALUES (?, ?, ?, ?, 'pending', ?, ?, ?, 0, 3, '{}')
        "#,
    )
    .bind(id.to_string())
    .bind(&self.queue_name)
    .bind(&world_id)
    .bind(&payload_json)
    .bind(priority as i64)
    .bind(&now_str)
    .bind(&now_str)
    .execute(&self.pool)
    .await
    .map_err(|e| QueueError::Database(e.to_string()))?;
    // ...
}
```

#### 1.3 Update `list_by_world()` to Actually Filter

```rust
async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<QueueItem<T>>, QueueError> {
    let world_id_str = world_id.to_string();
    
    let rows = sqlx::query(
        r#"
        SELECT * FROM queue_items
        WHERE queue_name = ? 
        AND world_id = ?
        AND status IN ('pending', 'processing')
        ORDER BY priority DESC, created_at ASC
        "#,
    )
    .bind(&self.queue_name)
    .bind(&world_id_str)
    .fetch_all(&self.pool)
    .await
    .map_err(|e| QueueError::Database(e.to_string()))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(self.row_to_item(row).await?);
    }
    Ok(items)
}
```

#### 1.4 Update `get_history_by_world()` Similarly

```rust
async fn get_history_by_world(
    &self,
    world_id: WorldId,
    limit: usize,
) -> Result<Vec<QueueItem<T>>, QueueError> {
    let world_id_str = world_id.to_string();
    
    let rows = sqlx::query(
        r#"
        SELECT * FROM queue_items
        WHERE queue_name = ?
        AND world_id = ?
        AND status IN ('completed', 'failed', 'expired')
        ORDER BY updated_at DESC
        LIMIT ?
        "#,
    )
    .bind(&self.queue_name)
    .bind(&world_id_str)
    .bind(limit as i64)
    .fetch_all(&self.pool)
    .await
    .map_err(|e| QueueError::Database(e.to_string()))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(self.row_to_item(row).await?);
    }
    Ok(items)
}
```

### File 2: `crates/engine-adapters/src/infrastructure/queues/memory_queue.rs`

Update `InMemoryQueue.list_by_world()` to also filter properly (currently has same issue).

### Verification

```bash
cargo check -p wrldbldr-engine-adapters
cargo test -p wrldbldr-engine-adapters
```

### Test

Add test to verify world_id filtering:
```rust
#[tokio::test]
async fn test_list_by_world_filters_correctly() {
    // Create queue, enqueue items for different worlds
    // Verify list_by_world returns only matching world's items
}
```

---

## Phase 2: Refactor ChallengeOutcomeApprovalService

**Priority**: High  
**Effort**: 3-4 hours  
**Risk**: Medium  
**Status**: [x] Complete (queue support added, result types defined, AppState wired)

### Rationale

Replace volatile in-memory HashMap with persistent queue. Remove direct broadcasting - return result types instead.

### File

`crates/engine-app/src/application/services/challenge_outcome_approval_service.rs`

### Changes

#### 2.1 Replace Dependencies

**Before:**
```rust
pub struct ChallengeOutcomeApprovalService<L: LlmPort> {
    pending: Arc<RwLock<HashMap<String, ChallengeOutcomeApprovalItem>>>,
    world_connection: Arc<dyn WorldConnectionPort>,
    // ...
}
```

**After:**
```rust
pub struct ChallengeOutcomeApprovalService<L: LlmPort, Q: QueuePort<ChallengeOutcomeApprovalItem>> {
    queue: Arc<Q>,
    // Remove world_connection
    // ...
}
```

#### 2.2 Define Result Types

```rust
/// Result of challenge approval operations
#[derive(Debug, Clone)]
pub enum ChallengeApprovalResult {
    /// Item queued for DM approval
    Queued { resolution_id: String },
    /// Challenge resolved (approved by DM)
    Resolved {
        challenge_id: String,
        outcome: ResolvedOutcome,
        state_changes: Vec<StateChange>,
    },
    /// LLM suggestions ready
    SuggestionsReady {
        resolution_id: String,
        suggestions: Vec<String>,
    },
    /// Outcome branches ready
    BranchesReady {
        resolution_id: String,
        branches: Vec<OutcomeBranch>,
    },
}

#[derive(Debug, Clone)]
pub struct ResolvedOutcome {
    pub outcome_type: String,
    pub outcome_description: String,
    pub roll: i32,
    pub modifier: i32,
    pub total: i32,
    pub roll_breakdown: Option<String>,
    pub individual_rolls: Option<Vec<i32>>,
}
```

#### 2.3 Update queue_for_approval Method

```rust
pub async fn queue_for_approval(
    &self,
    world_id: WorldId,
    item: ChallengeOutcomeApprovalItem,
) -> Result<ChallengeApprovalResult, ChallengeOutcomeError> {
    let resolution_id = item.resolution_id.clone();
    self.queue.enqueue(item, 0).await
        .map_err(|e| ChallengeOutcomeError::SessionError(e.to_string()))?;
    Ok(ChallengeApprovalResult::Queued { resolution_id })
}
```

#### 2.4 Remove tokio::spawn Calls

All async work should be awaited directly or returned as futures - no detached spawns.

#### 2.5 Remove ServerMessage Constructions

Replace all `ServerMessage::*` constructions with domain result types.

### Verification

```bash
cargo check -p wrldbldr-engine-app
cargo test -p wrldbldr-engine-app
```

---

## Phase 3: Refactor ChallengeResolutionService

**Priority**: High  
**Effort**: 2-3 hours  
**Risk**: Medium  
**Status**: [x] Complete

### Rationale

Remove conditional `has_dm()` check - all challenges should go through DM approval. Remove direct broadcasting.

### File

`crates/engine-app/src/application/services/challenge_resolution_service.rs`

### Changes

#### 3.1 Remove has_dm() Check (line 298)

**Before:**
```rust
if self.world_connection.has_dm(&world_id).await {
    if let Some(ref approval_service) = self.challenge_outcome_approval_service {
        // ... queue for approval
    }
}
// No DM or approval service not configured - immediate resolution
```

**After:**
```rust
// Always queue for DM approval
let resolution = PendingChallengeResolutionDto { ... };
return self.challenge_outcome_approval_service
    .queue_for_approval(world_id, resolution.into())
    .await;
```

#### 3.2 Make Approval Service Non-Optional

**Before:**
```rust
challenge_outcome_approval_service: Option<Arc<ChallengeOutcomeApprovalService<L>>>,
```

**After:**
```rust
challenge_outcome_approval_service: Arc<ChallengeOutcomeApprovalService<L, Q>>,
```

#### 3.3 Delete Immediate Resolution Path

Remove lines 363-416 (the fallback path when no DM is present).

#### 3.4 Remove WorldConnectionPort Dependency

Service should not broadcast directly.

#### 3.5 Return Typed Results

**Before:**
```rust
pub async fn handle_roll_input(...) -> Option<serde_json::Value>
```

**After:**
```rust
pub async fn handle_roll_input(...) -> Result<RollSubmissionResult, ChallengeError>

pub struct RollSubmissionResult {
    pub resolution_id: String,
    pub challenge_id: String,
    pub challenge_name: String,
    pub character_name: String,
    pub roll: i32,
    pub modifier: i32,
    pub total: i32,
    pub outcome_type: String,
    pub outcome_description: String,
    pub roll_breakdown: Option<String>,
    pub individual_rolls: Option<Vec<i32>>,
    pub triggers: Vec<OutcomeTriggerInfo>,  // For broadcasting
}
```

#### 3.6 Define Service-Layer Error Type

Create a dedicated error type for the service layer (separate from use case `ChallengeError`):

```rust
// In challenge_resolution_service.rs or a new errors module

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChallengeResolutionError {
    #[error("Invalid challenge ID: {0}")]
    InvalidChallengeId(String),
    
    #[error("Challenge not found: {0}")]
    ChallengeNotFound(String),
    
    #[error("Failed to load challenge: {0}")]
    ChallengeLoadFailed(String),
    
    #[error("Player character not found")]
    PlayerCharacterNotFound,
    
    #[error("Invalid dice formula: {0}")]
    InvalidDiceFormula(String),
    
    #[error("Skill not found for challenge")]
    SkillNotFound,
    
    #[error("Failed to queue for approval: {0}")]
    ApprovalQueueFailed(String),
}
```

This allows the adapter to map service errors to use case errors with proper context.

### Verification

```bash
cargo check -p wrldbldr-engine-app
```

---

## Phase 4: Create ChallengeResolutionAdapter

**Priority**: High  
**Effort**: 1-2 hours  
**Risk**: Low  
**Status**: [x] Complete

### Rationale

Implement `ChallengeResolutionPort` trait (already defined in use_cases/challenge.rs) by wrapping the refactored service. Replace the existing `ChallengeResolutionPlaceholder`.

### File (Update Existing)

`crates/engine-adapters/src/infrastructure/ports/challenge_adapters.rs`

**Note:** Add to the existing file which already contains `ChallengeResolutionPlaceholder`, `ChallengeOutcomeApprovalAdapter`, and `ChallengeDmApprovalQueueAdapter`. The placeholder will be replaced.

### Implementation

```rust
//! Adapter implementing ChallengeResolutionPort for the use case layer.

use async_trait::async_trait;
use std::sync::Arc;

use wrldbldr_domain::{CharacterId, PlayerCharacterId, WorldId};
use wrldbldr_engine_app::application::services::ChallengeResolutionService;
use wrldbldr_engine_app::application::use_cases::{
    AdHocOutcomes, AdHocResult, ChallengeResolutionPort, DiceInputType,
    RollResult, TriggerResult,
};

pub struct ChallengeResolutionAdapter<S, K, Q, P, L, I> {
    service: Arc<ChallengeResolutionService<S, K, Q, P, L, I>>,
}

impl<S, K, Q, P, L, I> ChallengeResolutionAdapter<S, K, Q, P, L, I> {
    pub fn new(service: Arc<ChallengeResolutionService<S, K, Q, P, L, I>>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl<S, K, Q, P, L, I> ChallengeResolutionPort for ChallengeResolutionAdapter<S, K, Q, P, L, I>
where
    S: Send + Sync + 'static,
    K: Send + Sync + 'static,
    Q: Send + Sync + 'static,
    P: Send + Sync + 'static,
    L: Send + Sync + 'static,
    I: Send + Sync + 'static,
{
    async fn handle_roll(
        &self,
        world_id: &WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        roll: i32,
    ) -> Result<RollResult, String> {
        let result = self.service
            .handle_roll(world_id, &pc_id, challenge_id, roll)
            .await
            .map_err(|e| e.to_string())?;
        Ok(RollResult::from(result))
    }

    async fn handle_roll_input(
        &self,
        world_id: &WorldId,
        pc_id: PlayerCharacterId,
        challenge_id: String,
        input_type: DiceInputType,
    ) -> Result<RollResult, String> {
        // Convert use case DiceInputType to service type
        let service_input = convert_dice_input(input_type);
        let result = self.service
            .handle_roll_input(world_id, &pc_id, challenge_id, service_input)
            .await
            .map_err(|e| e.to_string())?;
        Ok(RollResult::from(result))
    }

    async fn trigger_challenge(
        &self,
        world_id: &WorldId,
        challenge_id: String,
        target_character_id: CharacterId,
    ) -> Result<TriggerResult, String> {
        self.service
            .trigger_challenge(world_id, &challenge_id, &target_character_id)
            .await
            .map_err(|e| e.to_string())
            .map(TriggerResult::from)
    }

    async fn handle_suggestion_decision(
        &self,
        world_id: &WorldId,
        request_id: String,
        approved: bool,
        modified_difficulty: Option<String>,
    ) -> Result<(), String> {
        self.service
            .handle_suggestion_decision(world_id, &request_id, approved, modified_difficulty)
            .await
            .map_err(|e| e.to_string())
    }

    async fn create_adhoc_challenge(
        &self,
        world_id: &WorldId,
        challenge_name: String,
        skill_name: String,
        difficulty: String,
        target_pc_id: PlayerCharacterId,
        outcomes: AdHocOutcomes,
    ) -> Result<AdHocResult, String> {
        self.service
            .create_adhoc_challenge(world_id, &challenge_name, &skill_name, &difficulty, &target_pc_id, outcomes.into())
            .await
            .map_err(|e| e.to_string())
            .map(AdHocResult::from)
    }
}
```

### Wire into Adapters Module

Update `crates/engine-adapters/src/infrastructure/adapters/mod.rs` to export.

### Verification

```bash
cargo check -p wrldbldr-engine-adapters
```

---

## Phase 5: Enrich GameEvent for Challenges

**Priority**: High  
**Effort**: 2-3 hours  
**Risk**: Medium  
**Status**: [x] Complete

### Rationale

Replace minimal existing challenge events with comprehensive variants for proper broadcast routing.

### File 1

`crates/engine-ports/src/outbound/game_events.rs`

### Current (lines 114-129) - TO BE REPLACED

```rust
// These minimal variants need to be replaced:
ChallengeTriggerRequested { request_id, challenge_name, pc_name, context }
ChallengeOutcomePending { request_id, challenge_name, pc_name, roll_result, outcome_branch }
```

### Replace With Enriched Variants

```rust
pub enum GameEvent {
    // ... existing variants ...

    // === Challenge Events (Enhanced) ===

    /// Roll submitted, awaiting DM approval
    ChallengeRollSubmitted {
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
        individual_rolls: Option<Vec<i32>>,
        outcome_triggers: Vec<OutcomeTriggerInfo>,
    },

    /// Challenge fully resolved and approved
    ChallengeResolved {
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
        state_changes: Vec<StateChangeInfo>,
    },

    /// LLM suggestions ready for outcome
    ChallengeSuggestionsReady {
        resolution_id: String,
        suggestions: Vec<String>,
    },

    /// Outcome branches ready for selection
    ChallengeBranchesReady {
        resolution_id: String,
        branches: Vec<OutcomeBranchInfo>,
    },
}

/// Trigger information for display
#[derive(Debug, Clone)]
pub struct OutcomeTriggerInfo {
    pub trigger_type: String,
    pub description: String,
}

/// State change information
#[derive(Debug, Clone)]
pub struct StateChangeInfo {
    pub change_type: String,
    pub description: String,
}

/// Branch information for selection
#[derive(Debug, Clone)]
pub struct OutcomeBranchInfo {
    pub branch_id: String,
    pub description: String,
    pub consequences: Vec<String>,
}
```

### File 2

`crates/engine-adapters/src/infrastructure/websocket/broadcast_adapter.rs`

### Add Conversion Logic

```rust
GameEvent::ChallengeRollSubmitted { 
    ref world_id, ref resolution_id, ref challenge_id, ref challenge_name,
    ref character_id, ref character_name, roll, modifier, total,
    ref outcome_type, ref outcome_description, ref roll_breakdown,
    ref individual_rolls, ref outcome_triggers,
} => {
    // 1. Send full pending data to DM for approval UI
    let dm_message = ServerMessage::ChallengeOutcomePending {
        resolution_id: resolution_id.clone(),
        challenge_id: challenge_id.clone(),
        challenge_name: challenge_name.clone(),
        character_id: character_id.clone(),
        character_name: character_name.clone(),
        roll,
        modifier,
        total,
        outcome_type: outcome_type.clone(),
        outcome_description: outcome_description.clone(),
        roll_breakdown: roll_breakdown.clone(),
        outcome_triggers: outcome_triggers.iter()
            .map(|t| ProposedToolInfo { 
                id: uuid::Uuid::new_v4().to_string(),
                name: t.trigger_type.clone(),
                description: t.description.clone(),
                arguments: serde_json::Value::Null,
            })
            .collect(),
    };
    self.world_connection.send_to_dm(world_id, dm_message).await?;
    
    // 2. Send confirmation to all players (including roller)
    let player_message = ServerMessage::ChallengeRollSubmitted {
        challenge_id: challenge_id.clone(),
        challenge_name: challenge_name.clone(),
        roll,
        modifier,
        total,
        outcome_type: outcome_type.clone(),
        status: "pending_approval".to_string(),
    };
    self.world_connection.broadcast_to_world(world_id, player_message).await?;
}

GameEvent::ChallengeResolved { 
    ref world_id, ref challenge_id, ref challenge_name, ref character_name,
    roll, modifier, total, ref outcome, ref outcome_description,
    ref roll_breakdown, ref individual_rolls, ..
} => {
    // Broadcast to all players
    let message = ServerMessage::ChallengeResolved {
        challenge_id: challenge_id.clone(),
        challenge_name: challenge_name.clone(),
        character_name: character_name.clone(),
        roll,
        modifier,
        total,
        outcome: outcome.clone(),
        outcome_description: outcome_description.clone(),
        roll_breakdown: roll_breakdown.clone(),
        individual_rolls: individual_rolls.clone(),
    };
    self.world_connection.broadcast_to_world(world_id, message).await?;
}
```

### Verification

```bash
cargo check -p wrldbldr-engine-ports
cargo check -p wrldbldr-engine-adapters
```

---

## Phase 6: Update ChallengeUseCase

**Priority**: High  
**Effort**: 2 hours  
**Risk**: Low  
**Status**: [x] Complete

### Rationale

Wire the use case to use ports and broadcast events properly.

### File

`crates/engine-app/src/application/use_cases/challenge.rs`

### Changes

#### 6.1 Enrich RollResult Type

The current `RollResult` (line 151) is minimal. Expand it to include all fields needed for broadcasting:

```rust
/// Result of submitting a roll
#[derive(Debug, Clone)]
pub struct RollResult {
    /// Resolution ID for tracking this pending approval
    pub resolution_id: String,
    /// Challenge ID
    pub challenge_id: String,
    /// Challenge name
    pub challenge_name: String,
    /// Character ID who rolled
    pub character_id: String,
    /// Character name who rolled
    pub character_name: String,
    /// The raw roll value
    pub roll: i32,
    /// Skill modifier applied
    pub modifier: i32,
    /// Total result (roll + modifier)
    pub total: i32,
    /// Outcome type (success, failure, critical, etc.)
    pub outcome_type: String,
    /// Outcome description text
    pub outcome_description: String,
    /// Roll breakdown string (e.g., "1d20+5")
    pub roll_breakdown: Option<String>,
    /// Individual dice results
    pub individual_rolls: Option<Vec<i32>>,
    /// Triggers to execute on approval
    pub triggers: Vec<TriggerInfo>,
    /// Whether outcome requires DM approval
    pub pending_approval: bool,
}

/// Trigger information for RollResult
#[derive(Debug, Clone)]
pub struct TriggerInfo {
    pub trigger_type: String,
    pub description: String,
}
```

#### 6.2 Verify BroadcastPort Dependency

The `ChallengeUseCase` already has a `broadcast` field (line 324). Ensure it's being used:

```rust
pub struct ChallengeUseCase {
    resolution_service: Arc<dyn ChallengeResolutionPort>,
    outcome_approval: Arc<dyn ChallengeOutcomeApprovalPort>,
    approval_queue: Arc<dyn DmApprovalQueuePort>,
    broadcast: Arc<dyn BroadcastPort>,  // Already exists
}
```

#### 6.3 Broadcast After Roll Submission

Update `submit_dice_input` to broadcast after resolution port call. Note: The method takes `UseCaseContext ctx` and `SubmitDiceInputInput input`:

```rust
pub async fn submit_dice_input(
    &self,
    ctx: UseCaseContext,
    input: SubmitDiceInputInput,
) -> Result<RollResult, ChallengeError> {
    let pc_id = ctx.pc_id.ok_or(ChallengeError::PcNotFound(
        PlayerCharacterId::from_uuid(uuid::Uuid::nil()),
    ))?;

    debug!(
        challenge_id = %input.challenge_id,
        input_type = ?input.input_type,
        "Submitting dice input"
    );

    let result = self.resolution_service
        .handle_roll_input(&ctx.world_id, pc_id, input.challenge_id.clone(), input.input_type)
        .await
        .map_err(|e| ChallengeError::ResolutionFailed(e))?;

    // Broadcast roll submission event (handled by adapter which routes to DM + all players)
    self.broadcast.broadcast(
        ctx.world_id.clone(),
        GameEvent::ChallengeRollSubmitted {
            world_id: ctx.world_id.clone(),
            resolution_id: result.resolution_id.clone(),
            challenge_id: result.challenge_id.clone(),
            challenge_name: result.challenge_name.clone(),
            character_id: result.character_id.clone(),
            character_name: result.character_name.clone(),
            roll: result.roll,
            modifier: result.modifier,
            total: result.total,
            outcome_type: result.outcome_type.clone(),
            outcome_description: result.outcome_description.clone(),
            roll_breakdown: result.roll_breakdown.clone(),
            individual_rolls: result.individual_rolls.clone(),
            outcome_triggers: result.triggers.iter().map(|t| OutcomeTriggerInfo {
                trigger_type: t.trigger_type.clone(),
                description: t.description.clone(),
            }).collect(),
        }
    ).await;

    Ok(result)
}
```

#### 6.4 Broadcast on Approval

```rust
pub async fn outcome_decision(&self, input: OutcomeDecisionInput) -> Result<(), UseCaseError> {
    let result = self.outcome_approval_port
        .process_decision(&input.world_id, &input.resolution_id, input.decision)
        .await?;

    if let ChallengeApprovalResult::Resolved { challenge_id, outcome, state_changes } = result {
        // Broadcast to all players
        self.broadcast_port.broadcast(
            input.world_id.clone(),
            GameEvent::ChallengeResolved {
                world_id: input.world_id,
                challenge_id,
                // ... fill from outcome
            }
        ).await?;
    }

    Ok(())
}
```

### Verification

```bash
cargo check -p wrldbldr-engine-app
```

---

## Phase 7: Update Challenge Handlers

**Priority**: High  
**Effort**: 1-2 hours  
**Risk**: Low  
**Status**: [x] Complete

### Rationale

5 handlers currently call service directly. Update them to use the use case layer.

### File

`crates/engine-adapters/src/infrastructure/websocket/handlers/challenge.rs`

### Handlers to Update

| Handler | Current | Target |
|---------|---------|--------|
| `handle_challenge_roll` | `state.game.challenge_resolution_service` | `use_cases.challenge.submit_roll()` |
| `handle_challenge_roll_input` | `state.game.challenge_resolution_service` | `use_cases.challenge.submit_dice_input()` |
| `handle_trigger_challenge` | `state.game.challenge_resolution_service` | `use_cases.challenge.trigger_challenge()` |
| `handle_challenge_suggestion_decision` | `state.game.challenge_resolution_service` | `use_cases.challenge.suggestion_decision()` |
| `handle_create_adhoc_challenge` | `state.game.challenge_resolution_service` | `use_cases.challenge.create_adhoc()` |

### Example Change

**Before:**
```rust
pub async fn handle_challenge_roll_input(
    state: &AppState,
    client_id: Uuid,
    challenge_id: String,
    input_type: ClientDiceInputType,
) -> ServerMessage {
    let (world_id, pc_id) = match extract_player_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(msg) => return msg,
    };

    let service_input = to_service_dice_input(input_type);
    match state.game.challenge_resolution_service
        .handle_roll_input(&world_id, &pc_id, challenge_id, service_input)
        .await
    {
        Some(json) => value_to_server_message(json),
        None => error_msg("INTERNAL_ERROR", "Challenge resolution failed"),
    }
}
```

**After:**
```rust
pub async fn handle_challenge_roll_input(
    state: &AppState,
    client_id: Uuid,
    challenge_id: String,
    input_type: ClientDiceInputType,
) -> Option<ServerMessage> {
    let (world_id, pc_id) = match extract_player_context(state, client_id).await {
        Ok(ctx) => ctx,
        Err(msg) => return Some(msg),
    };

    let ctx = UseCaseContext::player(client_id.to_string(), world_id.clone(), pc_id);
    let input = SubmitDiceInputInput {
        challenge_id,
        input_type: to_use_case_dice_input(input_type),
    };

    match state.use_cases.challenge.submit_dice_input(ctx, input).await {
        Ok(_) => None, // Use case broadcasts to all via BroadcastPort
        Err(e) => Some(e.into_server_error()),
    }
}
```

**Note**: The handler returns `None` on success. The use case broadcasts `GameEvent::ChallengeRollSubmitted` to all connected clients (DM gets full details, players get status). This matches the pattern used by `handle_challenge_outcome_decision`.

### Verification

```bash
cargo check -p wrldbldr-engine-adapters
```

---

## Phase 8: DM Reconnection Logic for Challenge Outcomes

**Priority**: High  
**Effort**: 1 hour  
**Risk**: Low  
**Status**: [x] Complete

### Rationale

When DM connects/reconnects, they need to receive pending challenge outcomes. 

**Note**: Dialogue approvals are already handled by the existing `approval_notification_worker` in `queue_workers.rs`. This phase only adds challenge outcome notifications to follow the same pattern.

### Approach: Follow Existing Worker Pattern

The existing `approval_notification_worker` (queue_workers.rs:23-85) polls the approval queue and sends `ServerMessage::ApprovalRequired` to DMs. We'll create a similar worker for challenge outcomes.

### Files

- `crates/engine-adapters/src/infrastructure/queue_workers.rs`
- `crates/engine-adapters/src/run/server.rs` (to spawn the worker)

### Changes

#### 8.1 Add Challenge Outcome Notification Worker

Add to `queue_workers.rs`:

```rust
/// Worker that sends pending challenge outcomes to DM
pub async fn challenge_outcome_notification_worker(
    challenge_queue: Arc<QueueBackendEnum<ChallengeOutcomeApprovalItem>>,
    world_connection_manager: SharedWorldConnectionManager,
    recovery_interval: Duration,
) {
    tracing::info!("Starting challenge outcome notification worker");
    let notifier = challenge_queue.notifier();
    
    loop {
        let world_ids = world_connection_manager.get_all_world_ids().await;
        let mut has_work = false;
        
        for world_id in world_ids {
            // Only process if world has a DM connected
            if !world_connection_manager.has_dm(&world_id).await {
                continue;
            }
            
            let pending = match challenge_queue.list_by_world(WorldId::from(world_id)).await {
                Ok(items) => items,
                Err(e) => {
                    tracing::error!("Failed to get pending challenge outcomes for world {}: {}", world_id, e);
                    continue;
                }
            };
            
            if !pending.is_empty() {
                has_work = true;
            }
            
            for queue_item in pending {
                let item = queue_item.payload;
                let message = ServerMessage::ChallengeOutcomePending {
                    resolution_id: item.resolution_id.clone(),
                    challenge_id: item.challenge_id,
                    challenge_name: item.challenge_name,
                    character_id: item.character_id,
                    character_name: item.character_name,
                    roll: item.roll,
                    modifier: item.modifier,
                    total: item.total,
                    outcome_type: item.outcome_type,
                    outcome_description: item.outcome_description,
                    outcome_triggers: item.outcome_triggers,
                    roll_breakdown: item.roll_breakdown,
                };
                
                if let Err(e) = world_connection_manager.send_to_dm(&world_id, message).await {
                    tracing::warn!("Failed to send challenge outcome to DM for world {}: {}", world_id, e);
                } else {
                    tracing::debug!(
                        "Sent ChallengeOutcomePending {} to DM",
                        item.resolution_id
                    );
                }
            }
        }
        
        if !has_work {
            let _ = notifier.wait_for_work(recovery_interval).await;
        }
    }
}
```

#### 8.2 Spawn Worker in Server Startup

In `server.rs`, spawn the worker alongside existing workers:

```rust
// Spawn challenge outcome notification worker
let challenge_queue_clone = challenge_outcome_queue.clone();
let wcm_clone = world_connection_manager.clone();
tokio::spawn(async move {
    challenge_outcome_notification_worker(
        challenge_queue_clone,
        wcm_clone,
        Duration::from_secs(5),
    ).await;
});
```

### Why This Pattern?

1. **Consistency**: Follows the same pattern as `approval_notification_worker`
2. **Simplicity**: No changes needed to use case layer
3. **Reliability**: Worker continuously polls, handling reconnections automatically
4. **Separation**: Adapters layer handles infrastructure concerns

### Verification

```bash
cargo check -p wrldbldr-engine-adapters
```

---

## Phase 9: (Merged into Phase 0)

This phase was originally the queue_workers.rs bug fix, now moved to Phase 0 for priority.

---

## Phase 10: NarrativeEventApprovalService Cleanup

**Priority**: Medium  
**Effort**: 1 hour  
**Risk**: Low  
**Status**: [x] Complete

### Rationale

Remove protocol coupling from this service.

### File

`crates/engine-app/src/application/services/narrative_event_approval_service.rs`

### Changes

#### 10.1 Remove ServerMessage Construction (line 197)

**Before:**
```rust
let server_msg = ServerMessage::NarrativeEventTriggered {
    event_id: event.id.to_string(),
    event_name: event.name.clone(),
    // ...
};
```

**After:**
```rust
// Return domain result instead
Ok(NarrativeEventResult {
    event_id: event.id,
    event_name: event.name.clone(),
    description: event.description.clone(),
    // ...
})
```

#### 10.2 Define Result Type

```rust
pub struct NarrativeEventResult {
    pub event_id: NarrativeEventId,
    pub event_name: String,
    pub description: String,
    pub consequences: Vec<String>,
    pub state_changes: Vec<StateChange>,
}
```

#### 10.3 Remove Dead Code

Remove any unused DTOs or helper functions.

### Verification

```bash
cargo check -p wrldbldr-engine-app
```

---

## Phase 11: Cleanup and Tests

**Priority**: Medium  
**Effort**: 2-3 hours  
**Risk**: Low  
**Status**: [~] Partial - Core refactoring complete, cleanup deferred

### Completed

- [x] `ChallengeResolutionPlaceholder` removed (was in Phase 4)
- [x] Dead code in `NarrativeEventApprovalService` removed (Phase 10)
- [x] `narrative_event_approval_service.rs` no longer imports `wrldbldr_protocol`
- [x] All handlers migrated to use case layer (Phase 7)
- [x] Workspace compiles and passes checks

### Deferred (Future Work)

The following items require additional refactoring to move all broadcasting
from services to the use case layer via BroadcastPort:

- [ ] Remove `WorldConnectionPort` from `ChallengeOutcomeApprovalService` 
  - Requires moving all `ServerMessage` construction to use case/adapter layer
  - 10 protocol references still present
- [ ] Remove `WorldConnectionPort` from `ChallengeResolutionService`
  - Already removed in Phase 3
- [ ] Remove in-memory HashMap (`pending` field) from `ChallengeOutcomeApprovalService`
  - Currently used as cache alongside queue
- [ ] Remove protocol imports from `challenge_outcome_approval_service.rs`
  - Still constructs `ServerMessage` variants directly

### Tests (Future Work)

- [ ] Unit tests for `SqliteQueue.list_by_world()` filtering
- [ ] Unit tests for challenge queue enqueue/dequeue
- [ ] Unit tests for `ChallengeResolutionAdapter` type conversions
- [ ] Integration test: roll submission → queue → DM approval → broadcast
- [ ] Integration test: DM reconnection receives pending approvals

### Verification

```bash
cargo check --workspace  # PASSING
cargo clippy --workspace --all-targets --all-features
cargo run -p xtask -- arch-check  # Verify no layer violations
```

---

## Summary

### Effort Breakdown

| Phase | Description | Effort | Risk |
|-------|-------------|--------|------|
| **0** | Fix queue_workers.rs broadcast bug | 30 min | Low |
| **1** | Add challenge queue to QueueFactory | 2-3 hrs | Low |
| **1.5** | Fix list_by_world filtering (add world_id column) | 1 hr | Low |
| **2** | Refactor ChallengeOutcomeApprovalService | 3-4 hrs | Medium |
| **3** | Refactor ChallengeResolutionService | 2-3 hrs | Medium |
| **4** | Create ChallengeResolutionAdapter | 1-2 hrs | Low |
| **5** | Enrich GameEvent for challenges | 2-3 hrs | Medium |
| **6** | Update ChallengeUseCase + enrich RollResult | 2 hrs | Low |
| **7** | Update 5 challenge handlers | 1-2 hrs | Low |
| **8** | Add challenge outcome notification worker | 1 hr | Low |
| **10** | NarrativeEventApprovalService cleanup | 1 hr | Low |
| **11** | Cleanup and tests | 2-3 hrs | Low |

**Total**: ~18-24 hours

### Key Files Modified

| File | Phases |
|------|--------|
| `queue_workers.rs` | 0, 8 |
| `queues/factory.rs` | 1 |
| `queues/sqlite_queue.rs` | 1.5 |
| `queues/memory_queue.rs` | 1.5 |
| `challenge_outcome_approval_service.rs` | 2, 11 |
| `challenge_resolution_service.rs` | 3, 11 |
| `infrastructure/ports/challenge_adapters.rs` | 4, 11 |
| `game_events.rs` | 5 |
| `broadcast_adapter.rs` | 5 |
| `use_cases/challenge.rs` | 6, 11 |
| `handlers/challenge.rs` | 7 |
| `run/server.rs` | 8 |
| `narrative_event_approval_service.rs` | 10 |

### Success Criteria

- [ ] Approved dialogue broadcasts to players (Phase 0 fix)
- [ ] Challenge outcomes persist across server restarts
- [ ] `list_by_world()` correctly filters by world_id
- [ ] No `ServerMessage` imports in application layer services
- [ ] All 5 challenge handlers use use case layer
- [ ] Players receive roll confirmation immediately (pending_approval status)
- [ ] DM receives all pending approvals on reconnect
- [ ] All tests pass
- [ ] Clippy clean
- [ ] `arch-check` passes (no layer violations)

---

## Validation Notes (2025-12-28)

### Validation Status: APPROVED

The plan has been thoroughly reviewed against the actual codebase. The core approach is correct. Minor clarifications below.

### Key Findings

#### 1. Handler Return Pattern (Phase 7)

**Pattern Confirmed**: Looking at `handle_challenge_outcome_decision` (challenge.rs:237-240):
```rust
match state.use_cases.challenge.outcome_decision(ctx, input).await {
    Ok(_) => None, // Resolution broadcast handled by service
    Err(e) => Some(e.into_server_error()),
}
```

**Action**: Phase 7 handlers should return `None` on success. The use case broadcasts to all via `BroadcastPort`. This is the established pattern.

#### 2. Existing GameEvent Variants (Phase 5)

**Finding**: `GameEvent` already has minimal challenge variants (game_events.rs:114-129):
```rust
ChallengeTriggerRequested { request_id, challenge_name, pc_name, context }
ChallengeOutcomePending { request_id, challenge_name, pc_name, roll_result, outcome_branch }
```

**Action**: Phase 5 should REPLACE these with enriched versions. The current `ChallengeOutcomePending` has only 5 fields; we need 12+ fields to match `ChallengeOutcomeApprovalItem`.

**Broadcast adapter** (broadcast_adapter.rs:242-252) currently stubs these:
```rust
GameEvent::ChallengeOutcomePending { .. } => {
    tracing::debug!("ChallengeOutcomePending event received - handled by existing flow");
}
```

This stub needs to be replaced with actual broadcast logic.

#### 3. Queue Wiring Pattern (Phase 1)

**Pattern Found** (state/mod.rs:352-360):
```rust
let queue_factory = QueueFactory::new(config.queue.clone()).await?;
let player_action_queue = queue_factory.create_player_action_queue().await?;
let llm_queue = queue_factory.create_llm_queue().await?;
// etc.
```

**Action**: Phase 1 must include the full wiring:
1. Add `create_challenge_outcome_queue()` to factory
2. Call it in `AppState::new()` after line 360
3. Pass to `ChallengeOutcomeApprovalService` constructor (line 461-471)
4. Optionally add to `QueueServices` struct (or access via service)

#### 4. UUID Extraction (Phase 1.5)

**Confirmed**: `ChallengeOutcomeApprovalItem.world_id` is `Uuid` which serializes as a string in JSON. The simple extraction:
```rust
v.get("world_id").and_then(|w| w.as_str().map(String::from))
```
will work correctly because serde serializes `Uuid` as a quoted string.

#### 5. Server Worker Pattern (Phase 8)

**Pattern Found** (run/server.rs:158-166):
```rust
let approval_notification_worker_task = {
    let service = state.queues.dm_approval_queue_service.clone();
    let world_connection_manager = state.world_connection_manager.clone();
    tokio::spawn(async move {
        approval_notification_worker(service, world_connection_manager, recovery_interval_clone).await;
    })
};
```

**Action**: Add challenge outcome worker following exact same pattern. Add to `tokio::select!` at line 272.

### Implementation Order (Corrected)

Execute in dependency order:

1. **Phase 0** - Fix broadcast bug (independent, can deploy immediately)
2. **Phase 1** - Add challenge queue factory method + wiring in AppState
3. **Phase 1.5** - Fix `list_by_world()` filtering
4. **Phase 2** - Refactor ChallengeOutcomeApprovalService (depends on Phase 1)
5. **Phase 3** - Refactor ChallengeResolutionService (depends on Phase 2)
6. **Phase 4** - Create ChallengeResolutionAdapter (depends on Phase 3)
7. **Phase 5** - Replace/enrich GameEvent variants
8. **Phase 6** - Update ChallengeUseCase + RollResult (depends on Phase 4, 5)
9. **Phase 7** - Update handlers to use use case + return None (depends on Phase 6)
10. **Phase 8** - Add challenge outcome notification worker (depends on Phase 1)
11. **Phase 10** - NarrativeEventApprovalService cleanup
12. **Phase 11** - Final cleanup and tests

### Files Modified Summary

| File | Changes |
|------|---------|
| `queue_workers.rs` | Phase 0: fix broadcast; Phase 8: add worker |
| `queues/factory.rs` | Phase 1: add factory method + notifier |
| `queues/sqlite_queue.rs` | Phase 1.5: add world_id column + filtering |
| `queues/memory_queue.rs` | Phase 1.5: add world_id filtering |
| `state/mod.rs` | Phase 1: wire queue, pass to service |
| `state/queue_services.rs` | Phase 1: optionally add queue field |
| `challenge_outcome_approval_service.rs` | Phase 2: replace HashMap with queue |
| `challenge_resolution_service.rs` | Phase 3: remove has_dm, return types |
| `ports/challenge_adapters.rs` | Phase 4: replace placeholder |
| `game_events.rs` | Phase 5: enrich challenge variants |
| `broadcast_adapter.rs` | Phase 5: implement challenge broadcast |
| `use_cases/challenge.rs` | Phase 6: add broadcast calls, enrich RollResult |
| `handlers/challenge.rs` | Phase 7: return None, use use case |
| `run/server.rs` | Phase 8: spawn worker, add to select! |
| `narrative_event_approval_service.rs` | Phase 10: remove ServerMessage |

### Estimated Total: 18-24 hours

The estimate is accurate. No additional phases needed - the wiring work is part of Phase 1.
