# Game Systems Architecture

## Overview

WrldBldr supports multiple tabletop roleplaying game (TTRPG) systems through a modular, trait-based architecture. This document describes how different game systems are integrated while maintaining clean separation of concerns.

## System Categories

Game systems are organized by their fundamental mechanics:

### Category 1: D20-Based Systems
- **D&D 5th Edition** - d20 + modifier vs DC
- **Pathfinder 2e** - d20 + modifier with four degrees of success

### Category 2: Percentile Systems
- **Call of Cthulhu 7e** - d100 roll-under with Hard/Extreme success tiers

### Category 3: Narrative Dice Pool Systems
- **Blades in the Dark** - d6 dice pool, take highest
- **FATE Core** - 4dF + skill on a ladder

### Category 4: Powered by the Apocalypse
- **PbtA Games** - 2d6 + stat with move-based resolution

## Core Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Application Layer                                 │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐                │
│  │ Character UI   │  │  Combat UI     │  │  Content UI    │                │
│  │ (per-system)   │  │  (per-system)  │  │  (universal)   │                │
│  └───────┬────────┘  └───────┬────────┘  └───────┬────────┘                │
└──────────┼───────────────────┼───────────────────┼──────────────────────────┘
           │                   │                   │
┌──────────▼───────────────────▼───────────────────▼──────────────────────────┐
│                            Service Layer                                     │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                      ContentService                                  │   │
│  │  • Spell/Feat/Feature management                                    │   │
│  │  • Import adapters (5etools, etc.)                                  │   │
│  │  • Custom content per world                                         │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    GameSystemRegistry                                │   │
│  │  • System lookup by ID                                              │   │
│  │  • Calculation engine access                                        │   │
│  │  • System metadata                                                  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘
           │
┌──────────▼──────────────────────────────────────────────────────────────────┐
│                            Domain Layer                                      │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     Game System Traits                               │   │
│  │                                                                      │   │
│  │  trait GameSystem {                                                 │   │
│  │      fn system_id() -> &str;                                        │   │
│  │      fn calculation_engine() -> &dyn CalculationEngine;             │   │
│  │      fn dice_system() -> DiceSystem;                                │   │
│  │  }                                                                  │   │
│  │                                                                      │   │
│  │  trait CalculationEngine {                                          │   │
│  │      fn ability_modifier(score: i32) -> i32;                        │   │
│  │      fn roll_interpretation(roll: &RollResult) -> RollOutcome;      │   │
│  │      fn calculate_derived_stat(stat: &str, block: &StatBlock) -> i32;│   │
│  │  }                                                                  │   │
│  │                                                                      │   │
│  │  trait ProgressionSystem {                                          │   │
│  │      fn xp_for_level(level: u32) -> u32;                            │   │
│  │      fn features_at_level(class: &str, level: u32) -> Vec<Feature>;  │   │
│  │  }                                                                  │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐       │
│  │   Dnd5e      │ │   Pf2e       │ │   Coc7e      │ │   FateCore   │       │
│  │   System     │ │   System     │ │   System     │ │   System     │       │
│  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘       │
│  ┌──────────────┐ ┌──────────────┐                                          │
│  │   Blades     │ │   PbtA       │                                          │
│  │   System     │ │   System     │                                          │
│  └──────────────┘ └──────────────┘                                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Data Flow

### Character Creation Flow

```
┌─────────────┐     ┌─────────────┐     ┌─────────────────┐     ┌─────────────┐
│   UI Form   │────▶│  Protocol   │────▶│  GameSystem     │────▶│  Character  │
│  (System-   │     │  Request    │     │  Validation     │     │  Entity     │
│   specific) │     │             │     │  & Defaults     │     │             │
└─────────────┘     └─────────────┘     └─────────────────┘     └─────────────┘
                                               │
                                               ▼
                                        ┌─────────────────┐
                                        │  StatBlock      │
                                        │  (Universal)    │
                                        │  + Modifiers    │
                                        └─────────────────┘
```

### Dice Roll Resolution Flow

```
┌─────────────┐     ┌─────────────────┐     ┌─────────────────┐     ┌─────────────┐
│  Roll       │────▶│  DiceFormula    │────▶│  GameSystem     │────▶│  UI Display │
│  Request    │     │  Execute        │     │  Interpret      │     │  + Effects  │
│             │     │  (d20, d100,    │     │  Result         │     │             │
│             │     │   4dF, d6 pool) │     │                 │     │             │
└─────────────┘     └─────────────────┘     └─────────────────┘     └─────────────┘
                           │                        │
                           ▼                        ▼
                    ┌─────────────┐          ┌─────────────────┐
                    │ Raw Result  │          │  RollOutcome    │
                    │ (numbers)   │          │  (success type, │
                    │             │          │   degree, etc.) │
                    └─────────────┘          └─────────────────┘
```

### Stat Calculation Flow

```
┌─────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  StatBlock  │────▶│  Calculation    │────▶│  Derived Stats  │
│  Base Stats │     │  Engine         │     │  (AC, HP, etc.) │
│             │     │  (System-       │     │                 │
│             │     │   specific)     │     │                 │
└─────────────┘     └─────────────────┘     └─────────────────┘
       │                   │
       ▼                   ▼
┌─────────────┐     ┌─────────────────┐
│  Modifiers  │     │  Stacking Rules │
│  (buffs,    │────▶│  (system-       │
│   items)    │     │   specific)     │
└─────────────┘     └─────────────────┘
```

