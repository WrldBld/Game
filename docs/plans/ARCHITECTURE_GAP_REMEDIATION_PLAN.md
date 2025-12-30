# Architecture Gap Remediation Plan

**Created**: December 30, 2024  
**Status**: PLANNED  
**Estimated Total Effort**: 25-30 hours  
**Current Architecture Score**: 92/100  
**Target Architecture Score**: 98/100

---

## Executive Summary

This plan addresses all identified gaps from the comprehensive architecture review, organized into 6 phases. Each phase is designed to be independently executable with clear deliverables.

---

## Phase 1: Quick Wins (2-3 hours)

**Priority**: HIGH  
**Risk**: LOW  
**Dependencies**: None

### 1.1 Fix Clippy Auto-Fixable Warnings

**Effort**: 30 minutes  
**Command**:
```bash
cargo clippy --fix --workspace --allow-dirty
```

**Expected Result**: Eliminates ~344 warnings (81% of total 424)

**Verification**:
```bash
cargo clippy --workspace 2>&1 | grep "warning:" | wc -l
# Should drop from 424 to ~80
```

---

### 1.2 Replace `anyhow` with `thiserror` in Domain

**Effort**: 1 hour  
**Files to Modify**: 3

#### File 1: `crates/domain/src/value_objects/region.rs`

**Lines 29, 36** - `RegionShift::FromStr`:
```rust
// Before
impl std::str::FromStr for RegionShift {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // ...
        _ => Err(anyhow::anyhow!("Invalid region shift: {}", s)),
    }
}

// After
impl std::str::FromStr for RegionShift {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // ...
        _ => Err(DomainError::parse(format!("Invalid region shift: {}", s))),
    }
}
```

**Lines 61, 68** - `RegionFrequency::FromStr`:
```rust
// Same pattern as above
```

#### File 2: `crates/domain/src/entities/observation.rs`

**Lines 51, 58** - `ObservationType::FromStr`:
```rust
// Same pattern as above
```

#### File 3: `crates/domain/Cargo.toml`

**Line 16** - Remove:
```toml
anyhow = { workspace = true }
```

**Verification**:
```bash
cargo check -p wrldbldr-domain
grep -r "anyhow" crates/domain/src/  # Should return nothing
```

---

### 1.3 Fix `derivable_impls` Warnings

**Effort**: 30 minutes  
**Files to Modify**: 3 files in `crates/domain-types/src/`

#### `monomyth.rs` (line ~38):
```rust
// Before
impl Default for MonomythStage {
    fn default() -> Self {
        MonomythStage::OrdinaryWorld
    }
}

// After
#[derive(Default)]  // Add to existing derives
pub enum MonomythStage {
    #[default]
    OrdinaryWorld,
    // ... other variants
}
```

#### `archetype.rs` (line ~30):
```rust
// Same pattern - add #[derive(Default)] and #[default] on first variant
```

#### `rule_system.rs` (lines ~21, ~47):
```rust
// Same pattern for RuleSystemType and RuleSystemVariant
```

**Verification**:
```bash
cargo clippy -p wrldbldr-domain-types 2>&1 | grep "derivable_impls"
# Should return nothing
```

---

### 1.4 Add Missing Crates to Arch-Check

**Effort**: 15 minutes  
**File**: `crates/xtask/src/main.rs`

Add to the `check_dirs` array in `check_no_glob_reexports()` (around line 798):
```rust
let check_dirs = [
    // ... existing entries
    workspace_root.join("crates/engine-dto/src"),
    workspace_root.join("crates/domain-types/src"),
    workspace_root.join("crates/engine-composition/src"),
];
```

**Verification**:
```bash
cargo xtask arch-check
```

---

## Phase 2: DTO Consolidation (3-4 hours)

**Priority**: HIGH  
**Risk**: MEDIUM  
**Dependencies**: Phase 1.2 (domain changes)

### 2.1 Remove `DmApprovalDecision` Duplication

**Effort**: 1.5 hours  
**Current State**: 3 identical definitions

| Location | Status |
|----------|--------|
| `domain/src/value_objects/queue_data.rs:101` | KEEP (canonical) |
| `engine-dto/src/queue.rs:377-406` | REMOVE |
| `engine-ports/src/outbound/dm_approval_queue_service_port.rs:118` | REMOVE |

#### Step 1: Update `engine-dto/src/queue.rs`

Remove the `DmApprovalDecision` enum definition (lines ~377-406).

Add import:
```rust
use wrldbldr_domain::value_objects::DmApprovalDecision;
```

