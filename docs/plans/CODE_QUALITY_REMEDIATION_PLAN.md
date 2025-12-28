# Code Quality Remediation Plan

**Status**: ACTIVE  
**Created**: 2025-12-28  
**Last Updated**: 2025-12-28 (Validated and corrected after peer review)  
**Goal**: Achieve a clean, production-ready codebase with zero technical debt  
**Estimated Total Effort**: 40-50 hours

---

## Validation Notes (2025-12-28)

This plan was validated by two peer review agents. Key corrections applied:

### First Review
1. **Phase 1.3 REMOVED** - The `staging_service.rs:535` unwrap is inside `#[cfg(test)]` (test code), not production
2. **God trait method counts corrected**:
   - CharacterRepositoryPort: ~35 → **42 methods** (verified)
   - StoryEventRepositoryPort: ~35 → **34 methods** (verified)
   - ChallengeRepositoryPort: ~25 → **31 methods** (verified)
3. **Swallowed error count verified** - 43 instances confirmed in `services/` directory
4. **Phase 3.5 warning added** - Splitting god traits will break test compilation until Phase 7
5. **HTTP timeout/client claims verified** - Confirmed no timeouts, 11 instances of per-request client creation

### Second Review - Additional Issues Found
6. **File I/O in application layer** - 5 instances of `tokio::fs` in engine-app (architecture violation)
7. **Environment access in application layer** - 2 instances of `std::env` in prompt_template_service.rs
8. **Shadow variable bug** - `completed_count` in generation_service.rs is not unused, it has a shadow bug
9. **Protocol forward compatibility** - No `#[serde(other)]` on any enum (breaks client on new variants)
10. **Test compilation root cause** - Specific: staging_service_adapter.rs stubs return wrong error types

### Known Limitations (Not in Scope)
- **Authentication**: X-User-Id header is spoofable - intentional for MVP, production auth is separate work
- **Rate limiting**: RateLimitExceeded defined but unused - feature work, not remediation
- **Reconnection logic**: Reconnecting state unused - feature work, not remediation
- **Protocol versioning**: No version field - would be breaking change, separate effort

---

## Executive Summary

Two comprehensive code reviews identified issues across the WrldBldr codebase. This plan consolidates all findings into a prioritized remediation roadmap organized by severity and effort.

### Issue Summary

| Severity | Count | Categories |
|----------|-------|------------|
| Critical | 2 | Production panic risk, protocol forward compatibility |
| High | ~65 | Swallowed errors (43), god traits (3), I/O in app layer (7), architecture gaps |
| Medium | ~80 | Dead code, missing derives, config issues (hardcoded IPs), shadow bugs |
| Low | ~100+ | Unused variables, documentation, naming |

### Progress Tracking

| Phase | Description | Status | Completion |
|-------|-------------|--------|------------|
| Phase 1 | Critical Fixes | Pending | 0% |
| Phase 2 | High Priority | Pending | 0% |
| Phase 3 | Architecture Completion | Pending | 0% |
| Phase 4 | Dead Code Cleanup | Pending | 0% |
| Phase 5 | Domain Layer Polish | Pending | 0% |
| Phase 6 | Protocol Layer Polish | Pending | 0% |
| Phase 7 | Test Infrastructure | Pending | 0% |
| Phase 8 | Documentation | Pending | 0% |

---

## Phase 1: Critical Fixes (1 hour)

**Priority**: IMMEDIATE - These can cause production crashes or security issues

### 1.1 Fix Production Panic Risks

**Files**:
- `crates/player-ui/src/presentation/components/creator/motivations_tab.rs`

**Issue**: Lines 498 and 500 use `.unwrap()` on `strip_prefix()` which can panic if the guard condition doesn't match.

**Risk Level**: Low-Medium (guarded by `starts_with()` check, but still a code smell)

**Current Code** (lines 496-502):
```rust
let (actor_id, actor_type) = if target_str.starts_with("npc:") {
    (target_str.strip_prefix("npc:").unwrap().to_string(), ActorTypeData::Npc)
} else if target_str.starts_with("pc:") {
    (target_str.strip_prefix("pc:").unwrap().to_string(), ActorTypeData::Pc)
} else {
    // ...
}
```

**Fix**: Use `if let Some()` pattern:
```rust
let (actor_id, actor_type) = if let Some(id) = target_str.strip_prefix("npc:") {
    (id.to_string(), ActorTypeData::Npc)
} else if let Some(id) = target_str.strip_prefix("pc:") {
    (id.to_string(), ActorTypeData::Pc)
} else {
    // ...
}
```

| Task | Status |
|------|--------|
| [ ] Fix motivations_tab.rs:498 unwrap | Pending |
| [ ] Fix motivations_tab.rs:500 unwrap | Pending |
| [ ] Verify no other production unwrap() calls exist | Pending |

---

### 1.2 Replace Hardcoded Internal IP Addresses

**File**: `crates/engine-adapters/src/infrastructure/config.rs:80-84`

**Issue**: Hardcoded internal/VPN IP address `10.8.0.6` in default configuration.

