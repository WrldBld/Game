# Tier-Level Classification for Domain Value Objects

## Overview

Domain value objects in WrldBldr follow a tiered classification system (Tiers 1-5) based on validation rules, invariants, and complexity. This helps code reviewers and agents make consistent decisions about encapsulation levels.

## Tier Definitions

### Tier 1: Primitive Wrappers (Newtypes)

**Description:** Simple wrappers around primitive types with basic validation.

**Characteristics:**
- Wraps a single primitive type (`String`, `i32`, `f64`, `bool`, etc.)
- Valid by construction (constructor validates, returns `Result<Self, DomainError>`)
- No business logic beyond validation
- Immutable (only `&self` methods)

**Examples:**
- `CharacterName(String)` - Non-empty, max 200 chars, trimmed
- `LocationName(String)` - Non-empty, max 200 chars, trimmed
- `Description(String)` - Max 5000 chars (may be empty)
- `HpValue(i32)` - Non-negative integer

**Encapsulation:** Newtype wrapper with private field.

**Decision Flow:**
```
Is it a wrapper around a single primitive?
├─ Yes → Does it need validation rules?
│  ├─ Yes → Tier 1 (Primitive Wrapper)
│  └─ No → Type alias (e.g., `type UserId = Uuid`)
└─ No → Check Tier 2
```

### Tier 2: Validated Enums

**Description:** Enums with variants representing mutually exclusive states or choices.

**Characteristics:**
- Public variants (no private fields needed)
- May carry associated data
- Used for state machines, outcomes, or mode selection
- No business logic beyond `is_*()` helper methods

**Examples:**
- `CharacterState { Active, Inactive, Dead }`
- `MoodState { Friendly, Neutral, Hostile, Terrified }`
- `DispositionLevel { Low, Medium, High }`

**Encapsulation:** Public variants, optional `impl` with helper methods.

**Decision Flow:**
```
Is it an enum with mutually exclusive variants?
├─ Yes → Does it represent state or outcomes?
│  ├─ Yes → Tier 2 (Validated Enum)
│  └─ No → Consider domain event enum (Tier 4)
└─ No → Check Tier 3
```

### Tier 3: Composite Value Objects

**Description:** Structs combining multiple validated values into a cohesive concept.

**Characteristics:**
- Multiple fields (typically 2-5)
- Each field is itself a validated type (Tier 1 or Tier 2)
- May have cross-field validation rules
- No identity (two instances with same values are equal)
- Immutable (no `&mut self` methods)

**Examples:**
- `StatBlock { strength: StatValue, dexterity: StatValue, ... }`
- `MapBounds { x: f64, y: f64, width: f64, height: f64 }`
- `Quantity { value: i32, unit: QuantityUnit }`
- `ExpressionConfig { id: ExpressionId, sprites: Vec<SpriteId> }`

**Encapsulation:** Public fields for simple data structs; private fields with accessors if cross-field invariants exist.

**Decision Flow:**
```
Is it a struct with multiple fields?
├─ Yes → Does it have cross-field invariants?
│  ├─ Yes → Private fields + accessors + validation methods
│  └─ No → Public fields (simple data struct)
└─ No → Check Tier 4
```

### Tier 4: Domain Events

**Description:** Enums returned from aggregate mutations describing what happened.

**Characteristics:**
- Public variants with associated data
- Used as return types from aggregate mutation methods
- Describes outcomes (not state)
- Pure data (no methods beyond display/debug)

**Examples:**
- `DamageOutcome { AlreadyDead, Wounded { damage_dealt: i32, remaining_hp: i32 }, Killed { damage_dealt: i32 } }`
- `HealOutcome { AlreadyFull, Healed { amount: i32, new_hp: i32 } }`
- `ArchetypeShift { Old: CampbellArchetype, New: CampbellArchetype, Reason: String }`

**Encapsulation:** Public variants, no methods (except Display/Debug derives).

**Decision Flow:**
```
Is it an enum describing what happened?
├─ Yes → Tier 4 (Domain Event)
└─ No → Check Tier 5
```

### Tier 5: Complex Value Objects

**Description:** Value objects with significant business logic or complex validation.