Update any `From` implementations that convert between the types (they become identity or can be removed).

#### Step 2: Update `engine-ports/src/outbound/dm_approval_queue_service_port.rs`

Remove the `DmApprovalDecision` enum definition.

Add re-export:
```rust
pub use wrldbldr_domain::value_objects::DmApprovalDecision;
```

#### Step 3: Update `engine-ports/src/outbound/mod.rs`

Update exports to source from domain.

**Verification**:
```bash
cargo check --workspace
grep -rn "enum DmApprovalDecision" crates/  # Should only show domain
```

---

### 2.2 Remove Player-App `SuggestionContext` Duplication

**Effort**: 30 minutes  
**File**: `crates/player-app/src/application/dto/requests.rs`

Remove local `SuggestionContext` definition (around line 110).

Replace with:
```rust
pub use wrldbldr_protocol::SuggestionContextData as SuggestionContext;
```

Update any code that constructs this type to use the protocol version's field names.

**Verification**:
```bash
cargo check -p wrldbldr-player-app
```

---

### 2.3 Document PromptMappingDto/InputDefaultDto Separation

**Effort**: 30 minutes  
**Files**: 2

#### `crates/protocol/src/dto.rs` (near line 166):
```rust
/// Workflow prompt mapping for wire format.
/// 
/// NOTE: This type uses camelCase serialization for client communication.
/// A separate version exists in `engine-dto::persistence` with snake_case
/// for Neo4j persistence. Do not consolidate - they serve different purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMappingDto { ... }
```

#### `crates/engine-dto/src/persistence.rs` (near line 817):
```rust
/// Workflow prompt mapping for Neo4j persistence.
/// 
/// NOTE: This type uses default (snake_case) serialization for database storage.
/// A separate version exists in `protocol::dto` with camelCase for wire format.
/// Do not consolidate - they serve different purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMappingDto { ... }
```

---

## Phase 3: God Trait Splitting (8-10 hours)

**Priority**: MEDIUM  
**Risk**: MEDIUM  
**Dependencies**: None (can run in parallel with Phase 2)

### 3.1 Split `WorldConnectionManagerPort` (20 methods → 4 traits)

**Effort**: 2 hours  
**File**: `crates/engine-ports/src/outbound/world_connection_manager_port.rs`

#### New File Structure:
```
crates/engine-ports/src/outbound/
├── world_connection_manager/
│   ├── mod.rs
│   ├── query_port.rs         # ConnectionQueryPort (8 methods)
│   ├── context_port.rs       # ConnectionContextPort (6 methods)
│   ├── broadcast_port.rs     # ConnectionBroadcastPort (4 methods)
│   └── lifecycle_port.rs     # ConnectionLifecyclePort (1 method)
└── world_connection_manager_port.rs  # Deprecated, re-exports for compatibility
```

#### Trait Definitions:

```rust
// query_port.rs
#[async_trait]
pub trait ConnectionQueryPort: Send + Sync {
    async fn has_dm(&self, world_id: WorldId) -> bool;
    async fn get_dm_info(&self, world_id: WorldId) -> Option<DmInfo>;
    async fn get_connected_users(&self, world_id: WorldId) -> Vec<ConnectedUserInfo>;
    async fn get_user_role(&self, world_id: &WorldId, user_id: &str) -> Option<WorldRole>;
    async fn find_player_for_pc(&self, world_id: WorldId, pc_id: PlayerCharacterId) -> Option<String>;
    async fn get_world_pcs(&self, world_id: WorldId) -> Vec<PlayerCharacterId>;
    async fn get_all_world_ids(&self) -> Vec<Uuid>;
    async fn stats(&self) -> ConnectionStats;
}

// context_port.rs
#[async_trait]
pub trait ConnectionContextPort: Send + Sync {
    async fn get_user_id_by_client_id(&self, client_id: &str) -> Option<String>;
    async fn is_dm_by_client_id(&self, client_id: &str) -> bool;
    async fn get_world_id_by_client_id(&self, client_id: &str) -> Option<Uuid>;
    async fn get_connection_context(&self, client_id: &str) -> Option<ConnectionContext>;
    async fn get_connection_by_client_id(&self, client_id: &str) -> Option<ConnectionInfo>;
    async fn is_spectator_by_client_id(&self, client_id: &str) -> bool;
    async fn get_pc_id_by_client_id(&self, client_id: &str) -> Option<PlayerCharacterId>;
}

// broadcast_port.rs
#[async_trait]
pub trait ConnectionBroadcastPort: Send + Sync {
    async fn broadcast_to_world(&self, world_id: &Uuid, message: ServerMessage) -> Result<(), BroadcastError>;
    async fn broadcast_to_dms(&self, world_id: &Uuid, message: ServerMessage) -> Result<(), BroadcastError>;
    async fn broadcast_to_players(&self, world_id: &Uuid, message: ServerMessage) -> Result<(), BroadcastError>;
    async fn broadcast_to_all_worlds(&self, message: ServerMessage) -> Result<(), BroadcastError>;
}

// lifecycle_port.rs
#[async_trait]
pub trait ConnectionLifecyclePort: Send + Sync {
    async fn unregister_connection(&self, client_id: &str);
}
```

