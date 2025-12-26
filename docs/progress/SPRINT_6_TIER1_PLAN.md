# Sprint 6 Tier 1 Implementation Plan

**Created**: 2025-12-26  
**Status**: PLANNING  
**Estimated Effort**: 1.5-2 days  
**Priority**: High (Infrastructure + Architecture)

This plan covers Tier 1 tasks from the Implementation Backlog: Token Budget Enforcement (P3.5) and WebSocket-First Architecture Phase 1 (P3.6).

---

## Executive Summary

Sprint 6 Tier 1 focuses on two foundational improvements:

1. **P3.5: Token Budget Enforcement** - Wire the existing `ContextBudgetConfig` and `TokenCounter` into prompt building so configured budgets actually take effect.

2. **P3.6: WebSocket-First Architecture Phase 1** - Audit REST endpoints that modify game state and add broadcasts to ensure multiplayer consistency.

Both tasks improve system reliability and prepare the codebase for future scaling.

---

## P3.5: Token Budget Enforcement

| Field | Value |
|-------|-------|
| **Status** | Not Started |
| **Effort** | 4-6 hours |
| **Priority** | Medium |
| **Complexity** | Medium |

### Problem Statement

`ContextBudgetConfig` settings exist in `domain/value_objects/context_budget.rs` and are exposed via the Settings API, but the actual token counting and budget enforcement is not implemented. Users can configure budgets but they have no effect on LLM prompts.

### Current Architecture

**Key Types** (in `crates/domain/src/value_objects/context_budget.rs`):

```rust
// Lines 89-124: Budget configuration with per-category limits
pub struct ContextBudgetConfig {
    pub total_budget_tokens: usize,      // Default: 4000
    pub scene_tokens: usize,             // Default: 500
    pub character_tokens: usize,         // Default: 800
    pub conversation_history_tokens: usize, // Default: 1000
    pub challenges_tokens: usize,        // Default: 400
    pub narrative_events_tokens: usize,  // Default: 400
    pub directorial_notes_tokens: usize, // Default: 300
    pub location_context_tokens: usize,  // Default: 300
    pub player_context_tokens: usize,    // Default: 300
    pub enable_summarization: bool,      // Default: true
    pub summarization_model: Option<String>,
}

// Lines 279-299: Token counter with hybrid char/word counting
pub struct TokenCounter {
    method: TokenCountMethod,
    chars_per_token: f64,    // ~4.0
    tokens_per_word: f64,    // ~1.33
    hybrid_threshold: usize, // 100 chars
}

// Lines 376-402: Truncation method
impl TokenCounter {
    pub fn truncate_to_budget(&self, text: &str, budget: usize) -> (String, bool);
}
```

**Current Prompt Building** (in `crates/engine-adapters/src/infrastructure/websocket_helpers.rs`):

The `build_prompt_from_action()` function (lines 24-356) builds `GamePromptRequest` with context sections:
- Scene context (lines 130-153)
- Character context (lines 156-231)
- Active challenges (lines 253-307)
- Active narrative events (lines 309-339)

Currently, no token counting or budget enforcement is applied.

### Implementation Plan

#### Step 1: Create BudgetEnforcedContextBuilder (1 hour)

Create a new helper module that wraps context section building with budget enforcement.

**File**: `crates/engine-adapters/src/infrastructure/context_budget.rs` (NEW)