**Current Code**:
```rust
ollama_base_url: env::var("OLLAMA_BASE_URL")
    .unwrap_or_else(|_| "http://10.8.0.6:11434/v1".to_string()),
comfyui_base_url: env::var("COMFYUI_BASE_URL")
    .unwrap_or_else(|_| "http://10.8.0.6:8188".to_string()),
```

**Fix**:
```rust
ollama_base_url: env::var("OLLAMA_BASE_URL")
    .unwrap_or_else(|_| "http://localhost:11434/v1".to_string()),
comfyui_base_url: env::var("COMFYUI_BASE_URL")
    .unwrap_or_else(|_| "http://localhost:8188".to_string()),
```

| Task | Status |
|------|--------|
| [ ] Replace 10.8.0.6 with localhost in config.rs | Pending |
| [ ] Search for other hardcoded IPs in codebase | Pending |

---

### 1.3 Add Protocol Forward Compatibility

**Priority**: CRITICAL - Without this, adding any new enum variant breaks all existing clients.

**Issue**: No protocol enums have `#[serde(other)]` catch-all variants. When we add a new variant to any enum (e.g., `ServerMessage`), older clients that don't know about it will fail to deserialize the entire message.

**Files to update**:
- `crates/protocol/src/messages.rs` - ClientMessage, ServerMessage
- `crates/protocol/src/requests.rs` - RequestPayload
- `crates/protocol/src/responses.rs` - ResponseResult
- Other enums as needed (~15 total)

**Pattern to apply**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    // ... existing variants ...
    
    /// Unknown message type for forward compatibility
    /// Older clients will deserialize unknown variants as this
    #[serde(other)]
    Unknown,
}
```

**Note**: This requires adding a catch-all variant to each enum. Consider whether `Unknown` should carry the raw JSON for debugging.

| Task | Status |
|------|--------|
| [ ] Add #[serde(other)] Unknown to ServerMessage | Pending |
| [ ] Add #[serde(other)] Unknown to ClientMessage | Pending |
| [ ] Add #[serde(other)] Unknown to RequestPayload | Pending |
| [ ] Add #[serde(other)] Unknown to ResponseResult | Pending |
| [ ] Audit remaining protocol enums | Pending |
| [ ] Add handling for Unknown variants in message processors | Pending |

---

### ~~1.4 Fix Production unwrap() in Staging Service~~ REMOVED

**Status**: ~~INVALID~~ - This item was removed after validation.

**Reason**: The `staging_service.rs:535` unwrap is inside `#[cfg(test)] mod tests`, not production code. Test code unwraps are acceptable.

---

## Phase 2: High Priority Error Handling (4-6 hours)

**Priority**: HIGH - Silent failures in production

### 2.1 Add Logging to Swallowed Errors in Queue Workers

**Issue**: 43 instances of `let _ =` silently discarding results in background workers.

**Files to fix**:

| File | Lines | Pattern |
|------|-------|---------|
| `llm_queue_service.rs` | 107, 180, 404, 408, 414, 423, 464, 470, 484, 489, 495 | Queue operations |
| `asset_generation_queue_service.rs` | 144, 158, 204, 233, 292 | Asset failures |
| `generation_service.rs` | 170, 300, 316 | Event drops |

**Pattern to apply**:
```rust
// Before:
let _ = self.generation_event_tx.send(event);

// After:
if let Err(e) = self.generation_event_tx.send(event) {
    tracing::warn!("Failed to send generation event: {}", e);
}
```

| Task | Status |
|------|--------|
| [ ] Fix llm_queue_service.rs (11 instances) | Pending |
| [ ] Fix asset_generation_queue_service.rs (5 instances) | Pending |
| [ ] Fix generation_service.rs (3 instances) | Pending |
| [ ] Audit remaining `let _ =` patterns for intentionality | Pending |
| [ ] Add comments to intentional `let _ =` patterns | Pending |

---

### 2.2 Add HTTP Request Timeouts

**Issue**: HTTP requests have no timeout, can hang indefinitely.

**Files to fix**:

| File | Issue |
|------|-------|
| `player-adapters/src/infrastructure/http_client.rs` | No timeout on requests |
| `engine-adapters/src/infrastructure/ollama.rs:53-58` | LLM calls no timeout |

**Fix for http_client.rs**:
```rust
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .unwrap_or_else(|_| reqwest::Client::new());
```

**Fix for ollama.rs** (longer timeout for LLM):
```rust
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(120))
    .build()?;
```

| Task | Status |
|------|--------|
| [ ] Add 30s timeout to http_client.rs | Pending |
| [ ] Add 120s timeout to ollama.rs | Pending |
| [ ] Add timeout to comfyui.rs if missing | Pending |

---

### 2.3 Fix HTTP Client Per-Request Creation

**File**: `crates/player-adapters/src/infrastructure/http_client.rs`

**Issue**: Creates new `reqwest::Client` for every request, preventing connection reuse.

**Fix**: Use shared static client:
```rust
use once_cell::sync::Lazy;

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});
```

| Task | Status |
|------|--------|
| [ ] Implement shared client in http_client.rs | Pending |

---

## Phase 3: Architecture Completion (8-10 hours)

**Priority**: HIGH - Complete hexagonal architecture gaps

