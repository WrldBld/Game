# ADR-009: Repository Layer Elimination

**Status:** Accepted  
**Date:** 2026-01-17  
**Deciders:** Otto  
**Context:** Architecture review identified over-abstraction in repository layer

## Context

The engine crate has a `repositories/` layer containing 25 wrapper structs that sit between port traits and use cases:

```
Port Traits (ports.rs) → Repository Wrappers → Use Cases → API Handlers
```

A code review revealed that **~60% of repository code is pure 1:1 delegation** adding no value:

```rust
// Example: CharacterRepository has 30+ methods like this
pub async fn get(&self, id: CharacterId) -> Result<Option<Character>, RepoError> {
    self.repo.get(id).await  // Just calls the port trait
}
```

Additionally, **~30% contain misplaced business logic** that should be in use cases (scene resolution, inventory operations, exit calculation).

## Decision

**Eliminate the repository wrapper layer.** Use cases will inject port traits directly.

### What Changes

| Before | After |
|--------|-------|
| Use cases inject `Arc<CharacterRepository>` | Use cases inject `Arc<dyn CharacterRepo>` |
| `App` creates repository wrappers | `App` passes port traits directly |
| 25 repository files | 0 repository wrappers (4 stores remain in `stores/`) |

### What Gets Deleted (~2,300 lines)

**Pure Delegation Repositories:**
- `character.rs` (313 lines)
- `player_character.rs` (95 lines)
- `act.rs` (34 lines)
- `content.rs` (40 lines)
- `interaction.rs` (40 lines)
- `goal.rs` (44 lines)
- `challenge.rs` (69 lines)
- `staging.rs` (219 lines)
- `region_state.rs` (68 lines)
- `location_state.rs` (71 lines)
- `lore.rs` (126 lines) - after extracting any logic
- `narrative.rs` (291 lines) - after extracting any logic

**Service Wrappers:**
- `clock.rs` (20 lines)
- `random.rs` (24 lines)
- `llm.rs` (28 lines)
- `queue.rs` (125 lines)

### What Gets Refactored (Logic → Use Cases)

| Current Location | Logic | Target Use Case |
|------------------|-------|-----------------|
| `scene.rs` | `resolve_scene()`, `evaluate_conditions()` | `use_cases/scene/resolve_scene.rs` |
| `location.rs` | `get_exits()`, `can_move_to()` | `use_cases/movement/get_region_exits.rs` |
| `inventory.rs` | `equip_item()`, `drop_item()`, `pickup_item()`, `give_item_to_pc()` | `use_cases/inventory/*.rs` |
| `observation.rs` | `record_visit()` | `use_cases/observation/record_visit.rs` |
| `world.rs` | `advance_time()` | Already in `use_cases/time/` (remove duplicate) |
| `flag.rs` | `get_all_flags_for_pc()` | `use_cases/flag/get_flags.rs` or inline |

### What Remains

**Renamed to `stores/` (in-memory state):**
- `session.rs` → `stores/session.rs` (wraps `ConnectionManager`)
- `pending_staging.rs` → `stores/pending_staging.rs`
- `directorial.rs` → `stores/directorial.rs`
- `time_suggestion.rs` → `stores/time_suggestion.rs`

**No repository wrappers remain.** All were either:
- Pure delegation (deleted)
- Business logic that moved to use cases
- In-memory state that moved to `stores/`

## Consequences

### Positive

1. **~2,300 fewer lines of code** - Less to maintain, review, test
2. **Clearer architecture** - Port traits ARE the data access abstraction
3. **No confusion about where logic belongs** - If it's not CRUD, it's a use case
4. **Faster navigation** - Use cases directly show their dependencies
5. **Consistent with AGENTS.md** - "~10 port traits, everything else concrete"

### Negative

1. **Use cases now depend on trait objects** - `Arc<dyn CharacterRepo>` instead of concrete type
2. **Larger refactoring effort** - ~50 use case files need import changes
3. **Tests need updating** - Mock the port traits directly

### Neutral

1. **Same testability** - Port traits are already mockable
2. **Same runtime behavior** - Just removing a layer of indirection

## Implementation Plan

### Phase 1: Extract Business Logic (Do First)

Move misplaced logic to proper use cases before deleting repositories:

1. `SceneRepository.resolve_scene()` → `use_cases/scene/resolve_scene.rs`
2. `Location.get_exits()` → `use_cases/movement/get_region_exits.rs`
3. `InventoryRepository.*` → `use_cases/inventory/*.rs`
4. `ObservationRepository.record_visit()` → `use_cases/observation/record_visit.rs`
5. `WorldRepository.advance_time()` → Remove (already in `use_cases/time/`)
6. `FlagRepository.get_all_flags_for_pc()` → Inline or new use case

### Phase 2: Rename In-Memory Stores

```
repositories/session.rs        → stores/session.rs
repositories/pending_staging.rs → stores/pending_staging.rs
repositories/directorial.rs    → stores/directorial.rs
repositories/time_suggestion.rs → stores/time_suggestion.rs
```

### Phase 3: Update Use Cases to Inject Ports Directly

For each use case file:

```rust
// Before
use crate::repositories::CharacterRepository;

pub struct MyUseCase {
    character: Arc<CharacterRepository>,
}

// After
use crate::infrastructure::ports::CharacterRepo;

pub struct MyUseCase {
    character: Arc<dyn CharacterRepo>,
}
```

### Phase 4: Update App Composition

```rust
// Before (app.rs)
let character = Arc::new(repositories::CharacterRepository::new(repos.character.clone()));
// ... pass to use cases

// After (app.rs)
let character: Arc<dyn CharacterRepo> = repos.character.clone();
// ... pass to use cases directly
```

### Phase 5: Delete Repository Files

Remove all pure-delegation repository files.

### Phase 6: Update AGENTS.md

Document the new architecture.

## Alternatives Considered

### Keep Repositories, Move Logic Only

Move business logic to use cases but keep the wrapper layer.

**Rejected because:** This preserves ~1,100 lines of pure boilerplate that adds no value. If we're going to refactor, we should eliminate the unnecessary abstraction entirely.

### Introduce a Macro for Repository Delegation

Use a proc-macro to auto-generate delegation methods.

**Rejected because:** This hides complexity rather than removing it. The delegation layer is fundamentally unnecessary.

## Migration Strategy

The change can be done incrementally:

1. **Week 1:** Extract business logic to use cases (Phase 1)
2. **Week 2:** Rename stores, update a few use cases as proof-of-concept (Phases 2-3)
3. **Week 3:** Update remaining use cases and App (Phases 3-4)
4. **Week 4:** Delete repository files, update docs (Phases 5-6)

Each phase can be merged independently, reducing risk.

## Verification

After completion:

```bash
# No repositories directory exists
ls crates/engine/src/repositories/ 2>/dev/null && echo "FAIL: repositories/ still exists" || echo "PASS: repositories/ eliminated"

# No repository imports in use cases
rg "use crate::repositories::" crates/engine/src/use_cases/ --type rust
# Should return nothing

# All tests pass
cargo test --workspace

# No dead code
cargo clippy --workspace -- -D warnings
```
