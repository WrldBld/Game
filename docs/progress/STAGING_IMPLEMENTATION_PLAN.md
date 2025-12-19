# Staging System Implementation Plan

**Created:** 2025-12-19
**Status:** Ready to Implement
**Estimated Effort:** ~16.5 hours
**Phase:** 3 of Code Review Remediation

This plan implements the **Staging System** which manages NPC presence in regions with DM approval workflow, combining rule-based logic with LLM reasoning.

---

## Overview

The Staging System replaces the simple `PresenceService` with a comprehensive workflow:

1. **Player enters region** → Check for valid cached staging
2. **If no valid staging** → Generate proposals (rules + LLM) → Send to DM
3. **DM reviews** → Approves with optional modifications
4. **Staging cached** → NPCs appear for player
5. **Pre-staging** → DM can set up regions before players arrive

---

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Naming | "Staging" | Theatre terminology matching domain language |
| Integration | Background (async) | Player sees loading while DM approves |
| DM Approval | Always required | DM controls narrative pacing |
| Rule + LLM | Show both options | DM chooses, LLM can override rules |
| Persistence | Database (Neo4j) | History preserved, survives restarts |
| Multiple PCs | Share staging | Same staging within TTL |
| Expired staging | Trigger new approval | Treat as no staging |
| Regeneration | With DM guidance | Text field for additional context |
| Pre-staging UI | Location view tab | Dedicated interface |
| TTL config | Per-location setting | Different locations have different dynamics |

---

## Dependencies

### Part A: Dialogue Tracking Enhancement (Required First)

The Staging System's LLM needs context about recent dialogues with NPCs to make informed decisions. Currently:

- **Exists**: `StoryEvent::DialogueExchange` type in domain
- **Exists**: In-memory conversation history in sessions (30 turns)
- **Missing**: Consistent persistence of dialogue exchanges
- **Missing**: Method to query dialogues by NPC
- **Missing**: SPOKE_TO edge for quick lookup

---

## Task Breakdown

### Part A: Dialogue Tracking (2.5 hours)

| # | Task | Est. | Status | Files |
|---|------|------|--------|-------|
| A1 | Add `get_dialogues_with_npc` to StoryEventRepositoryPort | 20m | ⏳ | `ports/outbound/story_event_repository_port.rs` |
| A2 | Implement Cypher query in StoryEventRepository | 30m | ⏳ | `persistence/story_event_repository.rs` |
| A3 | Call `record_dialogue_exchange` in DMApprovalQueueService | 30m | ⏳ | `dm_approval_queue_service.rs` |
| A4 | Add SPOKE_TO edge creation/update in StoryEventRepository | 30m | ⏳ | `persistence/story_event_repository.rs` |
| A5 | Create StoryEventService method for dialogue summary | 30m | ⏳ | `story_event_service.rs` |

**Cypher for A2:**
```cypher
MATCH (w:World {id: $world_id})-[:HAS_STORY_EVENT]->(e:StoryEvent)
WHERE e.event_type = 'DialogueExchange' 
  AND e.npc_id = $npc_id
RETURN e
ORDER BY e.occurred_at DESC
LIMIT $limit
```

**Cypher for A4 (SPOKE_TO edge):**
```cypher
MERGE (pc:PlayerCharacter {id: $pc_id})-[r:SPOKE_TO]->(npc:Character {id: $npc_id})
SET r.last_dialogue_at = datetime(),
    r.last_topic = $topic,
    r.conversation_count = COALESCE(r.conversation_count, 0) + 1
```

---

### Part B: Staging Domain (2 hours)

| # | Task | Est. | Status | Files |
|---|------|------|--------|-------|
| B1 | Create Staging entity | 30m | ⏳ | `entities/staging.rs`, `entities/mod.rs` |
| B2 | Create StagingContext value objects | 30m | ⏳ | `value_objects/staging_context.rs` |
| B3 | Add Location entity fields (TTL, use_llm) | 20m | ⏳ | `entities/location.rs` |
| B4 | Update LocationRepository for new fields | 20m | ⏳ | `persistence/location_repository.rs` |
| B5 | Update Location HTTP routes for settings | 20m | ⏳ | `http/location_routes.rs` |