### 3.0 Move I/O Operations Out of Application Layer

**Issue**: Application layer has direct file system and environment variable access, violating hexagonal architecture.

#### 3.0.1 File System I/O Violations

**Files with `tokio::fs` in application layer**:

| File | Lines | Operations |
|------|-------|------------|
| `generation_service.rs` | 353, 365, 403 | create_dir_all, write, read_to_string |
| `asset_generation_queue_service.rs` | 231, 245 | create_dir_all, write |

**Fix**: Create `FileStoragePort` in engine-ports:
```rust
#[async_trait]
pub trait FileStoragePort: Send + Sync {
    async fn create_dir_all(&self, path: &Path) -> Result<()>;
    async fn write(&self, path: &Path, data: &[u8]) -> Result<()>;
    async fn read_to_string(&self, path: &Path) -> Result<String>;
}
```

| Task | Status |
|------|--------|
| [ ] Create FileStoragePort trait in engine-ports | Pending |
| [ ] Create TokioFileStorageAdapter in engine-adapters | Pending |
| [ ] Update generation_service.rs to use FileStoragePort | Pending |
| [ ] Update asset_generation_queue_service.rs to use FileStoragePort | Pending |
| [ ] Wire up in runner | Pending |

#### 3.0.2 Environment Variable Access Violations

**Files with `std::env` in application layer**:

| File | Lines | Usage |
|------|-------|-------|
| `prompt_template_service.rs` | 222, 268 | Reading template overrides from env |

**Fix**: Inject configuration through constructor or config port.

| Task | Status |
|------|--------|
| [ ] Add template override config to PromptTemplateService constructor | Pending |
| [ ] Move env::var calls to adapter/runner layer | Pending |

---

### 3.1 Add Missing Challenge DTOs

**Issue**: `challenge_service.rs` directly uses protocol types instead of app-layer DTOs.

**File to create/update**: `crates/player-app/src/application/dto/requests.rs`

**DTOs to add**:
```rust
pub struct CreateChallengeRequest {
    pub name: String,
    pub description: Option<String>,
    pub challenge_type: String,
    pub difficulty: Option<String>,
    pub skill_id: Option<String>,
    // ... remaining fields from CreateChallengeData
}

pub struct UpdateChallengeRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    // ... remaining fields from UpdateChallengeData
}

impl From<CreateChallengeRequest> for wrldbldr_protocol::CreateChallengeData {
    fn from(req: CreateChallengeRequest) -> Self {
        Self {
            name: req.name,
            description: req.description,
            // ...
        }
    }
}
```

**File to update**: `crates/player-app/src/application/services/challenge_service.rs`

| Task | Status |
|------|--------|
| [ ] Add CreateChallengeRequest to requests.rs | Pending |
| [ ] Add UpdateChallengeRequest to requests.rs | Pending |
| [ ] Add From impls for both | Pending |
| [ ] Update challenge_service.rs to use new DTOs | Pending |
| [ ] Remove direct protocol imports from challenge_service.rs | Pending |

---

### 3.2 Consolidate Duplicate SuggestionContext DTO

**Issue**: Two definitions of SuggestionContext exist.

**Files**:
- `crates/player-app/src/application/dto/requests.rs:109-128`
- `crates/player-app/src/application/services/suggestion_service.rs:35-70`

**Fix**: Keep the one in requests.rs, update suggestion_service.rs to import it.

| Task | Status |
|------|--------|
| [ ] Verify requests.rs version is complete | Pending |
| [ ] Update suggestion_service.rs to use requests.rs version | Pending |
| [ ] Remove duplicate from suggestion_service.rs | Pending |

---

### 3.3 Document Port Placement Decision

**Issue**: Infrastructure ports remain in engine-app instead of engine-ports as originally planned.

**Ports affected**:
- `WorldStatePort` (scene.rs:227)
- `ConnectionManagerPort` (connection.rs:117)
- `StagingStatePort` (movement.rs:130)
- `StagingStateExtPort` (staging.rs)

**Decision**: These ports depend on use-case DTOs in engine-app. Moving them would create circular dependencies. Document as intentional.

**Files to update**:
- `crates/engine-app/src/application/use_cases/scene.rs`
- `crates/engine-app/src/application/use_cases/connection.rs`
- `crates/engine-app/src/application/use_cases/movement.rs`
- `crates/engine-app/src/application/use_cases/staging.rs`

**Comment to add**:
```rust
// ARCHITECTURE NOTE: This port is defined in engine-app rather than engine-ports
// because it depends on use-case-specific DTOs (WorldStateData, etc.) that are
// defined in this crate. Moving to engine-ports would create circular dependencies.
// This is an approved deviation from the standard hexagonal port placement.
```

| Task | Status |
|------|--------|
| [ ] Add architecture comment to WorldStatePort | Pending |
| [ ] Add architecture comment to ConnectionManagerPort | Pending |
| [ ] Add architecture comment to StagingStatePort | Pending |
| [ ] Add architecture comment to StagingStateExtPort | Pending |
| [ ] Update HEXAGONAL_GAP_REMEDIATION_PLAN.md to reflect decision | Pending |

---

### 3.4 Document Protocol Imports in Ports

