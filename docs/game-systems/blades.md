# Blades in the Dark System Reference

## Overview

Blades in the Dark is a heist-focused RPG using a d6 dice pool system with position/effect mechanics. Set in a haunted industrial city, it emphasizes flashbacks, crew dynamics, and fiction-first play. Characters are scoundrels pulling dangerous scores.

## Core Mechanics

### Dice System
- **Primary Roll**: Dice pool of d6s (based on action rating)
- **Take Highest**: Roll pool, keep single highest die
- **No Modifiers**: Position/Effect change outcomes, not dice

### Zero Dice Rule
If you have 0 dice in your pool:
- Roll 2d6 and take the **lowest** result

### Result Interpretation
| Highest Die | Outcome |
|-------------|---------|
| 1-3 | **Failure** - Things go badly, face full consequence |
| 4-5 | **Partial Success** - You do it, but there's a complication |
| 6 | **Full Success** - You achieve your goal cleanly |
| Multiple 6s | **Critical** - Success with increased effect |

```rust
fn resolve_blades_roll(dice_results: &[u8]) -> BladesOutcome {
    let highest = *dice_results.iter().max().unwrap_or(&0);
    let sixes = dice_results.iter().filter(|&&d| d == 6).count();

    if sixes >= 2 {
        BladesOutcome::Critical
    } else if highest == 6 {
        BladesOutcome::Success
    } else if highest >= 4 {
        BladesOutcome::PartialSuccess
    } else {
        BladesOutcome::Failure
    }
}
```

### Position (Risk Level)
Determines consequence severity:

| Position | Description | Typical Consequences |
|----------|-------------|---------------------|
| **Controlled** | You act on your terms | Minor: reduced effect, complication, worse position |
| **Risky** | Standard danger | Moderate: harm, serious complication, worse position |
| **Desperate** | Serious trouble | Severe: severe harm, lost opportunity, major complication |

### Effect Level (Impact)
Determines success magnitude:

| Effect | Clock Ticks | Description |
|--------|-------------|-------------|
| Zero | 0 | No meaningful progress |
| Limited | 1 | Partial/weak effect |
| Standard | 2 | Normal expected outcome |
| Great | 3 | More than usual |
| Extreme | 4 | Extraordinary (from critical) |

## Character Attributes

### Three Attributes
Characters have 12 actions grouped into 3 attributes:

**Insight** (Mental/Perceptive):
| Action | Description |
|--------|-------------|
| Hunt | Track, chase, notice details |
| Study | Scrutinize, research, interpret |
| Survey | Observe, anticipate, case |
| Tinker | Fiddle, create, repair, disable |

**Prowess** (Physical/Athletic):
| Action | Description |
|--------|-------------|
| Finesse | Dexterity, misdirection, precision |
| Prowl | Stealth, climbing, quiet movement |
| Skirmish | Close combat, melee fighting |
| Wreck | Smash, demolish, brute force |

**Resolve** (Social/Supernatural):
| Action | Description |
|--------|-------------|
| Attune | Supernatural, spirits, arcane |
| Command | Order, intimidate, lead |
| Consort | Socialize, network, blend in |
| Sway | Persuade, deceive, charm |

### Action Ratings (0-4)
| Rating | Dice Rolled |
|--------|-------------|
| 0 | Roll 2d6, take lowest |
| 1 | 1d6 |
| 2 | 2d6 |
| 3 | 3d6 |
| 4 | 4d6 (maximum) |

### Attribute Ratings (for Resistance)
Sum of action dots in that attribute:
```rust
fn insight_rating(char: &Character) -> u8 {
    char.hunt + char.study + char.survey + char.tinker
}

fn prowess_rating(char: &Character) -> u8 {
    char.finesse + char.prowl + char.skirmish + char.wreck
}

fn resolve_rating(char: &Character) -> u8 {
    char.attune + char.command + char.consort + char.sway
}
```

## Playbooks (Character Types)

### Seven Core Playbooks
| Playbook | Role | Starting Action | XP Trigger |
|----------|------|-----------------|------------|
| **Cutter** | Fighter | Skirmish 2 | Violence or coercion |
| **Hound** | Sharpshooter | Hunt 2 | Tracking or violence |
| **Leech** | Technician | Wreck 2 | Technical skill or mayhem |
| **Lurk** | Infiltrator | Prowl 2 | Stealth or evasion |
| **Slide** | Manipulator | Sway 2 | Deception or influence |
| **Spider** | Mastermind | Study 2 | Calculation or conspiracy |
| **Whisper** | Channeler | Attune 2 | Knowledge or arcane power |

### Special Abilities
- Each playbook has **8 unique abilities**
- Start with **1 ability**
- Gain more via XP advancement
- Can take from other playbooks at +1 XP cost

## Stress and Trauma

### Stress Track (0-9)
**Gaining Stress**:
| Action | Stress Cost |
|--------|-------------|
| Push Yourself | 2 stress (+1d or +1 effect) |
| Assist Another | 1 stress |
| Protect Teammate | Consequence + 1 stress |
| Resistance Roll | 6 - successes (minimum 0) |