**B1 - Staging Entity:**
```rust
pub struct Staging {
    pub id: StagingId,
    pub region_id: RegionId,
    pub location_id: LocationId,
    pub world_id: WorldId,
    pub npcs: Vec<StagedNpc>,
    pub game_time: DateTime<Utc>,
    pub approved_at: DateTime<Utc>,
    pub ttl_hours: i32,
    pub approved_by: String,
    pub source: StagingSource,
    pub dm_guidance: Option<String>,
    pub is_active: bool,
}

pub struct StagedNpc {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub reasoning: String,
}

pub enum StagingSource {
    RuleBased,
    LlmBased,
    DmCustomized,
    PreStaged,
}
```

**B2 - StagingContext:**
```rust
pub struct StagingContext {
    pub region_name: String,
    pub region_description: String,
    pub location_name: String,
    pub time_of_day: TimeOfDay,
    pub time_display: String,
    pub active_events: Vec<ActiveEventContext>,
    pub npc_dialogues: Vec<NpcDialogueContext>,
    pub additional_context: HashMap<String, String>,
}

pub struct ActiveEventContext {
    pub event_name: String,
    pub description: String,
    pub relevance: String,
}

pub struct NpcDialogueContext {
    pub character_id: CharacterId,
    pub character_name: String,
    pub last_dialogue_summary: String,
    pub game_time_of_dialogue: String,
    pub mentioned_locations: Vec<String>,
}
```

**B3 - Location fields:**
```rust
pub struct Location {
    // ... existing fields ...
    pub presence_cache_ttl_hours: i32,  // Default: 1
    pub use_llm_presence: bool,          // Default: true
}
```

---

### Part C: Staging Infrastructure (3 hours)

| # | Task | Est. | Status | Files |
|---|------|------|--------|-------|
| C1 | Create StagingRepositoryPort trait | 20m | ⏳ | `ports/outbound/staging_repository_port.rs` |
| C2 | Implement StagingRepository (Neo4j) | 1h | ⏳ | `persistence/staging_repository.rs` |
| C3 | Add protocol messages | 30m | ⏳ | `protocol/messages.rs` |
| C4 | Wire StagingService to AppState | 30m | ⏳ | `state/mod.rs`, `state/game_services.rs` |

**C1 - Repository Port:**
```rust
#[async_trait]
pub trait StagingRepositoryPort: Send + Sync {
    async fn get_current(&self, region_id: RegionId) -> Result<Option<Staging>>;
    async fn get_history(&self, region_id: RegionId, limit: u32) -> Result<Vec<Staging>>;
    async fn save(&self, staging: Staging) -> Result<StagingId>;
    async fn is_valid(&self, staging_id: StagingId, current_game_time: &GameTime) -> Result<bool>;
    async fn invalidate_all(&self, region_id: RegionId) -> Result<()>;
}
```

**C2 - Neo4j Schema:**
```cypher
// Staging node
(:Staging {
    id: "uuid",
    region_id: "uuid",
    location_id: "uuid",
    world_id: "uuid",
    game_time: datetime,
    approved_at: datetime,
    ttl_hours: 3,
    approved_by: "client_id",
    source: "llm",
    dm_guidance: null,
    is_active: true
})

// Edges
(staging:Staging)-[:INCLUDES_NPC {is_present: true, reasoning: "..."}]->(character:Character)
(region:Region)-[:CURRENT_STAGING]->(staging:Staging)
(region:Region)-[:HAS_STAGING]->(staging:Staging)
```

**C3 - Protocol Messages:**

Server → Client (DM):
- `StagingApprovalRequired { request_id, region_id, region_name, location_name, game_time_display, previous_staging, rule_based_npcs, llm_based_npcs, default_ttl_hours, waiting_pcs }`

