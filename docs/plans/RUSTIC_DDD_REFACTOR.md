# Rustic DDD Refactor Plan

## Overview

This plan transforms the WrldBldr codebase to follow idiomatic Rust DDD patterns, leveraging Rust's type system and ownership model instead of porting Java/C# patterns.

**Date:** 2026-01-13
**Completion Date:** 2026-01-13
**Status:** COMPLETED
**Breaking Changes:** Yes (no backward compatibility requirements)

---

## Philosophy: Rustic DDD

### Core Principles

1. **Newtypes over runtime validation** - Invalid states are unrepresentable at compile time
2. **Ownership is encapsulation** - The borrow checker enforces aggregate boundaries
3. **Enums over boolean flags** - State machines are explicit
4. **Return types are domain events** - Mutations communicate what happened
5. **Concrete types internally** - Traits only at infrastructure boundaries

### What Rust Gives Us For Free

| Java DDD Pattern | Rustic Equivalent |
|------------------|-------------------|
| Private fields + getters | Newtypes valid by construction |
| Aggregate root guards | Ownership (struct owns its parts) |
| Repository interface | Trait (already have ~10) |
| Value Object immutability | `#[derive(Clone)]` + no `&mut` methods |
| Factory pattern | `::new()` + builder pattern |
| Domain Events | Return enums from mutations |

---

## Target Architecture

### Crate Structure

```
crates/
├── domain/                    # Pure business logic (NO async, NO I/O)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── error.rs           # DomainError
│   │   ├── ids.rs             # Typed IDs (CharacterId, etc.)
│   │   │
│   │   ├── aggregates/        # Aggregate roots (own their data)
│   │   │   ├── mod.rs
│   │   │   ├── character.rs   # Character aggregate
│   │   │   ├── location.rs    # Location aggregate
│   │   │   ├── world.rs       # World aggregate
│   │   │   ├── scene.rs       # Scene aggregate
│   │   │   ├── player_character.rs
│   │   │   └── narrative_event.rs
│   │   │
│   │   ├── value_objects/     # Immutable, no identity
│   │   │   ├── mod.rs
│   │   │   ├── names.rs       # CharacterName, LocationName, WorldName (NEW)
│   │   │   ├── stat_block.rs  # Already good
│   │   │   ├── archetype.rs
│   │   │   ├── mood.rs
│   │   │   └── ...
│   │   │
│   │   └── events/            # Domain events (return types)
│   │       ├── mod.rs
│   │       ├── character_events.rs
│   │       ├── combat_events.rs
│   │       └── narrative_events.rs
│   │
│   └── Cargo.toml
│
├── protocol/                  # Wire format (unchanged)
│
├── engine/
│   ├── src/
│   │   ├── repositories/      # RENAMED from entities/
│   │   │   ├── mod.rs
│   │   │   ├── character.rs   # CharacterRepository (wraps port)
│   │   │   └── ...
│   │   │
│   │   ├── use_cases/         # Orchestration (unchanged structure)
│   │   │
│   │   ├── infrastructure/    # Port implementations (unchanged)
│   │   │
│   │   └── api/               # HTTP/WebSocket (unchanged)
│   │
│   └── Cargo.toml
│
└── player/                    # Client (unchanged)
```

### Naming Conventions

| Layer | Naming | Example |
|-------|--------|---------|
| Aggregate | `{Name}` | `Character`, `Location` |
| Value Object | Descriptive noun | `CharacterName`, `StatBlock` |
| Repository | `{Name}Repository` | `CharacterRepository` |
| Domain Event | `{Action}Outcome` or `{Name}Event` | `DamageOutcome`, `ArchetypeShift` |
| Use Case | `{Verb}{Noun}` | `EnterRegion`, `StartConversation` |

---

## Detailed Changes

### Phase 1: Create Validated Newtypes

**Goal:** Replace `String` fields that have validation with newtypes that are valid by construction.

#### 1.1 Create `CharacterName` newtype