```rust
//! Context budget enforcement for LLM prompts
//! 
//! Applies token counting and truncation to context sections
//! based on ContextBudgetConfig settings.

use wrldbldr_domain::value_objects::{
    ContextBudgetConfig, ContextCategory, TokenCounter,
};
use tracing::{debug, warn};

/// Result of enforcing budget on a context section
#[derive(Debug)]
pub struct BudgetResult {
    pub content: String,
    pub original_tokens: usize,
    pub final_tokens: usize,
    pub was_truncated: bool,
}

/// Enforces token budgets on context sections
pub struct ContextBudgetEnforcer {
    config: ContextBudgetConfig,
    counter: TokenCounter,
}

impl ContextBudgetEnforcer {
    /// Create enforcer from world settings
    pub fn new(config: ContextBudgetConfig) -> Self {
        Self {
            config,
            counter: TokenCounter::default(),
        }
    }
    
    /// Create enforcer tuned for Llama models
    pub fn llama_tuned(config: ContextBudgetConfig) -> Self {
        Self {
            config,
            counter: TokenCounter::llama_tuned(),
        }
    }

    /// Enforce budget on a context section, truncating if necessary
    pub fn enforce(&self, category: ContextCategory, content: &str) -> BudgetResult {
        let budget = self.config.budget_for(category);
        let original_tokens = self.counter.count(content);
        
        if original_tokens <= budget {
            return BudgetResult {
                content: content.to_string(),
                original_tokens,
                final_tokens: original_tokens,
                was_truncated: false,
            };
        }
        
        // Log truncation for debugging
        warn!(
            category = ?category,
            original_tokens = original_tokens,
            budget = budget,
            "Context section exceeds token budget, truncating"
        );
        
        let (truncated, _) = self.counter.truncate_to_budget(content, budget);
        let final_tokens = self.counter.count(&truncated);
        
        BudgetResult {
            content: truncated,
            original_tokens,
            final_tokens,
            was_truncated: true,
        }
    }
    
    /// Check if summarization is enabled
    pub fn summarization_enabled(&self) -> bool {
        self.config.enable_summarization
    }
    
    /// Get the total budget
    pub fn total_budget(&self) -> usize {
        self.config.total_budget_tokens
    }
    
    /// Count tokens in text
    pub fn count_tokens(&self, text: &str) -> usize {
        self.counter.count(text)
    }
}

/// Summary of budget enforcement across all sections
#[derive(Debug, Default)]
pub struct BudgetEnforcementSummary {
    pub sections_truncated: Vec<ContextCategory>,
    pub total_original_tokens: usize,
    pub total_final_tokens: usize,
}
```

**Verification**:
```bash
cargo check -p wrldbldr-engine-adapters
```

**Time**: 45 minutes

---

#### Step 2: Update websocket_helpers.rs to Use Budget Enforcer (2 hours)

Modify `build_prompt_from_action()` to apply budget enforcement to each context section.

**File**: `crates/engine-adapters/src/infrastructure/websocket_helpers.rs`

**Changes at line 6** (add import):
```rust
use crate::infrastructure::context_budget::ContextBudgetEnforcer;
use wrldbldr_domain::value_objects::{ContextBudgetConfig, ContextCategory};
```

**Changes at line 36** (add budget config parameter):
```rust
pub async fn build_prompt_from_action(
    sessions: &Arc<RwLock<SessionManager>>,
    challenge_service: &Arc<ChallengeServiceImpl>,
    skill_service: &Arc<SkillServiceImpl>,
    narrative_event_service: &Arc<NarrativeEventServiceImpl>,
    character_repo: &Arc<dyn CharacterRepositoryPort>,
    pc_repo: &Arc<dyn PlayerCharacterRepositoryPort>,
    region_repo: &Arc<dyn RegionRepositoryPort>,
    settings_service: &Arc<SettingsService>,
    mood_service: &Arc<MoodServiceImpl>,
    actantial_service: &Arc<ActantialContextServiceImpl>,
    action: &PlayerActionItem,
) -> Result<GamePromptRequest, QueueError> {
```

**Changes after line 240** (get budget config from settings):
```rust
    // Get per-world settings for conversation history limit
    let settings = settings_service.get_for_world(world_id).await;
    
    // Create budget enforcer from world settings
    let budget_enforcer = ContextBudgetEnforcer::new(settings.context_budget.clone());
```