Client → Server (DM):
- `StagingApprovalResponse { request_id, approved_npcs, ttl_hours, source }`
- `StagingRegenerateRequest { request_id, guidance }`
- `PreStageRegion { region_id, npcs, ttl_hours }`

Server → Client (Player):
- `StagingPending { region_id }`
- `StagingReady { region_id, npcs_present }`

---

### Part D: Staging Service (2 hours)

| # | Task | Est. | Status | Files |
|---|------|------|--------|-------|
| D1 | Create StagingContextProvider | 45m | ⏳ | `services/staging_context.rs` |
| D2 | Create StagingService | 1.25h | ⏳ | `services/staging_service.rs` |

**D2 - StagingService methods:**
```rust
impl StagingService {
    /// Get current valid staging, or None if expired/missing
    pub async fn get_current_staging(&self, region_id: RegionId, game_time: &GameTime) 
        -> Result<Option<Staging>>;
    
    /// Generate staging proposal with both rule and LLM options
    pub async fn generate_proposal(
        &self, 
        region_id: RegionId, 
        game_time: &GameTime,
        dm_guidance: Option<&str>,
    ) -> Result<StagingProposal>;
    
    /// Approve and save staging
    pub async fn approve(&self, staging: Staging) -> Result<StagingId>;
    
    /// Pre-stage a region
    pub async fn pre_stage(
        &self,
        region_id: RegionId,
        npcs: Vec<StagedNpc>,
        ttl_hours: i32,
        dm_id: &str,
    ) -> Result<Staging>;
    
    /// Get previous staging (even if expired)
    pub async fn get_previous(&self, region_id: RegionId) -> Result<Option<Staging>>;
    
    /// Get staging history
    pub async fn get_history(&self, region_id: RegionId, limit: u32) -> Result<Vec<Staging>>;
}
```

**LLM Prompt Template:**
```
You are helping determine which NPCs are present in a location for a TTRPG game.

## Location
{region_name} ({location_name})
{region_description}
Time: {time_of_day} ({time_display})

## Rule-Based Suggestions
{rule_suggestions}

## Your Role
You may AGREE with or OVERRIDE the rules based on narrative considerations.
Consider: story reasons, interesting opportunities, conflicts, current context.

## Active Story Elements
{active_events}

## Recent NPC Interactions
{npc_dialogues}

## DM Guidance
{dm_guidance}

## Response Format
[{"name": "NPC Name", "is_present": true/false, "reasoning": "..."}]
```

---

### Part E: Engine Integration (1.5 hours)

| # | Task | Est. | Status | Files |
|---|------|------|--------|-------|
| E1 | Update WebSocket for staging flow | 1h | ⏳ | `websocket.rs` |
| E2 | Add staging to approval queue worker | 30m | ⏳ | `queue_workers.rs` |

**E1 - WebSocket MoveToRegion handler changes:**
```rust
// Current flow:
// 1. Validate move
// 2. Update PC position  
// 3. Calculate NPCs inline (simple rules)
// 4. Send SceneChanged

// New flow:
// 1. Validate move
// 2. Update PC position
// 3. Check StagingService.get_current_staging(region_id, game_time)
//    - If valid staging exists → Proceed to step 5
//    - If no staging:
//      a. Send StagingPending to player
//      b. Generate proposal via StagingService
//      c. Send StagingApprovalRequired to DM
//      d. Return (wait for DM approval)
// 4. On DM approval (StagingApprovalResponse):
//    a. StagingService.approve(staging)
//    b. Send StagingReady to player
// 5. Send SceneChanged with npcs_present from staging
```

---

### Part F: Player UI (4.5 hours)

| # | Task | Est. | Status | Files |
|---|------|------|--------|-------|
| F1 | Add staging state to GameState | 20m | ⏳ | `state/game_state.rs` |
| F2 | Add staging message handlers | 30m | ⏳ | `handlers/session_message_handler.rs` |
| F3 | Create StagingApproval popup component | 1.5h | ⏳ | `dm_panel/staging_approval.rs` |
| F4 | Create PreStaging UI (location tab) | 1h | ⏳ | `dm_panel/location_staging.rs` |
| F5 | Add StagingPending overlay to PC view | 30m | ⏳ | `views/pc_view.rs` |
| F6 | Update Location editor for TTL settings | 30m | ⏳ | `creator/location_editor.rs` |

