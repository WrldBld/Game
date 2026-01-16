# ADR-008: Tiered Encapsulation for Domain Types

## Status

Accepted

## Date

2026-01-16

## Context

During Phase 1 of the Clean Codebase Remediation, we applied Java/C#-style encapsulation uniformly across all domain types: private fields with accessor methods for every struct. This resulted in ~2000 accessor methods and ~700 builder methods, many for simple data structs with no invariants to protect.

This contradicts the "Rustic DDD Philosophy" already documented in AGENTS.md, which states:

> Instead of porting Java/C# DDD patterns, we leverage Rust's strengths:
> | Java DDD Pattern | Rustic Equivalent |
> | Private fields + getters | **Newtypes** valid by construction |

The question arose: when should we use private fields + accessors vs public fields vs newtypes?

## Decision

We adopt **tiered encapsulation** based on whether the type has invariants to protect:

### Tier 1: Aggregates with Invariants

Types where invalid states must be prevented at compile time or construction time.

**Use:** Private fields + accessors + mutation methods returning events

**Examples:**
- `Character` (hp cannot exceed max_hp, name must be validated)
- `Challenge` (difficulty constraints, outcome state machine)
- `StatBlock` (modifier calculations must be consistent)

```rust
pub struct Character {
    id: CharacterId,          // Private
    name: CharacterName,      // Private
    current_hp: i32,          // Private - must be <= max_hp
    max_hp: i32,
}

impl Character {
    pub fn id(&self) -> CharacterId { self.id }
    pub fn apply_damage(&mut self, amount: i32) -> DamageOutcome { ... }
}
```

### Tier 2: Validated Newtypes

Wrapper types that validate their contents on construction.

**Use:** Newtype with `::new()` returning `Result<Self, DomainError>`, `#[serde(try_from)]`

**Examples:**
- `CharacterName` (non-empty, max length)
- `Description` (max length, trimmed)
- `Tag` (non-empty, lowercase)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct CharacterName(String);

impl CharacterName {
    pub fn new(s: impl Into<String>) -> Result<Self, DomainError> {
        let s = s.into().trim().to_string();
        if s.is_empty() { return Err(DomainError::validation("empty")); }
        Ok(Self(s))
    }
    pub fn as_str(&self) -> &str { &self.0 }
}
```

### Tier 3: Typed IDs

Always use newtype wrappers for identifiers - this provides compile-time type safety.

**Use:** Newtype wrapper around `Uuid`

**Examples:**
- `CharacterId`, `LocationId`, `WorldId`, `SceneId`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CharacterId(Uuid);

impl CharacterId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}
```

### Tier 4: Simple Data Structs

Types that just group related data with no invariants.

**Use:** Public fields, derive common traits

**Examples:**
- `MapBounds { x, y, width, height }` - just coordinates
- `TimeAdvanceResult { new_time, events }` - just a result tuple
- `SceneSnapshot { characters, location, time }` - just a data snapshot
- DTOs for wire format

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
```

### Tier 5: Enums

State machines, outcomes, and choices use public variants.

**Use:** Enum with public variants carrying relevant data

**Examples:**
- `DamageOutcome { Wounded, Killed, AlreadyDead }`
- `CharacterState { Active, Inactive, Dead }`
- `ChallengeResult { Success, Failure, Critical }`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DamageOutcome {
    AlreadyDead,
    Wounded { damage_dealt: i32, remaining_hp: i32 },
    Killed { damage_dealt: i32 },
}
```

## Decision Criteria Flowchart

```
Does this type have invariants to protect?
├─ YES → Does it need mutation methods?
│        ├─ YES → Tier 1: Aggregate (private fields + accessors + mutations)
│        └─ NO  → Tier 2: Validated Newtype (constructor validates)
│
└─ NO  → Is it an identifier?
         ├─ YES → Tier 3: Typed ID (always newtype)
         └─ NO  → Is it a state/outcome?
                  ├─ YES → Tier 5: Enum (public variants)
                  └─ NO  → Tier 4: Simple Data Struct (public fields)
```

## Consequences

### Positive

1. **Less boilerplate**: Simple data structs don't need accessor methods
2. **Rust-idiomatic**: Leverages Rust's ownership/borrowing instead of Java patterns
3. **Clear guidance**: Developers know when to encapsulate vs expose
4. **Easier refactoring**: Public fields are easier to extend/modify

### Negative

1. **Judgment required**: Developers must decide which tier applies
2. **Existing code**: Some over-encapsulated code from Phase 1 remains (low priority to fix)
3. **Consistency**: Mix of styles until fully migrated

### Neutral

1. **Documentation**: AGENTS.md updated with tiered encapsulation section
2. **Remediation plan**: Phases 1.4/1.6 updated with nuanced guidance

## Examples of Over-Encapsulation to Avoid

```rust
// BAD: Pointless encapsulation for a coordinate struct
pub struct MapBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl MapBounds {
    pub fn x(&self) -> f64 { self.x }
    pub fn y(&self) -> f64 { self.y }
    pub fn width(&self) -> f64 { self.width }
    pub fn height(&self) -> f64 { self.height }
    pub fn with_x(mut self, x: f64) -> Self { self.x = x; self }
    pub fn with_y(mut self, y: f64) -> Self { self.y = y; self }
    // ... 50 lines of boilerplate for no benefit
}

// GOOD: Just use public fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
```

## References

- AGENTS.md "Rustic DDD Philosophy" section
- Clean Codebase Remediation Plan phases 1.4, 1.6
- [Rust API Guidelines on encapsulation](https://rust-lang.github.io/api-guidelines/)