**Changes for scene context** (after line 153):
```rust
    // Build scene context string for budget checking
    let scene_text = format!(
        "Scene: {}\nLocation: {}\nTime: {}\nPresent: {}",
        scene_context.scene_name,
        scene_context.location_name,
        scene_context.time_context,
        scene_context.present_characters.join(", ")
    );
    let scene_budget_result = budget_enforcer.enforce(
        ContextCategory::Scene,
        &scene_text
    );
    if scene_budget_result.was_truncated {
        tracing::debug!(
            "Scene context truncated from {} to {} tokens",
            scene_budget_result.original_tokens,
            scene_budget_result.final_tokens
        );
    }
```

**Changes for character context** (after line 231):
```rust
    // Serialize character context for budget check
    let char_text = format!(
        "Character: {} ({})\nMood: {:?}\nRelationship: {:?}",
        character_context.name,
        character_context.archetype,
        character_context.current_mood,
        character_context.relationship_to_player
    );
    let char_budget_result = budget_enforcer.enforce(
        ContextCategory::Character,
        &char_text
    );
    if char_budget_result.was_truncated {
        tracing::debug!(
            "Character context truncated from {} to {} tokens",
            char_budget_result.original_tokens,
            char_budget_result.final_tokens
        );
    }
```

**Changes for conversation history** (after line 250):
```rust
    // Enforce budget on conversation history
    let history_text = conversation_history
        .iter()
        .map(|t| format!("{}: {}", t.speaker, t.text))
        .collect::<Vec<_>>()
        .join("\n");
    let history_budget_result = budget_enforcer.enforce(
        ContextCategory::ConversationHistory,
        &history_text
    );
    
    // If truncated, we need to reduce the history
    let conversation_history = if history_budget_result.was_truncated {
        // Take fewer turns to fit budget
        let target_chars = budget_enforcer.count_tokens(&history_budget_result.content) * 4;
        let mut kept_turns = Vec::new();
        let mut char_count = 0;
        for turn in conversation_history.into_iter().rev() {
            let turn_len = turn.speaker.len() + turn.text.len() + 2;
            if char_count + turn_len > target_chars {
                break;
            }
            char_count += turn_len;
            kept_turns.push(turn);
        }
        kept_turns.reverse();
        kept_turns
    } else {
        conversation_history
    };
```

**Verification**:
```bash
cargo check -p wrldbldr-engine-adapters
cargo test -p wrldbldr-engine-adapters
```

**Time**: 2 hours

---

#### Step 3: Update PromptBuilder for Budget Awareness (1.5 hours)

Optionally enhance the `PromptBuilder` in `prompt_builder.rs` to accept budget constraints.

**File**: `crates/engine-app/src/application/services/llm/prompt_builder.rs`

**Changes at line 26** (add imports):
```rust
use wrldbldr_domain::value_objects::{ContextBudgetConfig, ContextCategory, TokenCounter};
```

**Add new method after line 226**:
```rust
    /// Build system prompt with budget enforcement
    pub async fn build_system_prompt_with_budget(
        &self,
        world_id: Option<WorldId>,
        context: &SceneContext,
        character: &CharacterContext,
        directorial_notes: Option<&DirectorialNotes>,
        active_challenges: &[ActiveChallengeContext],
        active_narrative_events: &[ActiveNarrativeEventContext],
        budget_config: &ContextBudgetConfig,
    ) -> (String, BudgetReport) {
        let counter = TokenCounter::default();
        let mut report = BudgetReport::default();
        
        // Build the full prompt first
        let full_prompt = self.build_system_prompt_with_notes(
            world_id,
            context,
            character,
            directorial_notes,
            active_challenges,
            active_narrative_events,
        ).await;
        
        let total_tokens = counter.count(&full_prompt);
        report.total_tokens = total_tokens;
        
        // Check against total budget
        if total_tokens > budget_config.total_budget_tokens {
            report.exceeded_budget = true;
            report.budget_limit = budget_config.total_budget_tokens;
            tracing::warn!(
                total_tokens = total_tokens,
                budget = budget_config.total_budget_tokens,
                "System prompt exceeds total token budget"
            );
        }
        
        (full_prompt, report)
    }
}

/// Report of token budget usage
#[derive(Debug, Default)]
pub struct BudgetReport {
    pub total_tokens: usize,
    pub exceeded_budget: bool,
    pub budget_limit: usize,
}
```