**F1 - State signals:**
```rust
pub struct GameState {
    // ... existing ...
    pub staging_pending: Signal<Option<String>>,  // region_id if pending
    pub pending_staging_approval: Signal<Option<StagingApprovalData>>,
}
```

**F3 - StagingApproval component structure:**
- Header with region/location/time info
- Waiting PCs list
- Previous staging section (if exists)
- Side-by-side rule-based and LLM-based NPC lists
- Manual toggle section for customization
- TTL dropdown
- Regenerate with guidance section
- Approve button

**F5 - StagingPending overlay:**
```rust
if let Some(_region_id) = staging_pending.read().as_ref() {
    div {
        class: "fixed inset-0 bg-black/70 z-50 flex items-center justify-center",
        div {
            class: "bg-dark-surface rounded-xl p-8 text-center",
            div { class: "text-4xl mb-4", "🎭" }
            div { class: "text-xl text-gray-300", "Setting the scene..." }
            // Spinner
        }
    }
}
```

---

### Part G: Finalization (1 hour)

| # | Task | Est. | Status | Files |
|---|------|------|--------|-------|
| G1 | Integration testing | 30m | ⏳ | Manual testing |
| G2 | Remove/deprecate old PresenceService | 15m | ⏳ | `presence_service.rs` |
| G3 | Update CODE_REVIEW_REMEDIATION_PLAN.md | 15m | ⏳ | `docs/progress/` |

---

## Implementation Order

1. **Part A** (Dialogue Tracking) - Required dependency for LLM context
2. **Part B** (Domain) - Foundation entities and value objects
3. **Part C** (Infrastructure) - Repository, protocol messages
4. **Part D** (Service) - Core business logic
5. **Part E** (Engine Integration) - WebSocket flow changes
6. **Part F** (Player UI) - All UI components
7. **Part G** (Finalization) - Testing, cleanup, documentation

---

## Testing Checklist

### Part A Verification
- [ ] Dialogue exchange creates StoryEvent in Neo4j
- [ ] `get_dialogues_with_npc` returns correct events
- [ ] SPOKE_TO edge created/updated on dialogue

### Part B Verification
- [ ] Staging entity serializes to/from Neo4j correctly
- [ ] Location TTL fields persist correctly

### Part C Verification
- [ ] StagingRepository CRUD operations work
- [ ] Protocol messages serialize correctly

### Part D Verification
- [ ] Rule-based suggestions match expected logic
- [ ] LLM receives correct context
- [ ] Staging cache respects TTL

### Part E Verification
- [ ] Player entering unstaged region triggers approval flow
- [ ] Pre-staged region loads immediately
- [ ] Expired staging triggers new approval

### Part F Verification
- [ ] DM sees approval popup with both options
- [ ] Player sees loading state during pending
- [ ] Pre-staging UI shows correct region statuses
- [ ] Location settings save correctly

---

## Rollback Plan

If issues arise during implementation:

1. **Part A issues**: Dialogue tracking is additive; can skip if problematic
2. **Part B-D issues**: Keep old PresenceService as fallback
3. **Part E issues**: Feature flag to use old inline presence logic
4. **Part F issues**: Disable staging UI, use fallback approval

---

## Related Documentation

- [Staging System](../systems/staging-system.md) - System specification
- [NPC System](../systems/npc-system.md) - NPC-Region relationships (input to staging)
- [Dialogue System](../systems/dialogue-system.md) - Dialogue tracking enhancements
- [CODE_REVIEW_REMEDIATION_PLAN.md](./CODE_REVIEW_REMEDIATION_PLAN.md) - Overall remediation context

---

## Progress Log

| Date | Part | Task | Status | Notes |
|------|------|------|--------|-------|
| 2025-12-19 | - | Plan created | Done | |