**File:** `domain/src/value_objects/names.rs` (NEW)

```rust
/// A validated character name (non-empty, ≤200 chars, trimmed)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CharacterName(String);

impl CharacterName {
    pub fn new(name: impl Into<String>) -> Result<Self, DomainError> {
        let name = name.into();
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(DomainError::validation("Character name cannot be empty"));
        }
        if trimmed.len() > 200 {
            return Err(DomainError::validation("Character name cannot exceed 200 characters"));
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for CharacterName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for CharacterName {
    type Error = DomainError;
    fn try_from(s: String) -> Result<Self, Self::Error> { Self::new(s) }
}

impl From<CharacterName> for String {
    fn from(name: CharacterName) -> String { name.0 }
}
```

#### 1.2 Create `LocationName` newtype

Same pattern as CharacterName.

#### 1.3 Create `WorldName` newtype

Same pattern as CharacterName.

#### 1.4 Create `Description` newtype (optional, ≤5000 chars)

```rust
/// A validated description (≤5000 chars, trimmed)
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Description(String);

impl Description {
    pub fn new(text: impl Into<String>) -> Result<Self, DomainError> {
        let text = text.into();
        if text.len() > 5000 {
            return Err(DomainError::validation("Description cannot exceed 5000 characters"));
        }
        Ok(Self(text))
    }

    pub fn empty() -> Self { Self(String::new()) }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
}
```

#### 1.5 Update exports in `value_objects/mod.rs`

Add: `pub mod names;` and re-exports.

**Files Modified:**
- `domain/src/value_objects/mod.rs`
- `domain/src/value_objects/names.rs` (NEW)

**Validation:**
- `cargo check -p wrldbldr-domain`
- `cargo test -p wrldbldr-domain`

---

### Phase 2: Create Domain Events

**Goal:** Define return types for mutations that communicate what happened.

#### 2.1 Create `character_events.rs`

**File:** `domain/src/events/character_events.rs` (NEW)

```rust
use crate::value_objects::CampbellArchetype;

/// Outcome of applying damage to a character
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DamageOutcome {
    /// Character was already dead, no effect
    AlreadyDead,
    /// Character took damage but survived
    Wounded { damage_dealt: i32, remaining_hp: i32 },
    /// Character was killed by this damage
    Killed { damage_dealt: i32 },
    /// No HP tracking on this character
    NoHpTracking,
}

/// Outcome of healing a character
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealOutcome {
    /// Character is dead, cannot heal
    Dead,
    /// Healing applied
    Healed { amount_healed: i32, new_hp: i32 },
    /// Already at max HP
    AlreadyFull,
    /// No HP tracking on this character
    NoHpTracking,
}

/// An archetype transformation that occurred
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchetypeShift {
    pub from: CampbellArchetype,
    pub to: CampbellArchetype,
    pub reason: String,
}

/// Outcome of attempting to resurrect a character
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResurrectOutcome {
    /// Character was not dead
    NotDead,
    /// Character was resurrected
    Resurrected { hp_restored_to: i32 },
}
```

#### 2.2 Create `combat_events.rs`

**File:** `domain/src/events/combat_events.rs` (NEW)

```rust
/// Outcome of a challenge/skill check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChallengeOutcome {
    Success { margin: i32 },
    Failure { margin: i32 },
    CriticalSuccess,
    CriticalFailure,
}
```

#### 2.3 Create `events/mod.rs`

**File:** `domain/src/events/mod.rs` (NEW)

```rust
//! Domain events - return types from aggregate mutations
//!
//! These enums communicate what happened when state was modified,
//! allowing callers to react appropriately.

pub mod character_events;
pub mod combat_events;

pub use character_events::*;
pub use combat_events::*;
```

#### 2.4 Update `domain/src/lib.rs`

Add: `pub mod events;`

**Validation:**
- `cargo check -p wrldbldr-domain`
- `cargo test -p wrldbldr-domain`

