# Code Quality Remediation Plan

**Status**: ACTIVE  
**Created**: 2025-12-28  
**Last Updated**: 2025-12-28  
**Goal**: Achieve a clean, production-ready codebase with zero technical debt  
**Estimated Total Effort**: 40-50 hours

---

## Executive Summary

Two comprehensive code reviews identified issues across the WrldBldr codebase. This plan consolidates all findings into a prioritized remediation roadmap organized by severity and effort.

### Issue Summary

| Severity | Count | Categories |
|----------|-------|------------|
| Critical | 2 | Production panic risks |
| High | ~60 | Swallowed errors, god traits, architecture gaps |
| Medium | ~80 | Dead code, missing derives, config issues |
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

## Phase 1: Critical Fixes (1-2 hours)

**Priority**: IMMEDIATE - These can cause production crashes or security issues

### 1.1 Fix Production Panic Risks

**Files**:
- `crates/player-ui/src/presentation/components/creator/motivations_tab.rs`

**Issue**: Lines 498 and 500 use `.unwrap()` on `strip_prefix()` which can panic if the guard condition doesn't match.

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

### 1.3 Fix Production unwrap() in Staging Service

**File**: `crates/engine-app/src/application/services/staging_service.rs:535`

**Issue**: Production code uses `.unwrap()` on JSON parsing.

**Current Code**:
```rust
let json = extract_json_array(response).unwrap();
```

**Fix**:
```rust
let json = extract_json_array(response)
    .ok_or_else(|| anyhow::anyhow!("Failed to parse LLM response as JSON array"))?;
```

| Task | Status |
|------|--------|
| [ ] Fix staging_service.rs:535 unwrap | Pending |

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

## Phase 3: Architecture Completion (6-8 hours)

**Priority**: HIGH - Complete hexagonal architecture gaps

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

**Traits to split**:

#### CharacterRepositoryPort (~35 methods)

**Current**: `engine-ports/src/outbound/repository_port.rs:94-382`

**Split into**:
- `CharacterCrudPort` - Basic CRUD (5 methods)
- `CharacterWantsPort` - Want/motivation operations (6 methods)
- `CharacterActantialPort` - Actantial views (5 methods)
- `CharacterInventoryPort` - Inventory operations (5 methods)
- `CharacterLocationPort` - Location relationships (8 methods)
- `NpcDispositionPort` - NPC disposition (7 methods)

#### StoryEventRepositoryPort (~35 methods)

**Current**: `engine-ports/src/outbound/repository_port.rs:1183-1364`

**Split into**:
- `StoryEventCrudPort` - CRUD and search
- `StoryEventRelationshipPort` - Edge methods
- `DialogueHistoryPort` - Dialogue-specific methods

#### ChallengeRepositoryPort (~25 methods)

**Current**: `engine-ports/src/outbound/repository_port.rs:1007-1176`

**Split into**:
- `ChallengeCrudPort` - Basic CRUD
- `ChallengeSkillPort` - Skill relationships
- `ChallengeAvailabilityPort` - Availability checks

| Task | Status |
|------|--------|
| [ ] Create new trait files in engine-ports/outbound/ | Pending |
| [ ] Split CharacterRepositoryPort | Pending |
| [ ] Split StoryEventRepositoryPort | Pending |
| [ ] Split ChallengeRepositoryPort | Pending |
| [ ] Update all trait implementations in adapters | Pending |
| [ ] Update all trait usages in app layer | Pending |
| [ ] Verify compilation | Pending |

**Note**: This is a significant refactor. Consider doing in a separate PR.

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
| `generation_service.rs:116` | `completed_count` | DELETE |
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
| [ ] Delete completed_count from generation_service.rs | Pending |
| [ ] Delete character_repository from scene_resolution_service.rs | Pending |
| [ ] Delete challenge_repo, character_repo from trigger_evaluation_service.rs | Pending |
| [ ] Decide on broadcast fields (implement or delete) | Pending |
| [ ] Implement or delete broadcast fields based on decision | Pending |

---

### 4.3 Remove Unused Constants and Imports

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

### 4.4 Address #[allow(dead_code)] Suppressions

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

### 4.5 Fix Unused Variables in UI Layer

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

**Issue**: Test suite fails to compile due to trait mismatches.

| Task | Status |
|------|--------|
| [ ] Run `cargo test --workspace` and collect errors | Pending |
| [ ] Fix trait method mismatches | Pending |
| [ ] Fix missing type imports | Pending |
| [ ] Fix stale mock implementations | Pending |
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
| God traits (35+ methods) | 3 | 0 |
| Protocol imports in services | 14 | 0 |
| Unused structs | 4 | 0 |
| Unused fields | 12 | 0 |
| Test compilation | FAIL | PASS |
| Protocol test coverage | 0% | 100% |
| arch-check | PASS | PASS |

---

## Appendix A: Files Modified by Phase

### Phase 1
- `player-ui/src/presentation/components/creator/motivations_tab.rs`
- `engine-adapters/src/infrastructure/config.rs`
- `engine-app/src/application/services/staging_service.rs`

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
                     │
                     ├── Phase 4 (Dead Code)
                     │
                     ├── Phase 5 (Domain) ─────────── Phase 6 (Protocol)
                     │
                     └── Phase 7 (Tests) ──────────── Phase 8 (Docs)

* Phase 3.5 (God Traits) is large and can be done as separate PR
```

Phases 1, 2, 4, 5 can be done in parallel after Phase 1 critical fixes.
Phase 3 should be done before Phase 7 (tests depend on stable interfaces).
Phase 8 should be done last to capture final state.