**Issue**: GameConnectionPort and RequestHandler use protocol types directly.

**Files**:
- `crates/engine-ports/src/inbound/request_handler.rs:35-38`
- `crates/player-ports/src/outbound/game_connection_port.rs:17`

**Decision**: These are boundary ports where protocol types are appropriate. Document as approved exception.

**Comment to add**:
```rust
// ARCHITECTURE EXCEPTION: [APPROVED 2025-12-28]
// This port uses protocol types directly because it defines the primary
// engine-player communication boundary. The protocol crate exists specifically
// to share types across this boundary. Creating domain-level duplicates would
// add complexity without benefit.
```

| Task | Status |
|------|--------|
| [ ] Add exception comment to request_handler.rs | Pending |
| [ ] Add exception comment to game_connection_port.rs | Pending |

---

### 3.5 Split God Traits

**Issue**: Several repository traits are too large (35+ methods each).

> **WARNING**: Splitting these traits will break test compilation until Phase 7 (Test Infrastructure) updates the mock implementations. Consider doing this as the last item in Phase 3, or as a separate PR that includes mock updates.

**Traits to split**:

#### CharacterRepositoryPort (42 methods - VERIFIED)

**Current**: `engine-ports/src/outbound/repository_port.rs:94-389`

**Split into**:
- `CharacterCrudPort` - Basic CRUD (5 methods)
- `CharacterWantsPort` - Want/motivation operations (6 methods)
- `CharacterActantialPort` - Actantial views (5 methods)
- `CharacterInventoryPort` - Inventory operations (5 methods)
- `CharacterLocationPort` - Location relationships (8 methods)
- `NpcDispositionPort` - NPC disposition (7 methods)
- `CharacterRegionPort` - Region relationships (4 methods)

#### StoryEventRepositoryPort (34 methods - VERIFIED)

**Current**: `engine-ports/src/outbound/repository_port.rs:1184-1371`

**Split into**:
- `StoryEventCrudPort` - CRUD and search
- `StoryEventRelationshipPort` - Edge methods
- `DialogueHistoryPort` - Dialogue-specific methods

#### ChallengeRepositoryPort (31 methods - VERIFIED)

**Current**: `engine-ports/src/outbound/repository_port.rs:1007-1183`

**Split into**:
- `ChallengeCrudPort` - Basic CRUD
- `ChallengeSkillPort` - Skill relationships
- `ChallengeAvailabilityPort` - Availability checks

| Task | Status |
|------|--------|
| [ ] Create new trait files in engine-ports/outbound/ | Pending |
| [ ] Split CharacterRepositoryPort (42 methods) | Pending |
| [ ] Split StoryEventRepositoryPort (34 methods) | Pending |
| [ ] Split ChallengeRepositoryPort (31 methods) | Pending |
| [ ] Update all trait implementations in adapters | Pending |
| [ ] Update all trait usages in app layer | Pending |
| [ ] Update mock implementations (coordinate with Phase 7) | Pending |
| [ ] Verify compilation | Pending |

**Note**: This is a significant refactor (~107 methods total). Consider doing in a separate PR that includes mock updates to avoid breaking test compilation.

---

## Phase 4: Dead Code Cleanup (3-4 hours)

**Priority**: MEDIUM - Code hygiene

### 4.1 Remove Unused Structs

| File | Struct | Action |
|------|--------|--------|
| `domain/entities/challenge.rs:531` | `ComplexChallengeSettings` | DELETE |
| `engine-app/dto/narrative_event.rs:130` | `NarrativeEventDetailResponseDto` | DELETE |
| `engine-app/dto/narrative_event.rs:162` | `ChainMembershipDto` | DELETE |
| `engine-app/dto/narrative_event.rs:180` | `FeaturedNpcDto` | DELETE |
| `engine-app/dto/narrative_event.rs:196` | `fn new()` | DELETE |

| Task | Status |
|------|--------|
| [ ] Delete ComplexChallengeSettings | Pending |
| [ ] Delete NarrativeEventDetailResponseDto cluster (~100 lines) | Pending |
| [ ] Verify no references remain | Pending |

---

### 4.2 Remove or Use Unused Fields

| File | Field | Action |
|------|-------|--------|
| `actantial_context_service.rs:198` | `item_repo` | DELETE (injected but unused) |
| `scene_resolution_service.rs:59` | `character_repository` | DELETE |
| `trigger_evaluation_service.rs:200` | `challenge_repo` | DELETE |
| `trigger_evaluation_service.rs:201` | `character_repo` | DELETE |

**Broadcast fields** (4 use cases) - Requires decision:

| File | Field | Decision Needed |
|------|-------|-----------------|
| `connection.rs:224` | `broadcast` | IMPLEMENT or DELETE |
| `observation.rs:137` | `broadcast` | IMPLEMENT or DELETE |
| `player_action.rs:116` | `broadcast` | IMPLEMENT or DELETE |
| `scene.rs:291` | `broadcast` | IMPLEMENT or DELETE |

| Task | Status |
|------|--------|
| [ ] Delete item_repo from actantial_context_service.rs | Pending |
| [ ] Delete character_repository from scene_resolution_service.rs | Pending |
| [ ] Delete challenge_repo, character_repo from trigger_evaluation_service.rs | Pending |