**Verification**:
```bash
cargo check -p wrldbldr-engine-app
cargo test -p wrldbldr-engine-app
```

**Time**: 1 hour

---

#### Step 4: Wire Module into Infrastructure (30 minutes)

**File**: `crates/engine-adapters/src/infrastructure/mod.rs`

Add the new module to the infrastructure module exports:
```rust
pub mod context_budget;
```

**Verification**:
```bash
cargo check --workspace
cargo xtask arch-check
```

**Time**: 30 minutes

---

#### Step 5: Add Integration Tests (1 hour)

**File**: `crates/engine-adapters/src/infrastructure/context_budget.rs` (append tests)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_domain::value_objects::ContextBudgetConfig;

    #[test]
    fn test_enforce_under_budget() {
        let config = ContextBudgetConfig::default();
        let enforcer = ContextBudgetEnforcer::new(config);
        
        let short_text = "A brief scene description.";
        let result = enforcer.enforce(ContextCategory::Scene, short_text);
        
        assert!(!result.was_truncated);
        assert_eq!(result.content, short_text);
    }
    
    #[test]
    fn test_enforce_over_budget() {
        let mut config = ContextBudgetConfig::default();
        config.scene_tokens = 5; // Very small budget
        let enforcer = ContextBudgetEnforcer::new(config);
        
        let long_text = "This is a very long scene description that should definitely exceed the tiny budget we have set for testing purposes.";
        let result = enforcer.enforce(ContextCategory::Scene, long_text);
        
        assert!(result.was_truncated);
        assert!(result.final_tokens <= 10); // Allow some margin
        assert!(result.content.ends_with("..."));
    }
    
    #[test]
    fn test_category_budgets() {
        let config = ContextBudgetConfig::default();
        let enforcer = ContextBudgetEnforcer::new(config.clone());
        
        assert_eq!(enforcer.enforce(ContextCategory::Scene, "x").final_tokens, 1);
        assert_eq!(config.budget_for(ContextCategory::Scene), 500);
        assert_eq!(config.budget_for(ContextCategory::Character), 800);
    }
}
```

**Verification**:
```bash
cargo test -p wrldbldr-engine-adapters context_budget
```

**Time**: 1 hour

---

### P3.5 Success Criteria

- [ ] `ContextBudgetEnforcer` module created and tested
- [ ] `websocket_helpers.rs` uses budget enforcer for all context sections
- [ ] Truncation logs appear when sections exceed budget (debug level)
- [ ] `cargo check --workspace && cargo xtask arch-check` passes
- [ ] Token counts in prompts respect configured budgets

### P3.5 Time Estimate Summary

| Step | Task | Time |
|------|------|------|
| 1 | Create ContextBudgetEnforcer module | 45 min |
| 2 | Update websocket_helpers.rs | 2 hours |
| 3 | Update PromptBuilder (optional) | 1 hour |
| 4 | Wire into infrastructure | 30 min |
| 5 | Add integration tests | 1 hour |
| **Total** | | **5-6 hours** |

---

## P3.6: WebSocket-First Architecture Phase 1

| Field | Value |
|-------|-------|
| **Status** | Not Started |
| **Effort** | 1 day |
| **Priority** | High |
| **Complexity** | Low-Medium |

### Problem Statement

Game logic is split between REST and WebSocket, causing:
- Duplicate code paths (actantial CRUD exists in both REST and WebSocket)
- Missing broadcasts (REST game actions don't always notify clients)
- Inconsistent multiplayer behavior

**Reference**: `docs/plans/WEBSOCKET_ARCHITECTURE.md` for full analysis.

### Current Architecture Analysis

**REST Endpoints That Modify Game State** (from audit):

| Route File | State-Modifying Endpoints | Broadcasts? |
|------------|--------------------------|-------------|
| `want_routes.rs` | create_want, update_want, delete_want, set_want_target, add_actantial_view, remove_actantial_view | NO |
| `goal_routes.rs` | create_goal, update_goal, delete_goal | NO |
| `character_routes.rs` | create_character, update_character, delete_character, change_archetype, create_relationship, delete_relationship, add_region_relationship, remove_region_relationship | NO |
| `challenge_routes.rs` | create, update, delete, trigger_conditions | NO |
| `narrative_event_routes.rs` | create, update, delete | NO |
| `scene_routes.rs` | create, update, delete | PARTIAL |
| `location_routes.rs` | create, update, delete | NO |
| `settings_routes.rs` | update_settings | NO |

**WebSocket Already Has Broadcasts For**:
- Actantial changes (NpcWantCreated, NpcWantUpdated, etc.)
- Scene changes (SceneChanged, SceneUpdate)
- Staging (StagingApproved, StagingReady)
- Challenges (ChallengeResolved, ChallengeRollSubmitted)

### Implementation Plan

#### Step 1: Create Broadcast Helper Module (1 hour)

Create a utility module for broadcasting state changes from REST handlers.

**File**: `crates/engine-adapters/src/infrastructure/broadcast.rs` (NEW)

```rust
//! Broadcast utilities for REST endpoint state changes
//!
//! Ensures REST operations that modify game state notify connected WebSocket clients.