#### Backward Compatibility:

```rust
// world_connection_manager_port.rs
#[deprecated(note = "Use the split traits: ConnectionQueryPort, ConnectionContextPort, ConnectionBroadcastPort, ConnectionLifecyclePort")]
pub trait WorldConnectionManagerPort: 
    ConnectionQueryPort + ConnectionContextPort + ConnectionBroadcastPort + ConnectionLifecyclePort {}

// Blanket implementation for backward compatibility
impl<T> WorldConnectionManagerPort for T 
where T: ConnectionQueryPort + ConnectionContextPort + ConnectionBroadcastPort + ConnectionLifecyclePort {}
```

---

### 3.2 Split `WorldStatePort` (17 methods → 6 traits)

**Effort**: 2 hours  
**Pattern**: Same as 3.1

| New Trait | Methods |
|-----------|---------|
| `WorldTimeStatePort` | get_game_time, set_game_time, advance_game_time |
| `WorldConversationStatePort` | add_conversation, get_conversation_history, clear_conversation_history |
| `WorldApprovalStatePort` | add_pending_approval, remove_pending_approval, get_pending_approvals |
| `WorldSceneStatePort` | get_current_scene, set_current_scene |
| `WorldDirectorialStatePort` | get_directorial_context, set_directorial_context, clear_directorial_context |
| `WorldLifecyclePort` | initialize_world, cleanup_world, is_world_initialized |

---

### 3.3 Split `PlayerCharacterRepositoryPort` (17 methods → 5 traits)

**Effort**: 2 hours  

| New Trait | Methods |
|-----------|---------|
| `PlayerCharacterCrudPort` | create, get, update, delete |
| `PlayerCharacterQueryPort` | get_by_location, get_by_user_and_world, get_all_by_world, get_unbound_by_user |
| `PlayerCharacterPositionPort` | update_location, update_region, update_position |
| `PlayerCharacterSessionPort` | unbind_from_session |
| `PlayerCharacterInventoryPort` | add_inventory_item, get_inventory, get_inventory_item, update_inventory_item, remove_inventory_item |

---

### 3.4 Split `SceneRepositoryPort` (17 methods → 5 traits)

**Effort**: 1.5 hours  

| New Trait | Methods |
|-----------|---------|
| `SceneCrudPort` | create, get, update, delete, update_directorial_notes |
| `SceneQueryPort` | list_by_act, list_by_location |
| `SceneLocationPort` | set_location, get_location |
| `SceneCharacterPort` | add_featured_character, get_featured_characters, update_featured_character, remove_featured_character, get_scenes_for_character |
| `SceneCompletionPort` | mark_scene_completed, is_scene_completed, get_completed_scenes |

---

### 3.5 Split `EventChainServicePort` and `EventChainRepositoryPort`

**Effort**: 2 hours  

Both follow similar pattern with 4 sub-traits each:
- `*CrudPort` (4 methods)
- `*QueryPort` (4 methods)  
- `*MembershipPort` (3 methods)
- `*StatusPort` (5 methods)

---

### 3.6 Deprecate Legacy Monolithic Traits

**Effort**: 30 minutes  
**File**: `crates/engine-ports/src/outbound/repository_port.rs`

Add deprecation notices:
```rust
#[deprecated(since = "0.2.0", note = "Use split traits in location_repository/ module")]
pub trait LocationRepositoryPort { ... }

#[deprecated(since = "0.2.0", note = "Use split traits in region_repository/ module")]  
pub trait RegionRepositoryPort { ... }
```

---

## Phase 4: app_state.rs Decomposition (4-5 hours)

**Priority**: MEDIUM  
**Risk**: MEDIUM  
**Dependencies**: None

### 4.1 Create New Module Structure

**Effort**: 30 minutes