---

### 4.3 Fix Shadow Variable Bug in generation_service.rs

**Issue**: The `completed_count` field in `BatchTracker` struct is never used because a local variable with the same name shadows it.

**Location**: `crates/engine-app/src/application/services/generation_service.rs`

**Lines**:
- Line 116: `completed_count: u8` - field in BatchTracker struct
- Line 251: `let mut completed_count = 0u8;` - local variable shadows the field

**Current Code**:
```rust
struct BatchTracker {
    batch: GenerationBatch,
    prompt_ids: Vec<String>,
    completed_count: u8,  // Never used - shadowed by local variable
}

// Later in check_batch_progress():
let mut completed_count = 0u8;  // Shadows the field!
for prompt_id in &prompt_ids {
    // ...
    completed_count += 1;  // Increments local, not field
}
```

**Fix Options**:
1. Use the field: Replace local with `tracker.completed_count` 
2. Remove the field: If batch-level tracking isn't needed

| Task | Status |
|------|--------|
| [ ] Determine if batch-level completed_count tracking is needed | Pending |
| [ ] Either use the field or remove it | Pending |
| [ ] Decide on broadcast fields (implement or delete) | Pending |
| [ ] Implement or delete broadcast fields based on decision | Pending |

---

### 4.4 Remove Unused Constants and Imports

| File | Item | Action |
|------|------|--------|
| `llm_queue_service.rs:27` | `PRIORITY_HIGH` | DELETE |
| `disposition.rs:30` | `use uuid::Uuid` | DELETE |
| `state/use_cases.rs:45` | `ApprovalQueuePort` | DELETE |
| `export_routes.rs:13` | `WorldService` | DELETE |

| Task | Status |
|------|--------|
| [ ] Run `cargo fix --workspace --allow-dirty` | Pending |
| [ ] Manually fix remaining unused imports | Pending |
| [ ] Delete PRIORITY_HIGH constant | Pending |

---

### 4.5 Address #[allow(dead_code)] Suppressions

**Suspicious suppressions to audit**:

| File | Item | Action |
|------|------|--------|
| `handlers/common.rs:103` | `parse_goal_id` | DELETE or USE |
| `handlers/common.rs:110` | `parse_want_id` | DELETE or USE |
| `handlers/common.rs:123` | `parse_relationship_id` | DELETE or USE |
| `handlers/common.rs:130` | `parse_story_event_id` | DELETE or USE |
| `websocket/converters.rs:54` | `to_domain_visibility` | DELETE or USE |
| `websocket/converters.rs:64` | `from_domain_visibility` | DELETE or USE |
| `websocket/converters.rs:74` | `to_domain_role` | DELETE or USE |
| `services.rs:302` | `apply_generation_read_state` | IMPLEMENT or DELETE |

| Task | Status |
|------|--------|
| [ ] Audit each #[allow(dead_code)] | Pending |
| [ ] Delete truly dead code | Pending |
| [ ] Remove #[allow(dead_code)] from used code | Pending |

---

### 4.6 Fix Unused Variables in UI Layer

| File | Variable | Fix |
|------|----------|-----|
| `location_preview_modal.rs:40` | `world_id` | Prefix with `_` |
| `edit_character_modal.rs:63` | `desc_val` | Prefix with `_` |
| `skills_panel.rs:254` | `world_id_for_delete` | Prefix with `_` |
| `skills_panel.rs:275` | `world_id` | Prefix with `_` |
| `skills_panel.rs:546` | `world_id` | Prefix with `_` |
| `pc_creation.rs:166` | `desc_val` | Prefix with `_` |
| `pc_creation.rs:169` | `session_id` | Prefix with `_` |
| `world_select.rs:66` | `user_id` | Prefix with `_` |

| Task | Status |
|------|--------|
| [ ] Fix all unused UI variables | Pending |

---

## Phase 5: Domain Layer Polish (2-3 hours)

**Priority**: MEDIUM - Serialization and type safety

### 5.1 Add Serde Derives to Entities

**Issue**: Core entities lack `Serialize, Deserialize` derives.

**Entities to update**:
- `Character`
- `World`
- `Location`
- `Scene`
- `Challenge`
- `Item`
- `PlayerCharacter`
- `StoryEvent`
- `NarrativeEvent`

**Pattern**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    // ...
}
```

| Task | Status |
|------|--------|
| [ ] Add serde derives to Character | Pending |
| [ ] Add serde derives to World | Pending |
| [ ] Add serde derives to Location | Pending |
| [ ] Add serde derives to Scene | Pending |
| [ ] Add serde derives to Challenge | Pending |
| [ ] Add serde derives to Item | Pending |
| [ ] Add serde derives to PlayerCharacter | Pending |
| [ ] Add serde derives to StoryEvent | Pending |
| [ ] Add serde derives to NarrativeEvent | Pending |

---

### 5.2 Add Serde to ID Types

**File**: `crates/domain/src/ids.rs`

**Issue**: Macro-generated ID types lack serde derives.

**Current macro**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct $name(Uuid);
```