use std::sync::Arc;
use tokio::sync::broadcast;
use wrldbldr_domain::{CharacterId, GoalId, WantId, WorldId};
use wrldbldr_protocol::{ClientMessage, ServerMessage};

/// Broadcast channel for server-to-client messages
pub type BroadcastTx = broadcast::Sender<ServerMessage>;

/// Helper for broadcasting state changes
pub struct StateBroadcaster {
    tx: BroadcastTx,
}

impl StateBroadcaster {
    pub fn new(tx: BroadcastTx) -> Self {
        Self { tx }
    }
    
    /// Broadcast that a want was created
    pub fn want_created(&self, npc_id: CharacterId, want_id: WantId, description: String) {
        let _ = self.tx.send(ServerMessage::NpcWantCreated {
            npc_id: npc_id.to_string(),
            want_id: want_id.to_string(),
            description,
        });
    }
    
    /// Broadcast that a want was updated
    pub fn want_updated(&self, npc_id: CharacterId, want_id: WantId) {
        let _ = self.tx.send(ServerMessage::NpcWantUpdated {
            npc_id: npc_id.to_string(),
            want_id: want_id.to_string(),
        });
    }
    
    /// Broadcast that a want was deleted
    pub fn want_deleted(&self, npc_id: CharacterId, want_id: WantId) {
        let _ = self.tx.send(ServerMessage::NpcWantDeleted {
            npc_id: npc_id.to_string(),
            want_id: want_id.to_string(),
        });
    }
    
    /// Broadcast that an actantial view was added
    pub fn actantial_view_added(&self, npc_id: CharacterId) {
        let _ = self.tx.send(ServerMessage::NpcActantialViewAdded {
            npc_id: npc_id.to_string(),
        });
    }
    
    /// Broadcast that an actantial view was removed
    pub fn actantial_view_removed(&self, npc_id: CharacterId) {
        let _ = self.tx.send(ServerMessage::NpcActantialViewRemoved {
            npc_id: npc_id.to_string(),
        });
    }
    
    /// Broadcast that a goal was created
    pub fn goal_created(&self, world_id: WorldId, goal_id: GoalId, name: String) {
        let _ = self.tx.send(ServerMessage::GoalCreated {
            world_id: world_id.to_string(),
            goal_id: goal_id.to_string(),
            name,
        });
    }
    