**Characteristics:**
- Encapsulates complex domain concepts
- Has business logic methods (not just validation)
- May compute derived values
- May have internal state representation (e.g., maps, sets)
- Still immutable (only `&self` methods)

**Examples:**
- `Calendar` - Game calendar with date arithmetic
- `ActantialContext` - NPC psychology context with wants/goals
- `StagingContext` - NPC staging logic with region affinities
- `RuleSystem` - Complex game system rules

**Encapsulation:** Private fields + accessors + business logic methods.

**Decision Flow:**
```
Does it have significant business logic beyond validation?
├─ Yes → Tier 5 (Complex Value Object)
└─ No → Re-evaluate Tier 1-4 classification
```

## Quick Reference Table

| Tier | Name | Encapsulation | Has Business Logic? | Example |
|------|------|---------------|-------------------|---------|
| 1 | Primitive Wrapper | Private field + validation | No | `CharacterName(String)` |
| 2 | Validated Enum | Public variants | No (except helpers) | `CharacterState { Active, Dead }` |
| 3 | Composite VO | Public fields (simple) or private (invariants) | Maybe | `StatBlock { strength, dex, ... }` |
| 4 | Domain Event | Public variants | No | `DamageOutcome { Wounded, Killed }` |
| 5 | Complex VO | Private fields + methods | Yes | `Calendar`, `ActantialContext` |

## Decision Tree

```
Start: Does this represent a domain value?
│
├─ Is it a wrapper around a single primitive?
│  └─ Yes → Tier 1 (Primitive Wrapper)
│
├─ Is it an enum?
│  ├─ Mutually exclusive states? → Tier 2 (Validated Enum)
│  └─ Describes what happened? → Tier 4 (Domain Event)
│
├─ Is it a struct?
│  ├─ Simple data grouping (no invariants)? → Tier 3a (Public fields)
│  ├─ Cross-field invariants? → Tier 3b (Private fields + accessors)
│  └─ Significant business logic? → Tier 5 (Complex VO)
│
└─ None of the above → Reconsider design (may be aggregate or DTO)
```

## Encapsulation Guidelines by Tier

### Tier 1: Primitive Wrappers
```rust
pub struct CharacterName(String);  // Private field

impl CharacterName {
    pub fn new(s: impl Into<String>) -> Result<Self, DomainError> { ... }
    pub fn as_str(&self) -> &str { &self.0 }
}
```

### Tier 2: Validated Enums
```rust
pub enum CharacterState {
    Active,
    Inactive,
    Dead,
}

impl CharacterState {
    pub fn is_alive(&self) -> bool {
        matches!(self, Self::Active)
    }
}
```

### Tier 3a: Composite VOs (Simple Data)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
```

### Tier 3b: Composite VOs (With Invariants)
```rust
pub struct StatBlock {
    base: HashMap<StatName, StatValue>,
    modifiers: Vec<StatModifier>,  // Private for validation
}

impl StatBlock {
    pub fn get(&self, stat: StatName) -> StatValue { ... }
    pub fn add_modifier(&mut self, modifier: StatModifier) -> Result<(), DomainError> { ... }
}
```

### Tier 4: Domain Events
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DamageOutcome {
    AlreadyDead,
    Wounded { damage_dealt: i32, remaining_hp: i32 },
    Killed { damage_dealt: i32 },
}
```

### Tier 5: Complex VOs
```rust
pub struct Calendar {
    current: GameTime,
    events: BTreeMap<GameTime, Vec<NarrativeEvent>>,  // Private
}

impl Calendar {
    pub fn advance(&self, amount: Duration) -> Calendar { ... }
    pub fn events_between(&self, start: GameTime, end: GameTime) -> Vec<NarrativeEvent> { ... }
}
```

## When to Create a New Tier

Before creating a new tier, ask:
1. Does this pattern appear frequently in the codebase?
2. Does it require different encapsulation rules?
3. Would it help reviewers make consistent decisions?

If yes to all three, propose adding it to this document.

## References

- [ADR-008: Tiered Encapsulation](ADR-008-tiered-encapsulation.md) - Overall encapsulation strategy
- [AGENTS.md](../../AGENTS.md) - Agent guidelines including Tiered Encapsulation section
- [Domain Crate Structure](../../AGENTS.md#domain-crate-structure) - Domain crate organization