**Fix**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct $name(Uuid);
```

| Task | Status |
|------|--------|
| [ ] Update define_id! macro to include serde | Pending |
| [ ] Verify all ID types serialize correctly | Pending |

---

### 5.3 Move Environment I/O Out of Domain

**File**: `crates/domain/src/value_objects/settings.rs:157-196`

**Issue**: `AppSettings::from_env()` reads environment variables in domain layer.

**Fix**: Move to adapters layer.

| Task | Status |
|------|--------|
| [ ] Create settings adapter in engine-adapters | Pending |
| [ ] Move from_env() to adapter | Pending |
| [ ] Update domain to only define AppSettings struct | Pending |
| [ ] Update all callers to use adapter | Pending |

---

## Phase 6: Protocol Layer Polish (2-3 hours)

**Priority**: MEDIUM - Wire format safety

### 6.1 Add Documentation to Protocol Imports

**File**: `crates/protocol/src/dto.rs:10-11`

**Issue**: Domain imports not documented as exception.

**Fix**: Add comment:
```rust
// ARCHITECTURE EXCEPTION: [APPROVED 2025-12-28]
// Uses domain ID types for DTO conversion methods only.
// Wire format uses raw Uuid; these imports enable to_domain() conversion.
use wrldbldr_domain::value_objects::{DispositionLevel, NpcDispositionState, RelationshipLevel};
use wrldbldr_domain::{CharacterId, PlayerCharacterId};
```

| Task | Status |
|------|--------|
| [ ] Add exception comment to dto.rs | Pending |

---

### 6.2 Add Serde to RequestError

**File**: `crates/protocol/src/responses.rs:139-151`

**Issue**: `RequestError` lacks serde derives.

**Fix**:
```rust
/// Client-side request errors (not serialized over wire)
///
/// These errors occur locally on the client and are never transmitted.
/// If wire transmission is needed in future, add Serialize/Deserialize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestError {
    // ...
}
```

| Task | Status |
|------|--------|
| [ ] Add documentation explaining why no serde | Pending |
| [ ] OR add serde derives if wire transmission needed | Pending |

---

### 6.3 Document Versioning Strategy

**File**: `crates/protocol/src/messages.rs`

**Issue**: Large enums (70+ variants) without versioning strategy.

**Fix**: Add module-level documentation:
```rust
//! ## Versioning Policy
//!
//! - New variants can be added at the end (forward compatible)
//! - Removing variants requires major version bump
//! - Renaming variants is a breaking change
//! - Consider `#[serde(other)]` catch-all for unknown variants in future
```

| Task | Status |
|------|--------|
| [ ] Add versioning documentation to messages.rs | Pending |
| [ ] Add versioning documentation to requests.rs | Pending |

---

### 6.4 Standardize ID Types

**Issue**: Inconsistent use of `String` vs `Uuid` for IDs.

**Pattern to follow**:
- Entity IDs: Use `Uuid`
- Correlation/Request IDs: Use `String`

| Task | Status |
|------|--------|
| [ ] Audit all ID fields in protocol types | Pending |
| [ ] Standardize to Uuid where appropriate | Pending |
| [ ] Document the pattern in protocol/src/lib.rs | Pending |

---

## Phase 7: Test Infrastructure (8-12 hours)

**Priority**: MEDIUM - Enable quality verification

### 7.1 Fix Test Compilation

**Issue**: Test suite fails to compile with ~36 errors. Root cause identified.

**Root Cause**: `crates/engine-adapters/src/infrastructure/ports/staging_service_adapter.rs:244-335`

The stub implementations return wrong error types:
- Stubs return `Result<(), String>` 
- Traits expect `Result<(), anyhow::Error>`

**Specific errors**:
- Missing trait methods: `is_valid`, `get_staged_npcs`
- Type mismatches: `Result<(), String>` vs `Result<(), anyhow::Error>`
- Missing `futures` crate import

**Additional issues**:
- Duplicated mocks: `MockPromptTemplateRepository` in 3 files, `MockLlm` in 2 files
- Empty tests: `disposition_service.rs:283`, `actantial_context_service.rs:652` have `assert!(true)`

| Task | Status |
|------|--------|
| [ ] Fix staging_service_adapter.rs stub error types (root cause) | Pending |
| [ ] Add missing trait methods to stubs | Pending |
| [ ] Add futures import | Pending |
| [ ] Run `cargo test --workspace` and fix remaining errors | Pending |
| [ ] Consolidate duplicated mock implementations | Pending |
| [ ] Fix or remove empty tests | Pending |
| [ ] Verify all tests compile | Pending |

---

### 7.2 Add Protocol Serialization Tests

**Issue**: Zero tests for protocol message serialization.

**File to create**: `crates/protocol/src/lib.rs` (add tests module)

**Tests to add**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn client_message_roundtrip() {
        let msg = ClientMessage::Ping;
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }
    
    #[test]
    fn server_message_roundtrip() {
        // Test each major variant
    }
    
    #[test]
    fn request_payload_roundtrip() {
        // Test each major variant
    }
}
```

| Task | Status |
|------|--------|
| [ ] Add ClientMessage roundtrip tests | Pending |
| [ ] Add ServerMessage roundtrip tests | Pending |
| [ ] Add RequestPayload roundtrip tests | Pending |
| [ ] Add ResponseResult roundtrip tests | Pending |