    /// Broadcast that a goal was updated
    pub fn goal_updated(&self, goal_id: GoalId) {
        let _ = self.tx.send(ServerMessage::GoalUpdated {
            goal_id: goal_id.to_string(),
        });
    }
    
    /// Broadcast that a goal was deleted
    pub fn goal_deleted(&self, goal_id: GoalId) {
        let _ = self.tx.send(ServerMessage::GoalDeleted {
            goal_id: goal_id.to_string(),
        });
    }
    
    /// Broadcast generic refresh signal for a world
    pub fn world_data_changed(&self, world_id: WorldId) {
        let _ = self.tx.send(ServerMessage::WorldDataChanged {
            world_id: world_id.to_string(),
        });
    }
}
```

**Note**: Some of these message types may need to be added to the protocol.

**Verification**:
```bash
cargo check -p wrldbldr-engine-adapters
```

**Time**: 1 hour

---

#### Step 2: Add Missing Protocol Messages (30 minutes)

**File**: `crates/protocol/src/messages.rs`

Add broadcast message types for operations not yet covered:

```rust
// Add to ServerMessage enum (if not present):

/// A goal was created
GoalCreated {
    world_id: String,
    goal_id: String,
    name: String,
},

/// A goal was updated
GoalUpdated {
    goal_id: String,
},

/// A goal was deleted
GoalDeleted {
    goal_id: String,
},

/// Generic signal that world data changed (causes UI refresh)
WorldDataChanged {
    world_id: String,
},

/// Character data was updated
CharacterUpdated {
    character_id: String,
},

/// Character was deleted
CharacterDeleted {
    character_id: String,
},
```

**Verification**:
```bash
cargo check -p wrldbldr-protocol
```

**Time**: 30 minutes

---

#### Step 3: Add Broadcaster to AppState (30 minutes)

**File**: `crates/engine-adapters/src/infrastructure/state/mod.rs`

Add broadcast channel to AppState:

```rust
use crate::infrastructure::broadcast::{BroadcastTx, StateBroadcaster};
use tokio::sync::broadcast;

// In AppState struct (or create if needed):
pub struct AppState {
    // ... existing fields ...
    pub broadcaster: Arc<StateBroadcaster>,
}

// In AppState initialization:
impl AppState {
    pub fn new(/* params */) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1024);
        let broadcaster = Arc::new(StateBroadcaster::new(broadcast_tx.clone()));
        
        Self {
            // ... existing fields ...
            broadcaster,
        }
    }
}
```

**Verification**:
```bash
cargo check -p wrldbldr-engine-adapters
```

**Time**: 30 minutes

---

#### Step 4: Update want_routes.rs with Broadcasts (1 hour)

**File**: `crates/engine-adapters/src/infrastructure/http/want_routes.rs`

**Changes to create_want** (after line 301, before returning):
```rust
    // Broadcast the change
    state.broadcaster.want_created(
        char_id,
        want_id,
        create_req.description.clone(),
    );

    Ok((
        StatusCode::CREATED,
        // ...
    ))
```

**Changes to update_want** (after line 337, before returning):
```rust
    // Broadcast the change - need to get character_id from want
    // Note: May need to query want to get parent character
    state.broadcaster.want_updated(
        CharacterId::from_uuid(/* get from service */),
        want_id,
    );

    Ok(StatusCode::NO_CONTENT)
```

**Changes to delete_want** (after line 358, before returning):
```rust
    state.broadcaster.want_deleted(
        CharacterId::from_uuid(/* get from service */),
        want_id,
    );

    Ok(StatusCode::NO_CONTENT)
```

**Changes to add_actantial_view** (after line 510):
```rust
    state.broadcaster.actantial_view_added(char_id);
    Ok(StatusCode::CREATED)
```

**Changes to remove_actantial_view_impl** (after line 559):
```rust
    state.broadcaster.actantial_view_removed(char_id);
    Ok(StatusCode::NO_CONTENT)