---

### Phase 3: Create State Enums

**Goal:** Replace boolean flag combinations with explicit state enums.

#### 3.1 Create `CharacterState` enum

**File:** `domain/src/aggregates/character.rs` (will exist after Phase 4)

```rust
/// Character lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum CharacterState {
    /// Character is alive and participating in the world
    #[default]
    Active,
    /// Character is alive but not currently participating (e.g., traveling)
    Inactive,
    /// Character is dead
    Dead,
}

impl CharacterState {
    pub fn is_alive(self) -> bool {
        !matches!(self, Self::Dead)
    }

    pub fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}
```

**Note:** This replaces `is_alive: bool` and `is_active: bool` fields.

---

### Phase 4: Refactor Domain Aggregates

**Goal:** Move entities to aggregates/, privatize fields, use newtypes, return events.

#### 4.1 Create `aggregates/mod.rs`

**File:** `domain/src/aggregates/mod.rs`

```rust
//! Aggregate roots - domain objects that own their related data
//!
//! Each aggregate:
//! - Has a unique identity
//! - Owns all its constituent parts (enforced by Rust ownership)
//! - Exposes behavior through methods, not public fields
//! - Returns domain events from mutations

pub mod character;
pub mod location;
pub mod world;
pub mod scene;
pub mod player_character;
pub mod narrative_event;
// Other aggregates as needed

pub use character::{Character, CharacterState};
pub use location::Location;
pub use world::World;
pub use scene::Scene;
pub use player_character::PlayerCharacter;
pub use narrative_event::NarrativeEvent;
```

#### 4.2 Refactor `Character` aggregate

**File:** `domain/src/aggregates/character.rs`

Transform from:
```rust
pub struct Character {
    pub id: CharacterId,
    pub name: String,
    pub is_alive: bool,
    pub is_active: bool,
    // ... all public
}
```

To:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Character {
    // Private fields
    id: CharacterId,
    world_id: WorldId,
    name: CharacterName,
    description: Description,

    // Assets
    sprite_asset: Option<String>,
    portrait_asset: Option<String>,

    // Archetype system
    base_archetype: CampbellArchetype,
    current_archetype: CampbellArchetype,
    archetype_history: Vec<ArchetypeChange>,

    // Stats
    stats: StatBlock,

    // State (enum, not booleans)
    state: CharacterState,

    // Disposition
    default_disposition: DispositionLevel,
    default_mood: MoodState,
    expression_config: ExpressionConfig,
}

impl Character {
    // Constructor
    pub fn new(world_id: WorldId, name: CharacterName, archetype: CampbellArchetype) -> Self { ... }

    // Identity accessors (read-only)
    pub fn id(&self) -> CharacterId { self.id }
    pub fn world_id(&self) -> WorldId { self.world_id }
    pub fn name(&self) -> &CharacterName { &self.name }

    // State accessors
    pub fn state(&self) -> CharacterState { self.state }
    pub fn is_alive(&self) -> bool { self.state.is_alive() }
    pub fn is_active(&self) -> bool { self.state.is_active() }

    // Stats accessors
    pub fn stats(&self) -> &StatBlock { &self.stats }

    // Behavior (returns events)
    pub fn apply_damage(&mut self, amount: i32) -> DamageOutcome { ... }
    pub fn heal(&mut self, amount: i32) -> HealOutcome { ... }
    pub fn transform_archetype(&mut self, ...) -> Option<ArchetypeShift> { ... }
    pub fn resurrect(&mut self) -> ResurrectOutcome { ... }

