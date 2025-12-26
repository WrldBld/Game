# Sprint 6: Tier 2 Implementation Plan

**Created**: 2025-12-26  
**Status**: Not Started  
**Estimated Effort**: 8-9 hours  
**Priority**: P1 (Tier 2)

---

## Executive Summary

This sprint implements Tier 2 features that enhance NPC immersion and player agency:

| Task | Effort | Status | Priority |
|------|--------|--------|----------|
| P1.1: US-REGION-ITEMS Phases 5-6 | 6h | N/A - Already Complete | High |
| P1.4: Character Mood Evolution | 2-3h | **SUPERSEDED** | High |

**Discovery Note**: During Sprint 4 analysis, it was discovered that US-REGION-ITEMS Phases 5-6 are **already implemented**:
- `RegionItemContext` exists in `llm_context.rs`
- `SceneContext.region_items` populated during prompt building
- `RegionItemsPanel` modal component with "Pick Up" buttons exists
- Full pickup flow via `send_pickup_item()` is functional

---

## P1.4: Character Mood Evolution - SUPERSEDED

### Status: Superseded by Mood & Expression System Redesign

The original P1.4 plan for automatic mood updates has been **superseded** by a comprehensive redesign of the mood system.

**See**: `docs/plans/MOOD_EXPRESSION_SYSTEM.md`

### Key Changes from Original Plan

The new design takes a different approach:

| Original P1.4 Approach | New Mood System Approach |
|------------------------|--------------------------|
| Automatic sentiment analysis of dialogue | LLM embeds mood markers in dialogue |
| Keyword-based mood detection | Explicit `*mood*` or `*mood|expression*` markers |
| System-driven mood updates | LLM proposes mood changes via tool calls |
| Single mood per conversation turn | Multiple mood shifts during typewriter playback |

### New System Features

1. **Inline Mood Markers**: `*happy*` `*nervous|afraid*` embedded in dialogue text
2. **Expression Mapping**: Custom moods map to available character expressions
3. **Action Markers**: `*sighs*` `*laughs*` for transient physical actions
4. **Tool-Based Mood Changes**: LLM proposes `change_mood` tool calls for permanent state changes
5. **PC Support**: Players can add mood markers to their input
6. **Mood History**: Track mood changes over conversation

### Implementation

The mood system redesign is a larger effort (3-4 days) and should be implemented as a dedicated sprint. The original P1.4 automatic mood evolution is no longer needed because:

1. The LLM will naturally include mood markers in dialogue
2. The `change_mood` tool allows the LLM to propose permanent mood changes
3. DM approval ensures mood changes are appropriate

---

## Remaining Tier 2 Work

Since P1.1 is already complete and P1.4 is superseded, Tier 2 is effectively complete once the Mood & Expression System is implemented (tracked separately).

**Next Steps**:
1. Complete Tier 1 (Token Budget + WebSocket-First)
2. Implement Mood & Expression System from `docs/plans/MOOD_EXPRESSION_SYSTEM.md`
3. Proceed to Tier 3

---

## Original P1.4 Plan (For Reference)

The content below is preserved for historical reference but is no longer the implementation plan.

---

## P1.4: Character Mood Evolution (Original - 2-3 hours)

### Problem Statement (Original)

NPC moods are currently only set manually by the DM via `SetNpcMood` WebSocket command. They don't evolve based on:
- Dialogue exchanges (positive/negative interactions)
- Challenge outcomes (success/failure)
- Narrative events

This breaks immersion as NPCs don't react naturally to player actions.

### Current State Analysis

**Existing Infrastructure (mood_service.rs:1-279):**
- `MoodService` trait with `apply_interaction()` method (line 43)
- `MoodServiceImpl` implements full interaction outcome handling (line 139)
- `InteractionOutcome` enum supports: `Positive`, `Negative`, `Neutral`, `ChallengeResult`
- Sentiment adjustment and relationship points calculation working
- Mood levels derive from sentiment: Friendly → Hostile spectrum

**Existing Domain (mood.rs:399-476):**
- `ChallengeSignificance` enum: Minor, Normal, Significant, Major
- Success/failure deltas: Minor success = +0.05, Major success = +0.4
- Relationship point changes: Minor = 1 point, Major = 10 points

**Missing Integration Points:**
1. Dialogue approval handler doesn't call `apply_interaction()`
2. Challenge resolution doesn't call `apply_interaction()`
3. No automatic mood updates broadcast to UI

