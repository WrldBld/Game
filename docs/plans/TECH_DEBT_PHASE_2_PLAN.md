# Tech Debt Remediation Plan - Phase 2

## Overview

This plan addresses remaining technical debt identified after completing the initial refactoring (god trait removal, Neo4j helpers, ID parser macros). The issues are categorized by priority and complexity.

## Current State

- **Clippy Warnings**: ~50 total
  - 41 "too many arguments" warnings
  - 4 "large size difference between variants" warnings
  - 3 "very complex type" warnings
- **Architecture Violations**: 5 glob re-export violations in `engine-dto`
- **Remaining God Traits**: 12 (without ISP replacements)

---

## Phase 1: Fix Glob Re-export Violations (Low Effort, High Value)

**Priority**: High  
**Effort**: Small  
**Files**: `crates/engine-dto/src/lib.rs`

### Problem

The architecture checker flags 5 glob re-exports that violate the "explicit exports" rule:

```rust
pub use llm::*;
pub use persistence::*;
pub use queue::*;
pub use request_context::*;
pub use staging::*;
```

### Solution

Replace each glob re-export with explicit type exports:

```rust
pub use llm::{LlmRequest, LlmResponse, /* ... */};
pub use persistence::{StoredType1, StoredType2, /* ... */};
// etc.
```

### Verification

```bash
cargo xtask arch-check  # Should show 0 glob violations
```

---

## Phase 2: Address Large Variant Size Warnings (Medium Effort)

**Priority**: Medium  
**Effort**: Medium  
**Files**: 
- `crates/player-ports/src/inbound/player_events.rs:327`
- `crates/engine-ports/src/outbound/use_case_types.rs:36`
- `crates/engine-ports/src/outbound/use_case_types.rs:855`
- `crates/player-app/src/application/services/session_service.rs:41`

### Problem

Enums have variants with significantly different sizes, causing memory inefficiency.

### Solution

Box the large variants to reduce enum size:

```rust
// Before
enum MyEnum {
    Small(u32),
    Large(LargeStruct),  // 500+ bytes
}

// After
enum MyEnum {
    Small(u32),
    Large(Box<LargeStruct>),  // Now just pointer-sized
}
```

### Verification

```bash
cargo clippy --workspace 2>&1 | grep "large size difference"  # Should be empty
```

---

## Phase 3: Address "Too Many Arguments" Warnings (High Effort)

**Priority**: Medium  
**Effort**: Large  
**Count**: 41 warnings

### Problem

Functions with more than 7 parameters are flagged by clippy. These are typically:
- Service constructors
- Factory functions
- Complex domain operations

### Solution Options

1. **Builder Pattern**: For constructors with many optional parameters
2. **Parameter Structs**: Group related parameters into a struct
3. **Selective `#[allow]`**: For cases where refactoring isn't practical

### Approach

1. Audit each warning location
2. Categorize: Can be refactored vs. Allow is appropriate
3. Apply builder pattern or parameter structs where beneficial
4. Add targeted `#[allow(clippy::too_many_arguments)]` for others

### Example Refactoring

```rust
// Before: 10 parameters
pub fn new(
    repo1: Arc<dyn Repo1>,
    repo2: Arc<dyn Repo2>,
    repo3: Arc<dyn Repo3>,
    // ... 7 more
) -> Self

// After: Parameter struct
pub struct ServiceDependencies {
    pub repo1: Arc<dyn Repo1>,
    pub repo2: Arc<dyn Repo2>,
    pub repo3: Arc<dyn Repo3>,
    // ...
}

pub fn new(deps: ServiceDependencies) -> Self
```

### Verification

```bash
cargo clippy --workspace 2>&1 | grep "too many arguments" | wc -l  # Target: 0
```

---

## Phase 4: Address Complex Type Warnings (Low Effort)

**Priority**: Low  
**Effort**: Small  
**Count**: 3 warnings

### Problem

Complex nested generic types that are hard to read.

### Solution

Create type aliases:

```rust
// Before
fn foo() -> Arc<RwLock<HashMap<String, Vec<Box<dyn Trait>>>>>

// After
type TraitRegistry = Arc<RwLock<HashMap<String, Vec<Box<dyn Trait>>>>>;
fn foo() -> TraitRegistry
```

### Verification

```bash
cargo clippy --workspace 2>&1 | grep "very complex type"  # Should be empty
```

---

## Phase 5: Migrate Remaining God Traits (High Effort, Optional)

**Priority**: Low (no immediate consumers requesting ISP)  
**Effort**: Large  

### Remaining God Traits (12)

| Trait | Est. Methods | Complexity |
|-------|-------------|------------|
| WorldRepositoryPort | ~10 | Medium |
| RelationshipRepositoryPort | ~8 | Medium |
| AssetRepositoryPort | ~8 | Medium |
| ItemRepositoryPort | ~6 | Low |
| SkillRepositoryPort | ~5 | Low |
| InteractionRepositoryPort | ~5 | Low |
| WorkflowRepositoryPort | ~5 | Low |
| ObservationRepositoryPort | ~5 | Low |
| GoalRepositoryPort | ~4 | Low |
| WantRepositoryPort | ~4 | Low |
| SheetTemplateRepositoryPort | ~4 | Low |
| FlagRepositoryPort | ~3 | Low |

### Approach (per trait)

1. Analyze trait methods and group by responsibility
2. Create ISP traits in `engine-ports/src/outbound/<name>_repository/`
3. Update adapter implementations
4. Migrate consumers to use ISP traits
5. Remove god trait from `repository_port.rs`

### Decision

Defer to future work unless a specific need arises. The high-priority god traits (PlayerCharacter, Scene, EventChain, Location, Region) have already been migrated.

---

## Execution Order

| Phase | Description | Effort | Priority | Depends On |
|-------|-------------|--------|----------|------------|
| 1 | Glob re-export fixes | Small | High | - |
| 2 | Large variant boxing | Medium | Medium | - |
| 3 | Too many arguments | Large | Medium | - |
| 4 | Complex type aliases | Small | Low | - |
| 5 | Remaining god traits | Large | Low | - |

**Recommended order**: Phase 1 → Phase 4 → Phase 2 → Phase 3 → Phase 5 (optional)

---

## Success Criteria

- [ ] `cargo xtask arch-check` passes with 0 violations
- [ ] `cargo clippy --workspace` has significantly fewer warnings
- [ ] All tests pass
- [ ] No new tech debt introduced

---

## Notes

- Phase 3 (too many arguments) may require architectural discussion before proceeding
- Phase 5 should only be done if there's a concrete need for ISP traits
- Some `#[allow]` annotations are acceptable for cases where refactoring adds complexity without benefit