---

### 7.3 Add #[automock] to All Ports

**Issue**: Most ports lack mock implementations.

**Files to update**:
- All traits in `engine-ports/src/outbound/`
- All traits in `player-ports/src/outbound/`

**Pattern**:
```rust
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SomePort: Send + Sync {
    // ...
}
```

| Task | Status |
|------|--------|
| [ ] Add mockall dependency if missing | Pending |
| [ ] Add #[automock] to all engine-ports traits | Pending |
| [ ] Add #[automock] to all player-ports traits | Pending |

---

### 7.4 Create Entity Test Factories

**File to create**: `crates/domain/src/testing/mod.rs`

**Content**:
```rust
//! Test factories for domain entities
//! 
//! Only compiled in test mode.

#[cfg(test)]
pub mod factories {
    use super::*;
    
    pub fn test_world() -> World {
        World::new("Test World".to_string(), None, None)
    }
    
    pub fn test_character() -> Character {
        Character::new(/* ... */)
    }
    
    // ... more factories
}
```

| Task | Status |
|------|--------|
| [ ] Create testing module in domain | Pending |
| [ ] Add factory for each major entity | Pending |
| [ ] Add factory customization builders | Pending |

---

## Phase 8: Documentation (2-3 hours)

**Priority**: LOW - Completeness

### 8.1 Add Layer Structure Diagram to CLAUDE.md

**File**: `CLAUDE.md`

**Add visual diagram**:
```
┌─────────────────────────────────────────────────────────────┐
│                         RUNNERS                              │
│              Composition root, wires everything              │
├─────────────────────────────────────────────────────────────┤
│                       PRESENTATION                           │
│                   UI components (player-ui)                  │
├─────────────────────────────────────────────────────────────┤
│                        ADAPTERS                              │
│    Implements ports, handles I/O, external systems           │
│    ONLY layer that constructs protocol types for wire        │
├─────────────────────────────────────────────────────────────┤
│                       APPLICATION                            │
│         Services, use cases, app-layer DTOs                  │
│         May define use-case-specific port traits             │
├─────────────────────────────────────────────────────────────┤
│                          PORTS                               │
│     Infrastructure port traits (repos, external services)    │
├─────────────────────────────────────────────────────────────┤
│                        PROTOCOL                              │
│      Wire-format DTOs, shared Engine↔Player types            │
│      May re-export stable domain types (documented)          │
├─────────────────────────────────────────────────────────────┤
│                         DOMAIN                               │
│       Entities, value objects, domain events                 │
│               Zero external dependencies                     │
└─────────────────────────────────────────────────────────────┘
```

| Task | Status |
|------|--------|
| [ ] Add diagram to CLAUDE.md | Pending |

---

### 8.2 Address TODO Comments

**17 TODO comments** need resolution:

| Priority | File | Line | Comment | Action |
|----------|------|------|---------|--------|
| DELETE | `tool_execution_service.rs` | 6 | Outdated refactor note | Delete |
| HIGH | `challenge_outcome_approval_service.rs` | 555 | Queue item ID mapping | Create issue |
| HIGH | `challenge_outcome_approval_service.rs` | 731 | Store branches in approval | Create issue |
| MEDIUM | `observation.rs` | 180 | World-based game time | Create issue |
| MEDIUM | `scene_builder.rs` | 284 | Get actual quantity from edge | Create issue |
| MEDIUM | `movement.rs` | 601 | Previous staging lookup | Create issue |
| MEDIUM | `trigger_evaluation_service.rs` | 384 | Get inventory from PC | Create issue |
| MEDIUM | `interaction_repository.rs` | 410 | Edge-based targeting | Create issue |
| LOW | `scene_builder.rs` | 275 | Region item system | Create issue |
| LOW | `staging_context_provider.rs` | 146 | Filter by region | Create issue |
| LOW | `staging_context_provider.rs` | 190 | Add timestamp | Create issue |
| LOW | `challenge.rs` | 350 | OutcomeDetail enhancement | Create issue |
| LOW | `queue_routes.rs` | 119 | Per-world breakdown | Create issue |
| LOW | `session_message_handler.rs` | 276 | Story Arc timeline | Create issue |
| LOW | `session_message_handler.rs` | 999 | Step 8 Phase 4 | Create issue |
| LOW | `content.rs` | 435 | View-as-character mode | Create issue |
| LOW | `event_chains.rs` | 97 | Navigate to event details | Create issue |

| Task | Status |
|------|--------|
| [ ] Delete outdated TODO in tool_execution_service.rs | Pending |
| [ ] Create GitHub issues for HIGH priority TODOs | Pending |
| [ ] Create GitHub issues for MEDIUM priority TODOs | Pending |
| [ ] Create GitHub issues for LOW priority TODOs | Pending |

---

## Verification Commands

Run after each phase:

```bash
# Compilation check
cargo check --workspace

# Architecture check
cargo run -p xtask -- arch-check

# Warnings check
cargo check --workspace 2>&1 | grep "^warning:" | wc -l

# Test compilation (Phase 7)
cargo test --workspace --no-run

# Full test run (after Phase 7)
cargo test --workspace
```