## Stat Block Flexibility

The `StatBlock` struct is designed to be flexible across all systems:

```rust
pub struct StatBlock {
    // Base values - named arbitrarily per system
    pub stats: HashMap<String, StatValue>,

    // Active modifiers with stacking rules
    pub modifiers: Vec<StatModifier>,

    // System-specific metadata
    pub metadata: HashMap<String, serde_json::Value>,
}
```

### Stat Naming Conventions by System

| System | Primary Stats | Derived Stats |
|--------|--------------|---------------|
| D&D 5e | STR, DEX, CON, INT, WIS, CHA | AC, HP, PROF_BONUS |
| PF2e | STR, DEX, CON, INT, WIS, CHA | AC, HP, PERCEPTION |
| CoC 7e | STR, CON, SIZ, DEX, APP, INT, POW, EDU | HP, SAN, MP, LUCK |
| FATE | APPROACHES or custom | REFRESH, STRESS_PHYS, STRESS_MENT |
| Blades | INSIGHT, PROWESS, RESOLVE | STRESS, TRAUMA |
| PbtA | Varies by game (COOL, HARD, etc.) | HP/HARM varies |

## Modifier Stacking Rules

Different systems have different stacking rules:

```rust
trait StackingRules {
    fn stack_modifiers(&self, modifiers: &[StatModifier]) -> i32;
}

// D&D 5e: Same-named bonuses don't stack (take highest)
// PF2e: Bonuses stack by type (circumstance, item, status)
// CoC 7e: Flat bonuses/penalties stack
// FATE: Aspects provide +2 each, can stack
// Blades: Dice added to pool, not bonuses
```

## UI Components per System

Each system may require specialized UI components:

### D&D 5e / PF2e
- Six-stat display with modifiers
- Spell slot tracker
- Feat/ability list
- Equipment with AC calculation

### Call of Cthulhu
- Eight-characteristic display
- Sanity meter with threshold markers
- Skill percentages with Hard/Extreme values
- Luck track

### FATE Core
- Aspect cards (draggable, invoke/compel buttons)
- Skill pyramid visualization
- Stress boxes with consequence slots
- Fate point tracker

### Blades in the Dark
- Action dot display (12 actions)
- Stress/trauma track
- Load selector
- Clock widgets

### PbtA
- Move cards with triggers
- Stat modifiers (-2 to +3)
- Harm/condition boxes
- Hold counters

## File Structure

```
crates/domain/src/game_systems/
├── mod.rs                 # Module exports, registry
├── traits.rs              # Core traits
├── dnd5e.rs               # D&D 5e implementation
├── pf2e.rs                # Pathfinder 2e implementation
├── coc7e.rs               # Call of Cthulhu 7e implementation
├── fate_core.rs           # FATE Core implementation
├── blades.rs              # Blades in the Dark implementation
├── pbta/
│   ├── mod.rs             # PbtA common code
│   ├── apocalypse_world.rs
│   ├── dungeon_world.rs
│   └── monster_of_week.rs
└── generic.rs             # Fallback generic system

docs/game-systems/
├── ARCHITECTURE.md        # This file
├── dnd5e.md               # D&D 5e rules reference
├── pf2e.md                # Pathfinder 2e rules reference
├── coc7e.md               # Call of Cthulhu 7e rules reference
├── fate_core.md           # FATE Core rules reference
├── blades.md              # Blades in the Dark rules reference
├── pbta.md                # PbtA rules reference
└── UI_MOCKUPS.md          # UI component designs
```

## Extension Points

### Adding a New System

1. Create `new_system.rs` in `game_systems/`
2. Implement `GameSystem` trait
3. Implement `CalculationEngine` trait
4. Add to `GameSystemRegistry::new()`
5. Create documentation in `docs/game-systems/`
6. Create UI components if needed

### System-Specific Content

Systems can define their own content types:

```rust
// In the system module
pub struct Pf2eAncestry { ... }
pub struct BladesPlaybook { ... }
pub struct PbtAMove { ... }
```

These are stored in the `ContentService` with system-specific keys.

## Integration with Existing Code

### StatBlock Usage

The existing `StatBlock` from `character.rs` works with all systems:

```rust
// D&D 5e usage
stats.set_stat("STR", 16);
let modifier = dnd5e.ability_modifier(stats.get_stat("STR").unwrap_or(10));

// CoC 7e usage
stats.set_stat("POW", 65);  // Percentile value
let san = stats.get_stat("POW").unwrap_or(50);  // Starting sanity = POW

// FATE usage
stats.set_stat("FIGHT", 3);  // +3 on the ladder
```

### RuleSystemConfig Integration

The existing `RuleSystemConfig` is extended:

```rust
pub enum RuleSystemVariant {
    Dnd5E,
    Pathfinder2E,
    CallOfCthulhu7E,
    FateCore,
    BladesInTheDark,
    PoweredByApocalypse,
    Custom,
}
```

Each variant maps to its corresponding `GameSystem` implementation.