```
crates/engine-runner/src/composition/
├── mod.rs                  # Module declarations and re-exports
├── app_state.rs            # Main orchestration (~150 lines)
├── infrastructure.rs       # Clock, env, LLM, ComfyUI, settings (~100 lines)
├── repositories.rs         # All 40+ repository ports (~150 lines)
├── core_services.rs        # Core application services (~200 lines)
├── queue_infrastructure.rs # Queue backends, event bus (~150 lines)
├── game_services.rs        # Game-specific services (~200 lines)
├── use_cases.rs            # Use case construction (~200 lines)
└── worker_services.rs      # WorkerServices struct (~80 lines)
```

### 4.2 Define Intermediate Structs

#### `infrastructure.rs`:
```rust
pub struct InfraServices {
    pub clock: Arc<dyn ClockPort>,
    pub environment: Arc<dyn EnvironmentPort>,
    pub llm_client: OllamaClient,
    pub comfyui_client: ComfyUIClient,
    pub settings_service: Arc<SettingsService>,
    pub prompt_template_service: Arc<PromptTemplateService>,
    pub directorial_repo: Arc<dyn DirectorialContextRepositoryPort>,
}

pub async fn create_infrastructure(config: &AppConfig) -> Result<InfraServices> {
    // ~70 lines extracted from app_state.rs lines 242-316
}
```

#### `repositories.rs`:
```rust
pub struct RepositoryPorts {
    pub world: Arc<dyn WorldRepositoryPort>,
    pub character_crud: Arc<dyn CharacterCrudPort>,
    pub character_want: Arc<dyn CharacterWantPort>,
    // ... 40+ more ports
}

pub fn create_repositories(neo4j: &Neo4jRepository) -> RepositoryPorts {
    // ~90 lines extracted from app_state.rs lines 317-401
}
```

#### Similar structs for:
- `CoreServiceBundle` in `core_services.rs`
- `QueueInfrastructure` in `queue_infrastructure.rs`
- `GameServiceBundle` in `game_services.rs`

### 4.3-4.8 Extract Factory Functions

Each factory function:
1. Takes required dependencies as parameters
2. Returns a struct containing created services
3. Is independently testable

### 4.9 Refactor `new_app_state()` to Orchestrator

Final `app_state.rs` (~150 lines):
```rust
pub async fn new_app_state(config: AppConfig) -> Result<(AppState, WorkerServices, ...)> {
    let infra = create_infrastructure(&config).await?;
    let neo4j = create_neo4j_connection(&config).await?;
    let repos = create_repositories(&neo4j);
    let core = create_core_services(&repos, &infra);
    let queues = create_queue_infrastructure(&config, &core).await?;
    let game = create_game_services(&repos, &core, &queues);
    let use_cases = create_use_cases(&repos, &core, &game, &queues);
    
    assemble_app_state(config, infra, repos, core, queues, game, use_cases)
}
```

---

## Phase 5: Manual Clippy Fixes (2-3 hours)

**Priority**: LOW  
**Risk**: LOW  
**Dependencies**: Phase 1.1 (auto-fix first)

### 5.1 Fix `result_large_err` Warnings

**Effort**: 1 hour  
**File**: `crates/engine-adapters/src/infrastructure/websocket/context.rs`  
**Count**: 13 warnings

Box the error type:
```rust
// Before
fn some_function() -> Result<T, LargeError>

// After  
fn some_function() -> Result<T, Box<LargeError>>
```

---

### 5.2 Fix `large_enum_variant` Warnings

**Effort**: 30 minutes  
**Files**: 4 locations

| File | Line | Fix |
|------|------|-----|
| `player-ports/src/inbound/player_events.rs` | 327 | Box large variant |
| `engine-ports/src/outbound/use_case_types.rs` | 36 | Box large variant |
| `engine-ports/src/outbound/use_case_types.rs` | 867 | Box large variant |
| `player-app/src/application/services/session_service.rs` | 41 | Box large variant |

Pattern:
```rust
// Before
enum MyEnum {
    SmallVariant(u32),
    LargeVariant(VeryLargeStruct),
}

// After
enum MyEnum {
    SmallVariant(u32),
    LargeVariant(Box<VeryLargeStruct>),
}
```

---

### 5.3 Address `too_many_arguments` (Priority Functions)

**Effort**: 1-1.5 hours  
**Focus**: Use cases and ports only (highest impact)