---

## Success Criteria

| Metric | Before | Target |
|--------|--------|--------|
| Critical issues | 2 | 0 |
| Compiler warnings | 51 | 0 |
| Swallowed errors | 43 | 0 (logged) |
| God traits (35+ methods) | 3 (107 methods total) | 0 |
| I/O in application layer | 7 | 0 |
| Protocol imports in services | 14 | 0 |
| Unused structs | 4 | 0 |
| Unused fields | 11 | 0 |
| Shadow variable bugs | 1 | 0 |
| Protocol forward compatibility | None | All enums have #[serde(other)] |
| Test compilation | FAIL (36 errors) | PASS |
| Protocol test coverage | 0% | 100% |
| arch-check | PASS | PASS |

---

## Appendix A: Files Modified by Phase

### Phase 1
- `player-ui/src/presentation/components/creator/motivations_tab.rs`
- `engine-adapters/src/infrastructure/config.rs`

### Phase 2
- `engine-app/src/application/services/llm_queue_service.rs`
- `engine-app/src/application/services/asset_generation_queue_service.rs`
- `engine-app/src/application/services/generation_service.rs`
- `player-adapters/src/infrastructure/http_client.rs`
- `engine-adapters/src/infrastructure/ollama.rs`

### Phase 3
- `player-app/src/application/dto/requests.rs`
- `player-app/src/application/services/challenge_service.rs`
- `player-app/src/application/services/suggestion_service.rs`
- `engine-app/src/application/use_cases/*.rs` (4 files)
- `engine-ports/src/inbound/request_handler.rs`
- `player-ports/src/outbound/game_connection_port.rs`
- `engine-ports/src/outbound/repository_port.rs` (split)

### Phase 4
- `domain/entities/challenge.rs`
- `engine-app/dto/narrative_event.rs`
- Multiple service files (unused fields)
- Multiple UI files (unused variables)

### Phase 5
- `domain/src/entities/*.rs` (9 files)
- `domain/src/ids.rs`
- `domain/src/value_objects/settings.rs`

### Phase 6
- `protocol/src/dto.rs`
- `protocol/src/responses.rs`
- `protocol/src/messages.rs`
- `protocol/src/requests.rs`

### Phase 7
- Multiple test files
- `protocol/src/lib.rs`
- `engine-ports/src/outbound/*.rs`
- `player-ports/src/outbound/*.rs`
- `domain/src/testing/mod.rs` (new)

### Phase 8
- `CLAUDE.md`
- Various files with TODOs

---

## Appendix B: Commit Strategy

Recommended commit sequence:

1. `fix: resolve critical panic risks in production code`
2. `fix: replace hardcoded IPs with localhost defaults`
3. `fix: add error logging to queue workers`
4. `feat: add HTTP request timeouts`
5. `refactor: complete challenge DTO migration`
6. `refactor: consolidate duplicate SuggestionContext`
7. `docs: document port placement architectural decisions`
8. `refactor: remove unused code and fix warnings`
9. `feat: add serde derives to domain entities`
10. `docs: add protocol versioning documentation`
11. `test: fix test compilation`
12. `test: add protocol serialization tests`
13. `docs: update CLAUDE.md with architecture diagram`

---

## Appendix C: Risk Assessment

| Phase | Risk | Mitigation |
|-------|------|------------|
| Phase 1 | Low - Simple fixes | Test manually |
| Phase 2 | Low - Additive changes | Verify logging works |
| Phase 3 | Medium - API changes | Run full test suite |
| Phase 4 | Low - Deletions | Verify no references |
| Phase 5 | Medium - Serialization changes | Test JSON roundtrips |
| Phase 6 | Low - Documentation | Review for accuracy |
| Phase 7 | High - Test infrastructure | Incremental approach |
| Phase 8 | Low - Documentation | Review for accuracy |

---

## Appendix D: Dependencies Between Phases

```
Phase 1 (Critical) ──┬── Phase 2 (Error Handling)
                     │
                     ├── Phase 3 (Architecture) ──── Phase 3.5 (God Traits)*
                     │                                      │
                     │                                      ▼
                     ├── Phase 4 (Dead Code)          Phase 7 (Tests)**
                     │                                      │
                     ├── Phase 5 (Domain) ───────────────────┘
                     │         │
                     │         └── Phase 6 (Protocol)
                     │
                     └── Phase 8 (Docs)

* Phase 3.5 (God Traits) is large (~107 methods) and should be done as separate PR
** Phase 3.5 will BREAK test compilation until Phase 7 updates mock implementations
```

**Recommended execution order**:
1. Phase 1 (Critical) - Do first
2. Phases 2, 4, 5 - Can be done in parallel
3. Phase 3.1-3.4 - Architecture documentation
4. Phase 6 - Protocol polish
5. Phase 3.5 + Phase 7 - God traits + test fixes (do together or sequentially)
6. Phase 8 - Documentation (last)

**Alternative**: Skip Phase 3.5 initially, complete everything else, then do Phase 3.5 + Phase 7 as a dedicated "Interface Segregation" PR.
