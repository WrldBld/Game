# Narrative Resolution System

## Overview

## Canonical vs Implementation

This document is canonical for how the system *should* behave in gameplay.
Implementation notes are included to track current status and may lag behind the spec.

**Legend**
- **Canonical**: Desired gameplay rule or behavior (source of truth)
- **Implemented**: Verified in code and wired end-to-end
- **Planned**: Designed but not fully implemented yet


The Narrative Resolution System provides configurable mechanics for fiction-first tabletop RPG systems. It supports three major resolution styles:

1. **PbtA (Powered by the Apocalypse)** - Fixed thresholds with 2d6+stat
2. **Fate/Ladder** - Descriptor-to-number ladder with configurable Fudge dice
3. **Blades (Forged in the Dark)** - Position/Effect system with d6 pools

Each style can be configured at the rule system level (stored with World in Neo4j) and overridden per-world via settings (stored in SQLite).

## Resolution Styles

### PbtA Style

**Dice**: 2d6 + stat modifier

**Thresholds** (configurable):
| Total | Outcome |
|-------|---------|
| 10+ | Full Success |
| 7-9 | Partial Success (success with cost/complication) |
| 6- | Miss/Failure |

**Descriptor Role**: In pure PbtA, difficulty descriptors affect the *narrative consequences* rather than the roll thresholds. A "Desperate" situation means failure is more severe, not that you need to roll higher.

**Configuration Options**:
- `full_success`: Threshold for full success (default: 10)
- `partial_success`: Threshold for partial success (default: 7)
- `critical_success`: Optional threshold for critical (default: none)
- `critical_failure`: Optional threshold for critical failure (default: none)

### Fate/Ladder Style

**Dice**: Configurable number of Fudge dice (default: 4dF)

Fudge dice have three faces: `+` (+1), `-` (-1), and blank (0). Rolling 4dF produces results from -4 to +4, with 0 being most common.

**Resolution**: Roll NdF + skill/approach, compare to ladder target number.

**Difficulty Ladder** (default Fate Core):
| Descriptor | Value | Display Name |
|------------|-------|--------------|
| Trivial | -2 | Terrible |
| Easy | 0 | Mediocre |
| Routine | +1 | Average |
| Moderate | +2 | Fair |
| Challenging | +3 | Good |
| Hard | +4 | Great |
| VeryHard | +5 | Superb |
| Extreme | +6 | Fantastic |
| Impossible | +8 | Legendary |

**Outcomes** based on shifts (roll result - target):
| Shifts | Outcome |
|--------|---------|
| +3 or more | Succeed with Style (critical) |
| +1 to +2 | Succeed |
| 0 | Tie (partial success) |
| Negative | Fail |

**Configuration Options**:
- `dice_count`: Number of Fudge dice (default: 4)
- `style_threshold`: Shifts needed for succeed with style (default: 3)
- `tie_threshold`: Shift value that counts as a tie (default: 0)
- `ladder`: Custom ladder entries mapping descriptors to values

### Blades/Position-Effect Style

**Dice**: d6 pool (take highest die)

**Core Mechanic**: The GM sets two independent variables before the roll:
- **Position**: How dangerous is this action? (determines consequence severity)
- **Effect**: How much will this accomplish? (determines progress/impact)

#### Position Levels

| Position | Description | Consequence Severity |
|----------|-------------|---------------------|
| Controlled | You act on your terms, exploit dominant advantage | Minor |
| Risky | You go head to head, take a chance | Moderate |
| Desperate | You overreach, in serious trouble | Severe |

#### Effect Levels

| Effect | Description | Clock Ticks |
|--------|-------------|-------------|
| Zero | No effect possible | 0 |
| Limited | Partial or weak effect | 1 |
| Standard | Normal expected effect | 2 |
| Great | More than usual effect | 3 |
| Extreme | Extraordinary effect (from critical) | 4 |

#### Dice Outcomes by Position

**Controlled Position**:
| Roll | Outcome |
|------|---------|
| Critical (6,6) | Full success with increased effect |
| 6 | Full success |
| 4-5 | Hesitate: withdraw OR do it with minor consequence |
| 1-3 | Falter: seize risky opportunity OR withdraw |

**Risky Position**:
| Roll | Outcome |
|------|---------|
| Critical (6,6) | Full success with increased effect |
| 6 | Full success |
| 4-5 | Success BUT consequence (harm, complication, reduced effect) |
| 1-3 | Failure (harm, complication, desperate position, lost opportunity) |

**Desperate Position**:
| Roll | Outcome |
|------|---------|
| Critical (6,6) | Full success with increased effect |
| 6 | Full success |
| 4-5 | Success BUT severe consequence |
| 1-3 | Worst outcome (severe harm, serious complication, lost opportunity) |

**Critical Success**: Rolling two or more 6s in the pool grants increased effect (effect level goes up one tier).

**Configuration Options**:
- `enable_critical`: Whether multiple 6s trigger critical (default: true)
- `full_success`: Die value for full success (default: 6)
- `partial_success_min`: Minimum die for partial (default: 4)
- `partial_success_max`: Maximum die for partial (default: 5)
- `effect_ticks`: Clock ticks per effect level

## Configuration Architecture

### Three-Layer Precedence