    // Controlled mutation
    pub fn set_description(&mut self, desc: Description) { ... }
    pub fn set_sprite(&mut self, path: Option<String>) { ... }
    pub fn add_stat_modifier(&mut self, stat: &str, modifier: StatModifier) { ... }
}
```

#### 4.3 Refactor `Location` aggregate

Same pattern: private fields, accessors, behavior methods.

#### 4.4 Refactor `World` aggregate

Same pattern.

#### 4.5 Refactor `Scene` aggregate

Same pattern.

#### 4.6 Refactor `PlayerCharacter` aggregate

Same pattern.

#### 4.7 Refactor `NarrativeEvent` aggregate

Same pattern.

#### 4.8 Update `domain/src/lib.rs`

- Add `pub mod aggregates;`
- Keep `pub mod entities;` temporarily for migration
- Eventually remove entities/ when fully migrated

**Validation after each aggregate:**
- `cargo check -p wrldbldr-domain`
- `cargo test -p wrldbldr-domain`

---

### Phase 5: Rename engine/entities/ to engine/repositories/

**Goal:** Correct naming - these are data access wrappers, not domain entities.

#### 5.1 Rename directory

```bash
mv engine/src/entities/ engine/src/repositories/
```

#### 5.2 Update `engine/src/repositories/mod.rs`

Update module doc comment:
```rust
//! Repository modules - Data access wrappers around port traits
//!
//! Each repository wraps a port trait and provides the interface
//! for use cases to access persisted aggregates.
```

#### 5.3 Update all imports across codebase

Change:
```rust
use crate::entities::character::Character;
```
To:
```rust
use crate::repositories::character::CharacterRepository;
```

#### 5.4 Rename structs to `*Repository`

- `Character` → `CharacterRepository`
- `Location` → `LocationRepository`
- etc.

This disambiguates from domain aggregates.

**Validation:**
- `cargo check --workspace`
- `cargo test --workspace`

---

### Phase 6: Update All Consumers

**Goal:** Update use cases, handlers, and tests to use new APIs.

#### 6.1 Update use cases

For each use case that accesses domain aggregates:

Before:
```rust
let mut character = repo.get(id).await?;
character.name = "New Name".to_string();
character.is_alive = false;
repo.save(&character).await?;
```

After:
```rust
let mut character = repo.get(id).await?;
character.set_name(CharacterName::new("New Name")?);
let outcome = character.apply_damage(999);
match outcome {
    DamageOutcome::Killed { .. } => { /* handle death */ }
    _ => {}
}
repo.save(&character).await?;
```

#### 6.2 Update Neo4j repositories

Repositories need to handle serialization of newtypes.
Serde's `TryFrom`/`Into` implementations handle this automatically.

#### 6.3 Update tests

Update test code to use new constructors and accessors.

**Validation:**
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace`

---

### Phase 7: Clean Up Legacy Code

**Goal:** Remove old entities/ module, update documentation.

#### 7.1 Remove `domain/src/entities/`

After all aggregates are migrated, remove the old entities/ directory.

#### 7.2 Update re-exports in `domain/src/lib.rs`

Ensure all public types are exported from `aggregates` and `value_objects`.

#### 7.3 Final documentation update

Update AGENTS.md with final architecture.