### Implementation Plan

#### Step 1: Integrate Mood Updates with Challenge Resolution (45 min)

**Goal**: Automatically update NPC mood when a challenge involving an NPC is resolved.

**File**: `crates/engine-app/src/application/services/challenge_resolution_service.rs`

**Location**: After line 454 (after `publish ChallengeResolved event`)

**Changes**:

```rust
// After line 454 in resolve_challenge_internal()

// P1.4: Update NPC mood based on challenge outcome
// Determine if this challenge involves an NPC (check challenge.context or triggers)
if let Some(npc_id) = self.extract_challenge_npc_id(&preamble.challenge) {
    if let Some(pc_id) = preamble.character_id.and_then(|c| self.get_pc_id_from_character(&c)) {
        let significance = self.determine_challenge_significance(&preamble.challenge);
        let outcome = InteractionOutcome::ChallengeResult {
            succeeded: success,
            skill_name: preamble.challenge.skill.name.clone(),
            significance,
        };
        
        if let Err(e) = self.mood_service.apply_interaction(npc_id, pc_id, outcome).await {
            tracing::warn!("Failed to update NPC mood after challenge: {}", e);
        }
    }
}
```

**Helper methods to add**:

```rust
/// Extract NPC ID from challenge context if applicable
fn extract_challenge_npc_id(&self, challenge: &Challenge) -> Option<CharacterId> {
    // Check if challenge has an associated NPC (e.g., from social challenges)
    // This might be stored in challenge metadata or context
    challenge.context.get("npc_id")
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .map(CharacterId::from_uuid)
}

/// Determine challenge significance based on difficulty
fn determine_challenge_significance(&self, challenge: &Challenge) -> ChallengeSignificance {
    match challenge.difficulty.target_number {
        0..=10 => ChallengeSignificance::Minor,
        11..=15 => ChallengeSignificance::Normal,
        16..=20 => ChallengeSignificance::Significant,
        _ => ChallengeSignificance::Major,
    }
}
```

**Dependencies**: Add `mood_service: Arc<dyn MoodService>` to `ChallengeResolutionService` constructor.

**Verification**:
```bash
cargo check -p wrldbldr-engine-app
```

---

#### Step 2: Integrate Mood Updates with Dialogue Approval (45 min)

**Goal**: Update NPC mood after dialogue exchanges based on sentiment analysis.

**File**: `crates/engine-app/src/application/services/dm_approval_queue_service.rs`

**Changes**: After dialogue approval is processed (in `process_decision()`), analyze the dialogue and update mood.

**Location**: After the dialogue response is broadcast

```rust
// P1.4: Update NPC mood based on dialogue sentiment
if let Some(ref responding_npc_id) = approval_item.responding_character_id {
    if let Some(ref pc_id) = approval_item.initiating_pc_id {
        let sentiment = self.analyze_dialogue_sentiment(&approved_dialogue);
        let outcome = if sentiment > 0.1 {
            InteractionOutcome::Positive {
                magnitude: sentiment.abs(),
                reason: format!("Positive dialogue exchange"),
            }
        } else if sentiment < -0.1 {
            InteractionOutcome::Negative {
                magnitude: sentiment.abs(),
                reason: format!("Negative dialogue exchange"),
            }
        } else {
            InteractionOutcome::Neutral
        };
        
        if let Err(e) = self.mood_service.apply_interaction(
            *responding_npc_id, 
            *pc_id, 
            outcome
        ).await {
            tracing::warn!("Failed to update NPC mood after dialogue: {}", e);
        }
    }
}
```

**Sentiment Analysis Helper**:

```rust
/// Simple keyword-based sentiment analysis for dialogue
/// Returns a value from -1.0 to 1.0
fn analyze_dialogue_sentiment(&self, text: &str) -> f32 {
    let text_lower = text.to_lowercase();
    
    // Positive indicators
    let positive_keywords = [
        "thank", "grateful", "appreciate", "friend", "trust", "help",
        "wonderful", "excellent", "love", "pleased", "happy", "agree"
    ];
    let positive_count = positive_keywords.iter()
        .filter(|kw| text_lower.contains(*kw))
        .count() as f32;
    
    // Negative indicators
    let negative_keywords = [
        "hate", "angry", "betray", "liar", "fool", "stupid",
        "enemy", "threat", "kill", "destroy", "refuse", "never"
    ];
    let negative_count = negative_keywords.iter()
        .filter(|kw| text_lower.contains(*kw))
        .count() as f32;
    
    // Calculate sentiment (-1.0 to 1.0)
    let total = (positive_count + negative_count).max(1.0);
    ((positive_count - negative_count) / total).clamp(-1.0, 1.0) * 0.3
}
```