```
┌─────────────────────────────────────────────────────────────────┐
│                    Per-World Settings (SQLite)                  │
│  Stored in: world_settings table                                │
│  Highest priority - overrides all below                         │
├─────────────────────────────────────────────────────────────────┤
│                    Environment Variables                         │
│  Loaded from: .env file via WRLDBLDR_* prefix                   │
│  Overrides code defaults and rule system presets                │
├─────────────────────────────────────────────────────────────────┤
│                    Rule System Config (Neo4j)                    │
│  Stored with: World entity as narrative_config JSON             │
│  Preset values from RuleSystemVariant                           │
├─────────────────────────────────────────────────────────────────┤
│                    Code Defaults                                 │
│  Defined in: impl Default for NarrativeResolutionConfig         │
│  Base fallback values                                            │
└─────────────────────────────────────────────────────────────────┘
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `WRLDBLDR_NARRATIVE_RESOLUTION_STYLE` | (none) | Override style: `pbta`, `ladder`, `blades`, `custom` |
| `WRLDBLDR_PBTA_FULL_SUCCESS` | 10 | PbtA full success threshold |
| `WRLDBLDR_PBTA_PARTIAL_SUCCESS` | 7 | PbtA partial success threshold |
| `WRLDBLDR_FATE_DICE_COUNT` | 4 | Number of Fudge dice to roll |
| `WRLDBLDR_FATE_STYLE_THRESHOLD` | 3 | Shifts needed for succeed with style |
| `WRLDBLDR_BLADES_ENABLE_CRITICAL` | true | Enable critical on multiple 6s |

### Per-World Settings (AppSettings)

These fields in `AppSettings` allow per-world overrides:

```rust
// Narrative Resolution Overrides
pub narrative_resolution_style: Option<NarrativeResolutionStyle>,
pub pbta_full_success_threshold: Option<i32>,
pub pbta_partial_success_threshold: Option<i32>,
pub fate_dice_count: Option<u8>,
pub fate_style_threshold: Option<i32>,
pub blades_enable_critical: Option<bool>,
```

## Type Definitions

### NarrativeResolutionConfig

The main configuration struct stored with `RuleSystemConfig`:

```rust
pub struct NarrativeResolutionConfig {
    /// The narrative resolution style
    pub style: NarrativeResolutionStyle,
    /// Thresholds for PbtA/Custom styles
    pub thresholds: NarrativeThresholds,
    /// Difficulty ladder for Fate style
    pub ladder: DifficultyLadder,
    /// Dice configuration
    pub dice_config: NarrativeDiceConfig,
    /// Position/Effect config for Blades style
    pub position_effect: PositionEffectConfig,
}
```

### NarrativeResolutionStyle

```rust
pub enum NarrativeResolutionStyle {
    /// Fixed thresholds (10+/7-9/6-), 2d6+stat
    PbtA,
    /// Descriptor maps to ladder value, NdF+skill vs target
    Ladder,
    /// Position determines consequences, Effect determines progress
    Blades,
    /// User-configurable thresholds
    Custom,
}
```

### Position and EffectLevel

For Blades-style challenges, these are set by the DM before the roll:

```rust
pub enum Position {
    Controlled,  // Minor consequences
    Risky,       // Moderate consequences (default)
    Desperate,   // Severe consequences
}

pub enum EffectLevel {
    Zero,     // No progress
    Limited,  // 1 tick
    Standard, // 2 ticks (default)
    Great,    // 3 ticks
    Extreme,  // 4 ticks (from critical)
}
```

## Integration with Challenge System

### Challenge Entity Updates

The `Difficulty::Descriptor` variant uses `NarrativeResolutionConfig` for resolution:

```rust
pub fn evaluate_roll(
    &self,
    roll: i32,
    modifier: i32,
    narrative_config: Option<&NarrativeResolutionConfig>,
    position: Option<Position>,
    effect: Option<EffectLevel>,
    dice_results: Option<&[i32]>,
) -> (OutcomeType, &Outcome)
```

### Resolution Flow

1. Player initiates challenge roll
2. For Blades-style, DM sets Position and Effect before roll
3. Challenge resolution service fetches world's `NarrativeResolutionConfig`
4. Merges with any per-world setting overrides
5. Calls `Challenge::evaluate_roll()` with full context
6. Returns appropriate outcome based on resolution style

## UI Components

### DM Settings Panel

The Narrative Resolution Settings panel (DM role only) provides:

- Resolution style dropdown (PbtA, Fate/Ladder, Blades, Custom)
- PbtA thresholds (full success, partial success)
- Fate settings (dice count, style threshold, ladder editor)
- Blades settings (critical enable, die thresholds)

### Challenge Editor

For Blades-style worlds, the challenge editor shows:

- Position selector (Controlled / Risky / Desperate)
- Effect selector (Limited / Standard / Great)
- Contextual descriptions for each selection
- Dice info display showing pool mechanics

### Roll Result Display

Adapts based on resolution style:

**PbtA**: `2d6+2 = [4][3]+2 = 9 → Partial Success (7-9)`

**Fate**: `4dF+3 = [+][-][0][+]+3 = 4 vs Fair (+2) → +2 shifts → Success!`

**Blades**: `3d6 pool = [6][4][2] → Highest: 6 | Risky/Standard → Full Success`

## Default Presets by Variant

| Variant | Style | Dice | Notes |
|---------|-------|------|-------|
| PoweredByApocalypse | PbtA | 2d6 | Standard 10/7 thresholds |
| FateCore | Ladder | 4dF | Full Fate ladder |
| KidsOnBikes | Custom | Variable | Uses stat-based die size |

## Future Considerations

### Potential Extensions

1. **Custom Ladders**: Allow DMs to define entirely custom descriptor-to-value mappings
2. **Hybrid Styles**: Combine elements (e.g., PbtA thresholds with Position modifiers)
3. **Clock Integration**: Direct integration with progress clocks for Effect ticks
4. **Resistance Rolls**: Blades-style stress/resistance mechanics

### Compatibility Notes

- D20 and D100 systems continue to use existing `Difficulty::DC` and `Difficulty::Percentage`
- `Difficulty::Descriptor` triggers narrative resolution
- The system gracefully falls back to PbtA defaults if no config is provided