```

**Verification**:
```bash
cargo check -p wrldbldr-engine-adapters
```

**Time**: 1 hour

---

#### Step 5: Update goal_routes.rs with Broadcasts (45 minutes)

**File**: `crates/engine-adapters/src/infrastructure/http/goal_routes.rs`

**Changes to create_goal** (after line 127, before returning):
```rust
    state.broadcaster.goal_created(world_id, goal.id, goal.name.clone());

    Ok((StatusCode::CREATED, Json(GoalResponse::from(goal))))
```

**Changes to update_goal** (after line 191, before returning):
```rust
    state.broadcaster.goal_updated(goal_id);

    Ok(Json(GoalResponse::from(goal)))
```

**Changes to delete_goal** (after line 236, before returning):
```rust
    state.broadcaster.goal_deleted(goal_id);

    Ok(StatusCode::NO_CONTENT)
```

**Verification**:
```bash
cargo check -p wrldbldr-engine-adapters
```

**Time**: 45 minutes

---

#### Step 6: Update character_routes.rs with Broadcasts (1 hour)

**File**: `crates/engine-adapters/src/infrastructure/http/character_routes.rs`

Add broadcasts after state-modifying operations:

- `create_character` → broadcast `WorldDataChanged`
- `update_character` → broadcast `CharacterUpdated`
- `delete_character` → broadcast `CharacterDeleted`
- `change_archetype` → broadcast `CharacterUpdated`
- `create_relationship` → broadcast `WorldDataChanged`
- `delete_relationship` → broadcast `WorldDataChanged`
- `add_region_relationship` → broadcast `CharacterUpdated`
- `remove_region_relationship` → broadcast `CharacterUpdated`

**Verification**:
```bash
cargo check -p wrldbldr-engine-adapters
```

**Time**: 1 hour

---

#### Step 7: Update Player UI Message Handler (1 hour)

**File**: `crates/player-ui/src/presentation/handlers/session_message_handler.rs`

Add handlers for new message types to trigger UI refresh:

```rust
ServerMessage::GoalCreated { world_id, .. } => {
    // Trigger goals panel refresh
    state.trigger_actantial_refresh();
}

ServerMessage::GoalUpdated { .. } => {
    state.trigger_actantial_refresh();
}

ServerMessage::GoalDeleted { .. } => {
    state.trigger_actantial_refresh();
}

ServerMessage::WorldDataChanged { world_id } => {
    // Generic refresh - could trigger multiple signals
    state.trigger_actantial_refresh();
}

ServerMessage::CharacterUpdated { character_id } => {
    // Refresh character-related UI
    state.trigger_actantial_refresh();
}

ServerMessage::CharacterDeleted { character_id } => {
    state.trigger_actantial_refresh();
}
```

**Verification**:
```bash
cargo check -p wrldbldr-player-ui
```

**Time**: 1 hour

---

#### Step 8: Document the Pattern (30 minutes)

Update documentation for future development.

**File**: `docs/plans/WEBSOCKET_ARCHITECTURE.md`

Add section:

```markdown
## Phase 1 Complete: REST Broadcast Pattern

As of Sprint 6, REST endpoints that modify game state now broadcast changes
to connected WebSocket clients. This ensures multiplayer consistency.

### Pattern for New Endpoints

When adding a REST endpoint that modifies game state:

1. Import the broadcaster: `use crate::infrastructure::broadcast::StateBroadcaster;`
2. After the state change, call the appropriate broadcast method:
   ```rust
   state.broadcaster.want_created(char_id, want_id, description);
   ```
3. Add handler in `session_message_handler.rs` if new message type

### Covered Endpoints