**Dependencies**: Add `mood_service: Arc<dyn MoodService>` to service constructor.

**Verification**:
```bash
cargo check -p wrldbldr-engine-app
```

---

#### Step 3: Broadcast Mood Changes to UI (30 min)

**Goal**: Notify players and DM when NPC mood changes significantly.

**File**: `crates/engine-app/src/application/services/mood_service.rs`

**Changes**: After `apply_interaction()` updates mood, broadcast change if significant.

**Add to MoodServiceImpl**:

```rust
/// Broadcast mood change notification to session
async fn broadcast_mood_change(
    &self,
    mood_state: &NpcMoodState,
    session_id: Option<SessionId>,
    npc_name: &str,
) -> Result<()> {
    if let Some(sid) = session_id {
        let msg = serde_json::json!({
            "type": "NpcMoodChanged",
            "npc_id": mood_state.npc_id.to_string(),
            "npc_name": npc_name,
            "pc_id": mood_state.pc_id.to_string(),
            "mood": mood_state.mood.display_name(),
            "relationship": mood_state.relationship.display_name(),
            "reason": mood_state.mood_reason.clone()
        });
        
        // Broadcast to DM and optionally to the PC
        if let Some(sessions) = &self.session_manager {
            let _ = sessions.broadcast_to_session(sid, msg).await;
        }
    }
    Ok(())
}
```

**Update apply_interaction() to call broadcast**:

```rust
// At end of apply_interaction(), before return:
// Get NPC name for broadcast
if let Ok(Some(npc)) = self.character_repo.get(npc_id).await {
    self.broadcast_mood_change(&mood_state, session_id, &npc.name).await?;
}
```

**Dependencies**: Add `session_manager: Option<Arc<dyn SessionManagementPort>>` to service.

**Verification**:
```bash
cargo check -p wrldbldr-engine-app
```

---

#### Step 4: Handle Mood Change in UI (30 min)

**Goal**: Display mood changes in DM panel when received.

**File**: `crates/player-ui/src/presentation/handlers/session_message_handler.rs`

**Add handler for NpcMoodChanged**:

```rust
// In the match statement for ServerMessage handling
ServerMessage::NpcMoodChanged {
    npc_id,
    npc_name,
    pc_id,
    mood,
    relationship,
    reason,
} => {
    tracing::info!(
        "NPC mood changed: {} ({}) is now {} toward PC {}",
        npc_name, npc_id, mood, pc_id
    );
    
    // Option 1: Add to activity log (if exists)
    // Option 2: Show toast notification
    // Option 3: Update cached mood state
    
    // For DM, show in approval queue notification area
    // For PC, optionally show subtle indicator if mood is visible
}
```

**File**: `crates/player-ui/src/presentation/state/game_state.rs`

**Add mood tracking signal**:

```rust
/// Track recent NPC mood changes for display
pub npc_mood_changes: Signal<Vec<NpcMoodChangeData>>,
```

**Data structure**:

```rust
#[derive(Debug, Clone)]
pub struct NpcMoodChangeData {
    pub npc_id: String,
    pub npc_name: String,
    pub mood: String,
    pub relationship: String,
    pub reason: Option<String>,
    pub timestamp: DateTime<Utc>,
}
```

**Verification**:
```bash
cargo check -p wrldbldr-player-ui
```

---

#### Step 5: Wire Up Dependencies (30 min)

**Goal**: Ensure MoodService is properly injected into services that need it.

**File**: `crates/engine-app/src/application/services/mod.rs`

**Ensure exports**:
```rust
pub use mood_service::{MoodService, MoodServiceImpl};
```

**File**: `crates/engine-adapters/src/infrastructure/state/mod.rs` or `game_services.rs`

**Update service construction**:

```rust
// In AppState initialization
let mood_service: Arc<dyn MoodService> = Arc::new(MoodServiceImpl::new(
    character_repo.clone(),
    // Optional: session_manager for broadcasts
));

// Pass to challenge_resolution_service
let challenge_resolution_service = Arc::new(ChallengeResolutionService::new(
    // ... existing deps ...
    mood_service.clone(),
));

// Pass to dm_approval_queue_service  
let dm_approval_queue_service = Arc::new(DMApprovalQueueService::new(
    // ... existing deps ...
    mood_service.clone(),
));
```