### Resistance Rolls
When facing a consequence:
1. Choose relevant attribute (Insight, Prowess, Resolve)
2. Roll dice = attribute rating
3. Count 6s rolled
4. Stress cost = 6 - (number of 6s)
5. Consequence is reduced/avoided

```rust
fn resistance_roll(attribute_rating: u8, dice_results: &[u8]) -> ResistanceResult {
    let sixes = dice_results.iter().filter(|&&d| d == 6).count();
    let stress_cost = (6 - sixes as i32).max(0) as u8;

    ResistanceResult {
        stress_cost,
        consequence_reduced: true,
    }
}
```

### Trauma
**When Triggered**: Stress reaches 9, then increases
- Reset stress to 0
- Mark one trauma condition

**Trauma Conditions**:
- **Cold**: Unmoved by emotion
- **Haunted**: Lost in past
- **Obsessed**: Single focus
- **Paranoid**: Trust no one
- **Reckless**: Disregard safety
- **Soft**: Lost your edge
- **Unstable**: Volatile emotions
- **Vicious**: Seeks to hurt

**At 4 Trauma**: Character retires

## Harm System

### Harm Levels
| Level | Slots | Effect | Examples |
|-------|-------|--------|----------|
| 1 (Lesser) | 2 | Narrative only | Battered, Drained |
| 2 (Moderate) | 2 | -1d to related actions | Exhausted, Deep Cut |
| 3 (Severe) | 1 | -1d (stacks) | Broken Leg, Shot |
| 4 (Fatal) | 1 | Dying/Dead | Needs immediate help |

### Overflow
If a harm level is full, next harm "rolls up":
- Level 1 full → becomes Level 2
- Level 2 full → becomes Level 3
- Level 3 full → becomes Level 4 (fatal)

### Armor
- Standard Armor: Reduce harm by 1 level (mark armor box)
- Heavy Armor: Reduce harm by 2 levels (mark 2 boxes)
- Special Armor: From abilities, vs specific threats

### Recovery
| Harm Level | Natural Healing |
|------------|-----------------|
| Level 1 | 1-2 days |
| Level 2 | 1 week |
| Level 3 | 1 month (with care) |
| Level 4 | Immediate treatment or death |

## Load System

### Load Levels
| Level | Items | Penalty |
|-------|-------|---------|
| Light | 3 | None |
| Normal | 5 | None |
| Heavy | 6 | -1d to Prowess actions |

### Key Rule: Declare Items During Play
You don't pre-select specific items:
1. Choose load level before score
2. During score, declare "I have a..." when needed
3. Mark load equal to item cost
4. Cannot exceed chosen load level

```rust
pub struct CharacterLoad {
    load_level: LoadLevel,
    items_declared: Vec<LoadItem>,
    current_load: u8,
}

impl CharacterLoad {
    pub fn declare_item(&mut self, item: LoadItem) -> Result<()> {
        let new_total = self.current_load + item.cost;
        if new_total > self.load_level.max() {
            return Err(Error::ExceedsLoadLevel);
        }
        self.items_declared.push(item);
        self.current_load = new_total;
        Ok(())
    }
}
```

### Common Items
| Item | Load |
|------|------|
| Blade, Pistol | 1 |
| Large Weapon | 2 |
| Armor | 1 |
| Heavy Armor | 3 |
| Burglary Gear | 1 |
| Climbing Gear | 2 |
| Arcane Implements | 1 |
| Demolitions | 2 |

## Crew Mechanics

### Crew Types
| Type | Focus | Hunting Grounds |
|------|-------|-----------------|
| **Assassins** | Murder for hire | Killings |
| **Bravos** | Thugs | Extortion, sabotage |
| **Cult** | Occultists | Occult operations |
| **Hawkers** | Dealers | Product sales |
| **Shadows** | Thieves | Burglary, espionage |
| **Smugglers** | Transporters | Smuggling routes |

### Crew Stats
- **Reputation** (0-12): Power/influence tier
- **Heat** (0-9): Attention from authorities
- **Wanted Level** (0-4): How hunted you are
- **Coin**: Shared treasury
- **Turf**: Controlled territory (+2 coin per score each)
- **Claims**: Special locations with benefits

### Heat and Wanted
**Gaining Heat** (after each score):
| Situation | Heat |
|-----------|------|
| Smooth & quiet | 0 |
| Contained incident | 2 |
| Loud & messy | 4 |
| Spectacular disaster | 6 |

**When Heat fills (9→10)**:
- Roll to potentially raise Wanted Level
- Clear heat track

**At Wanted Level 4**: Hunted by special forces

### Entanglements
After each score, roll for entanglement based on wanted level:
- Gang Trouble, Rivals, Unquiet Dead
- Bluecoat Raids, Interrogations
- Opportunities mixed with complications

## Unique Mechanics

### Flashbacks
**Purpose**: Avoid lengthy planning, keep action moving

**How It Works**:
1. During score, declare "Earlier, I prepared for this..."
2. Pay stress based on complexity:
   - 0 stress: Simple, obvious prep
   - 1 stress: Complex or unlikely
   - 2 stress: Elaborate or improbable