- want_routes.rs: All CRUD operations
- goal_routes.rs: All CRUD operations
- character_routes.rs: All CRUD operations
```

**Time**: 30 minutes

---

### P3.6 Success Criteria

- [ ] `StateBroadcaster` module created
- [ ] Protocol messages added for Goal and Character changes
- [ ] `want_routes.rs` broadcasts on all mutations
- [ ] `goal_routes.rs` broadcasts on all mutations
- [ ] `character_routes.rs` broadcasts on all mutations
- [ ] Player UI handles new message types
- [ ] Documentation updated
- [ ] `cargo check --workspace && cargo xtask arch-check` passes

### P3.6 Time Estimate Summary

| Step | Task | Time |
|------|------|------|
| 1 | Create StateBroadcaster module | 1 hour |
| 2 | Add protocol messages | 30 min |
| 3 | Add broadcaster to AppState | 30 min |
| 4 | Update want_routes.rs | 1 hour |
| 5 | Update goal_routes.rs | 45 min |
| 6 | Update character_routes.rs | 1 hour |
| 7 | Update player UI handler | 1 hour |
| 8 | Document pattern | 30 min |
| **Total** | | **6-7 hours** |

---

## Dependencies Between Tasks

```
P3.5 Token Budget ──────────────────────────────────────────> Independent
                                                                   │
P3.6 WebSocket Broadcasts ──────────────────────────────────────> │
         │                                                         │
         └─────────────────────────────────────────────────────────┘
                          No dependencies between tasks
```

Both P3.5 and P3.6 are **independent** and can be worked in parallel or any order.

---

## Implementation Order Recommendation

### Day 1 (Morning): P3.5 Token Budget

1. **Step 1**: Create `context_budget.rs` module (45 min)
2. **Step 2**: Update `websocket_helpers.rs` (2 hours)
3. **Step 3**: Wire into infrastructure (30 min)
4. **Step 4**: Test and verify (45 min)

### Day 1 (Afternoon): P3.6 WebSocket Broadcasts

1. **Step 1**: Create `broadcast.rs` module (1 hour)
2. **Step 2**: Add protocol messages (30 min)
3. **Step 3**: Add broadcaster to AppState (30 min)
4. **Step 4-6**: Update route files (2.5 hours)

### Day 2 (Morning): Complete P3.6

1. **Step 7**: Update player UI handler (1 hour)
2. **Step 8**: Document pattern (30 min)
3. **Verification**: Full integration test (1 hour)

---

## Verification Commands

After each step:
```bash
cargo check -p wrldbldr-engine-adapters
```

After completing P3.5:
```bash
cargo test -p wrldbldr-engine-adapters context_budget
```

After completing both:
```bash
cargo check --workspace && cargo xtask arch-check
```

Full test suite:
```bash
cargo test --workspace
```

---

## Rollback Plan

### P3.5 Rollback
If budget enforcement causes issues:
1. Remove calls to `ContextBudgetEnforcer` in `websocket_helpers.rs`
2. Keep module for future use
3. Budgets will be ignored (current behavior)

### P3.6 Rollback
If broadcasts cause issues:
1. Comment out broadcast calls in route files
2. REST operations continue to work, just without broadcasts
3. UI refresh happens on next explicit action

---

## Related Files

### P3.5
- `crates/domain/src/value_objects/context_budget.rs` (existing)
- `crates/engine-adapters/src/infrastructure/websocket_helpers.rs` (modify)
- `crates/engine-app/src/application/services/llm/prompt_builder.rs` (optional)
- `crates/engine-adapters/src/infrastructure/context_budget.rs` (new)

### P3.6
- `crates/engine-adapters/src/infrastructure/http/want_routes.rs` (modify)
- `crates/engine-adapters/src/infrastructure/http/goal_routes.rs` (modify)
- `crates/engine-adapters/src/infrastructure/http/character_routes.rs` (modify)
- `crates/protocol/src/messages.rs` (modify)
- `crates/player-ui/src/presentation/handlers/session_message_handler.rs` (modify)
- `crates/engine-adapters/src/infrastructure/broadcast.rs` (new)

---

## Notes

- All estimates assume familiarity with codebase
- Run `cargo check --workspace && cargo xtask arch-check` after each feature
- NixOS environment - use `nix-shell` for all commands
- Update `IMPLEMENTATION_BACKLOG.md` when completing features