| Function | Location | Solution |
|----------|----------|----------|
| `SceneUseCase::new()` | `use_cases/scene.rs:238` | Create `SceneUseCaseDeps` struct |
| `StagingUseCase::new()` | `use_cases/staging.rs:284` | Create `StagingUseCaseDeps` struct |
| `ConnectionUseCase::new()` | `use_cases/connection.rs:260` | Create `ConnectionUseCaseDeps` struct |

Pattern:
```rust
// Before
pub fn new(
    world_service: Arc<dyn WorldService>,
    character_service: Arc<dyn CharacterService>,
    location_service: Arc<dyn LocationService>,
    // ... 10+ more
) -> Self

// After
pub struct SceneUseCaseDeps {
    pub world_service: Arc<dyn WorldService>,
    pub character_service: Arc<dyn CharacterService>,
    pub location_service: Arc<dyn LocationService>,
    // ...
}

pub fn new(deps: SceneUseCaseDeps) -> Self
```

---

## Phase 6: Documentation & Cleanup (1-2 hours)

**Priority**: LOW  
**Risk**: LOW  
**Dependencies**: All other phases

### 6.1 Document `LlmPortDyn` Workaround

**Effort**: 30 minutes  
**File**: `crates/engine-composition/src/app_state.rs`

Create ADR or add detailed comment:
```rust
/// # LlmPortDyn Wrapper Pattern
/// 
/// This trait exists as a workaround for Rust's limitations with async trait objects.
/// 
/// ## Problem
/// `LlmPort` is an async trait with generic error types. Creating `Arc<dyn LlmPort>`
/// requires the error type to be object-safe, which conflicts with associated types.
/// 
/// ## Solution
/// `LlmPortDyn` provides a concrete, boxed-future version of each method that can
/// be made into a trait object. The adapter wraps `OllamaClient` and implements
/// `LlmPortDyn`.
/// 
/// ## Alternative Considered
/// Using `async-trait` with `#[async_trait]` was considered but doesn't solve the
/// associated error type issue for this specific use case.
```

---

### 6.2 Update Architecture Documentation

**Effort**: 30 minutes  
**File**: `docs/architecture/hexagonal-architecture.md`

Update:
- [ ] ISP compliance section with new split traits
- [ ] Crate file counts (now ~450 files)
- [ ] Architecture score (target: 98/100)

---

### 6.3 Archive Completed Plans

**Effort**: 30 minutes  

Add completion markers to:
- `docs/plans/CODE_QUALITY_REMEDIATION_PLAN.md` - Mark COMPLETE
- `docs/plans/HEXAGONAL_CLEANUP_PLAN.md` - Mark ARCHIVED
- `docs/plans/HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md` - Mark COMPLETE
- `docs/plans/HEXAGONAL_GAP_REMEDIATION_PLAN.md` - Mark COMPLETE
- `docs/plans/PROTOCOL_AS_OWNER_REFACTOR_PLAN.md` - Mark COMPLETE

Update `docs/progress/ACTIVE_DEVELOPMENT.md`:
- Mark Phase Q (Code Quality) as COMPLETE
- Reference this plan for remaining work

---

## Execution Summary

| Phase | Effort | Priority | Risk | Dependencies |
|-------|--------|----------|------|--------------|
| 1. Quick Wins | 2-3h | HIGH | LOW | None |
| 2. DTO Consolidation | 3-4h | HIGH | MEDIUM | Phase 1.2 |
| 3. God Trait Splitting | 8-10h | MEDIUM | MEDIUM | None |
| 4. app_state.rs Decomposition | 4-5h | MEDIUM | MEDIUM | None |
| 5. Manual Clippy Fixes | 2-3h | LOW | LOW | Phase 1.1 |
| 6. Documentation | 1-2h | LOW | LOW | All |
| **Total** | **20-27h** | | | |

---

## Verification Checklist

After each phase, run:
```bash
cargo check --workspace
cargo test --workspace
cargo xtask arch-check
cargo clippy --workspace
```

Final verification:
```bash
# All must pass
cargo check --workspace          # Compilation
cargo test --workspace           # Tests
cargo xtask arch-check           # Architecture rules
cargo clippy --workspace         # Lint warnings reduced

# Metrics to track
wc -l crates/engine-runner/src/composition/app_state.rs  # Target: <200 lines
cargo clippy --workspace 2>&1 | grep "warning:" | wc -l   # Target: <50 warnings
```

---

## Notes

- Phases 3 and 4 can run in parallel if multiple developers are available
- Phase 5 can be partially deferred - focus on use cases/ports first
- All trait splits maintain backward compatibility via blanket implementations
- Consider feature flags for deprecated traits if removal timeline is long