3. Might require action roll in flashback
4. Return to present with advantage

**Limits**:
- Can't contradict established facts
- Can't undo suffered consequences

### Devil's Bargains
**Offered by GM during action rolls**:
- Accept: +1d to roll, complication happens regardless
- Refuse: No extra die

**Examples**:
- "+1d, but leave evidence behind"
- "+1d, but owe dangerous favor"
- "+1d, but rival crew notices"

### Clocks (Progress Tracking)
**Progress Clocks**: Circles with 4, 6, or 8 segments

**Uses**:
- Obstacle Clocks: "Pick the Lock" (6 segments)
- Project Clocks: "Craft Device" (8 segments)
- Danger Clocks: "Guards Arrive" (4 segments)
- Racing Clocks: Competing progress

**Filling Clocks**:
- Success fills segments = effect level
- Limited: 1 tick, Standard: 2 ticks, Great: 3 ticks
- Critical: +1 extra tick

```rust
pub struct ProgressClock {
    pub name: String,
    pub segments: u8,     // 4, 6, or 8
    pub filled: u8,
    pub clock_type: ClockType,
}

pub enum ClockType {
    Progress,   // Player filling
    Danger,     // GM filling
    Racing,     // Both competing
}
```

## Downtime

### Downtime Actions (2 per character)

**Acquire Asset**: Get item/resource/information
- Roll tier or contact quality
- Each 6 = 1 use of asset

**Long-Term Project**: Progress on extended goal
- Roll appropriate action
- Each 6 = 1 tick on project clock

**Recover**: Heal harm
- Roll Tinker (or friend helps)
- Each 6 = clear 1 harm level

**Reduce Heat**: Lay low
- Automatic -2 heat (no roll)

**Train**: Mark XP
- +1 XP in chosen track
- Can spend coin for more XP

**Indulge Vice**: Clear stress
- Roll lowest attribute rating
- Clear stress = highest die result
- Risk: Overindulge on 1-3

### Vice and Overindulgence
**Vice Types**: Luxury, Stupor, Obligation, Pleasure, Gambling, Faith, Weird

**Overindulgence Results** (when highest die is 1-3):
- Attract unwanted attention
- Brag about crimes
- Lost for days
- Tapped (out of cash)

## Engagement Roll

Before each score:
1. Crew chooses approach
2. Detail plan in one sentence
3. Roll 1d6 + crew tier ± modifiers

| Result | Starting Position |
|--------|-------------------|
| Critical | Controlled with opportunity |
| 6 | Controlled |
| 4-5 | Risky |
| 1-3 | Desperate |

## XP and Advancement

### XP Triggers
**Per Attribute** (when used desperately):
- 1 XP for Insight, Prowess, or Resolve actions in desperate position

**Playbook XP**:
- Up to 3 XP per session for playbook-specific trigger
- Express beliefs, drives, heritage, background
- Struggle with vice or trauma

### Spending XP
| Advancement | Cost |
|-------------|------|
| New action dot | 1 XP (first), 2 XP (second), etc. |
| New special ability | Varies (1-8 XP) |
| Other playbook ability | +1 XP extra |

## Integration Considerations

### StatBlock Mapping
```rust
pub struct BladesStatBlock {
    // 12 Actions (0-4 each)
    pub hunt: u8,
    pub study: u8,
    pub survey: u8,
    pub tinker: u8,
    pub finesse: u8,
    pub prowl: u8,
    pub skirmish: u8,
    pub wreck: u8,
    pub attune: u8,
    pub command: u8,
    pub consort: u8,
    pub sway: u8,

    // Derived attribute ratings
    pub insight: u8,   // Sum of insight actions
    pub prowess: u8,   // Sum of prowess actions
    pub resolve: u8,   // Sum of resolve actions

    // Stress/Trauma
    pub stress: u8,    // 0-9
    pub trauma: u8,    // 0-4
    pub trauma_conditions: Vec<TraumaCondition>,

    // Harm
    pub harm_1: [Option<String>; 2],
    pub harm_2: [Option<String>; 2],
    pub harm_3: Option<String>,

    // Armor
    pub armor_used: u8,
    pub armor_max: u8,
}
```

### Position/Effect System
```rust
pub enum Position {
    Controlled,
    Risky,
    Desperate,
}

pub enum EffectLevel {
    Zero,
    Limited,
    Standard,
    Great,
    Extreme,
}

pub struct ActionContext {
    pub position: Position,
    pub effect: EffectLevel,
    pub action: String,
    pub dice_pool: u8,
}
```

### Key Differences from D20 Systems
1. **Dice Pool**: Multiple d6s, take highest (not d20 + modifier)
2. **No Target Numbers**: Outcomes based on die faces, not DC
3. **Position/Effect**: Separate from dice pool
4. **Stress as Resource**: Spend to boost, not just damage
5. **Flashbacks**: Retroactive preparation during play
6. **Load Declaration**: Equipment decided during play
7. **Crew Mechanics**: Shared group character
8. **Clocks**: Visual progress tracking
9. **Fiction-First**: Narrative determines mechanics