**Verification**:
```bash
cargo build -p wrldbldr-engine-adapters
```

---

### File Modification Summary

| File | Changes | Est. Lines |
|------|---------|------------|
| `challenge_resolution_service.rs` | Add mood update after challenge resolve | +30 |
| `dm_approval_queue_service.rs` | Add mood update after dialogue approval | +40 |
| `mood_service.rs` | Add broadcast capability | +25 |
| `session_message_handler.rs` | Handle NpcMoodChanged | +20 |
| `game_state.rs` | Add mood tracking signal | +10 |
| `state/mod.rs` or `game_services.rs` | Wire up dependencies | +15 |
| **Total** | | ~140 lines |

### Success Criteria

1. **Challenge-based mood changes**: When PC succeeds/fails a social challenge against an NPC, NPC mood updates automatically
2. **Dialogue-based mood changes**: After DM approves dialogue, NPC mood adjusts based on content sentiment
3. **Visible feedback**: DM sees mood change notifications in their panel
4. **Relationship progression**: Over multiple interactions, relationship level (Stranger → Friend → Ally) progresses naturally
5. **Persistence**: Mood changes persist across sessions (via Neo4j storage)

### Testing Checklist

- [ ] Create test NPC and PC in world
- [ ] Have PC talk to NPC with positive keywords ("thank you", "grateful")
- [ ] Verify mood shifts toward Friendly
- [ ] Have PC fail a challenge against NPC
- [ ] Verify mood shifts toward Hostile/Annoyed
- [ ] Check Neo4j for DISPOSITION_TOWARD edge with updated values
- [ ] Verify DM receives NpcMoodChanged notifications
- [ ] Test relationship level progression after multiple interactions

### Rollback Plan

If issues arise:
1. **Mood not updating**: Check `apply_interaction()` is being called via tracing logs
2. **Broadcast failing**: Session manager injection may be missing
3. **UI not updating**: Check `NpcMoodChanged` handler in session_message_handler.rs

The mood system is additive - existing manual DM control via `SetNpcMood` continues to work.

---

## Dependencies and Prerequisites

### Required Before Starting

1. MoodService already exists and is functional for manual mood setting
2. CharacterRepository has `get_mood_toward_pc()` and `set_mood_toward_pc()` methods
3. Protocol has `NpcMoodChanged` server message (already exists at messages.rs:739)

### No External Dependencies

All changes are internal to the engine and UI crates.

---

## Implementation Order

1. **Step 1**: Challenge resolution integration (foundational)
2. **Step 2**: Dialogue approval integration (builds on Step 1 patterns)
3. **Step 5**: Wire up dependencies (required before testing 1 & 2)
4. **Step 3**: Broadcast mechanism (enhances visibility)
5. **Step 4**: UI handler (completes the feedback loop)

---

## Notes on US-REGION-ITEMS (Already Complete)

For reference, the following was discovered to be already implemented during Sprint 4 analysis:

### Phase 5: LLM Context (Complete)
**Location**: `crates/engine-app/src/application/services/llm/prompt_builder.rs:96-108`

```rust
// Region items - visible objects in the area
if !context.region_items.is_empty() {
    prompt.push_str("\nVISIBLE ITEMS IN AREA:\n");
    for item in &context.region_items {
        // ... format item with type and description
    }
}
```

### Phase 6: UI Updates (Complete)
**Components**:
- `RegionItemsPanel` modal (`region_items_panel.rs`)
- `ActionPanel` shows items count badge
- `GameState.region_items` signal with optimistic removal
- Full pickup flow via `send_pickup_item()`

**Protocol**:
- `RegionItemData` struct (messages.rs:1021-1030)
- `SceneChanged` includes `region_items` field
- `PickupItem` client message
- `ItemPickedUp` server message

---

## Progress Log

| Date | Step | Status | Notes |
|------|------|--------|-------|
| 2025-12-26 | Plan created | Done | |
| | Step 1: Challenge integration | Not Started | |
| | Step 2: Dialogue integration | Not Started | |
| | Step 3: Broadcast mechanism | Not Started | |
| | Step 4: UI handler | Not Started | |
| | Step 5: Dependency wiring | Not Started | |