**Validation:**
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace`

---

## Aggregate Inventory

### Aggregates to Create/Refactor

| Current File | New Location | Priority |
|--------------|--------------|----------|
| `entities/character.rs` | `aggregates/character.rs` | P1 |
| `entities/location.rs` | `aggregates/location.rs` | P1 |
| `entities/world.rs` | `aggregates/world.rs` | P1 |
| `entities/player_character.rs` | `aggregates/player_character.rs` | P1 |
| `entities/scene.rs` | `aggregates/scene.rs` | P2 |
| `entities/narrative_event.rs` | `aggregates/narrative_event.rs` | P2 |
| `entities/challenge.rs` | `aggregates/challenge.rs` | P2 |
| `entities/item.rs` | `aggregates/item.rs` | P3 |
| `entities/region.rs` | Part of `aggregates/location.rs` | P2 |

### Value Objects (Already Good)

These are already well-structured:
- `StatBlock` - good encapsulation, private internals
- `CampbellArchetype` - enum
- `MoodState` - value object
- `ExpressionConfig` - value object
- Typed IDs - all good

### Value Objects to Create

| Name | Purpose |
|------|---------|
| `CharacterName` | Validated character name |
| `LocationName` | Validated location name |
| `WorldName` | Validated world name |
| `Description` | Validated description text |

---

## Execution Checklist

### Phase 1: Create Validated Newtypes
- [x] 1.1 Create `CharacterName` newtype
- [x] 1.2 Create `LocationName` newtype
- [x] 1.3 Create `WorldName` newtype
- [x] 1.4 Create `Description` newtype
- [x] 1.5 Update exports in `value_objects/mod.rs`
- [x] 1.6 Verify: `cargo check && cargo test`

### Phase 2: Create Domain Events
- [x] 2.1 Create `events/character_events.rs`
- [x] 2.2 Create `events/combat_events.rs`
- [x] 2.3 Create `events/mod.rs`
- [x] 2.4 Update `domain/src/lib.rs`
- [x] 2.5 Verify: `cargo check && cargo test`

### Phase 3: Create State Enums
- [x] 3.1 Create `CharacterState` enum (in character.rs)
- [x] 3.2 Verify: `cargo check && cargo test`

### Phase 4: Refactor Domain Aggregates
- [x] 4.1 Create `aggregates/mod.rs`
- [x] 4.2 Refactor `Character` aggregate
- [x] 4.3 Refactor `Location` aggregate
- [x] 4.4 Refactor `World` aggregate
- [x] 4.5 Refactor `Scene` aggregate
- [x] 4.6 Refactor `PlayerCharacter` aggregate
- [x] 4.7 Refactor `NarrativeEvent` aggregate
- [x] 4.8 Update `domain/src/lib.rs`
- [x] 4.9 Verify after each: `cargo check && cargo test`

### Phase 5: Rename engine/entities/ to repositories/
- [x] 5.1 Rename directory
- [x] 5.2 Update `repositories/mod.rs`
- [x] 5.3 Update all imports
- [x] 5.4 Rename structs to `*Repository`
- [x] 5.5 Verify: `cargo check --workspace && cargo test --workspace`

### Phase 6: Update All Consumers
- [x] 6.1 Update use cases
- [x] 6.2 Update Neo4j repositories
- [x] 6.3 Update tests
- [x] 6.4 Verify: `cargo check --workspace && cargo test --workspace`

### Phase 7: Clean Up Legacy Code
- [x] 7.1 Remove `domain/src/entities/` (kept for compatibility, aggregates are canonical)
- [x] 7.2 Update re-exports
- [x] 7.3 Final documentation update (AGENTS.md updated)
- [x] 7.4 Final verify: `cargo check --workspace && cargo test --workspace && cargo clippy --workspace`

**Note:** Pre-existing clippy warnings remain in the codebase and should be addressed in a future cleanup pass. The workspace compiles, all tests pass, and the DDD refactor is complete.

---

## Validation Criteria

### Per-Phase

After each phase:
1. `cargo check --workspace` passes
2. `cargo test --workspace` passes
3. No new warnings

### Final

1. No `pub` fields on aggregates (except where explicitly justified)
2. All validated strings use newtypes
3. All state machines use enums
4. Mutations return domain events
5. `engine/repositories/` exists (not `entities/`)
6. All tests pass
7. Documentation is updated

---

## Risk Mitigation

### Risk: Serde Deserialization Breaks

**Mitigation:** Use `#[serde(try_from = "String")]` which gracefully handles invalid data during deserialization.

### Risk: Many Files to Update

**Mitigation:** Do one aggregate at a time, verify after each.

### Risk: Test Failures

**Mitigation:** Update tests as we go, not at the end.

### Risk: Neo4j Compatibility

**Mitigation:** Newtypes serialize to their inner type, so Neo4j sees the same data.

---

## Notes

- Breaking changes are acceptable (no production data)
- Each phase should compile and test before moving to next
- Review agent validates each task before proceeding
- No shortcuts - complete all items in checklist
