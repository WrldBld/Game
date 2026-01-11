# Call of Cthulhu 7th Edition (CoC 7e) System Reference

## Overview

Call of Cthulhu is a horror investigation RPG using a percentile (d100) roll-under system. Unlike heroic fantasy games, investigators are ordinary people facing cosmic horrors, with fragile sanity and no progression through "leveling up."

## Core Mechanics

### Dice System
- **Primary Roll**: d100 (percentile dice) roll-under
- **Roll Formula**: Roll d100, compare to skill value
- **Lower is better**: Must roll equal to or under skill value

### Success Levels
Three tiers of success based on skill value:

| Level | Target | Description |
|-------|--------|-------------|
| Regular Success | ≤ skill value | Standard success |
| Hard Success | ≤ skill ÷ 2 | Difficult success |
| Extreme Success | ≤ skill ÷ 5 | Exceptional success |

**Implementation**:
```rust
fn check_success(roll: u8, skill: u8) -> SuccessLevel {
    let hard = skill / 2;
    let extreme = skill / 5;

    if roll <= extreme {
        SuccessLevel::Extreme
    } else if roll <= hard {
        SuccessLevel::Hard
    } else if roll <= skill {
        SuccessLevel::Regular
    } else {
        SuccessLevel::Failure
    }
}
```

### Critical and Fumble
- **Critical (01)**: Always succeeds, exceptional outcome
- **Fumble**: 96-100 if skill < 50, or 100 if skill ≥ 50

```rust
fn is_fumble(roll: u8, skill: u8) -> bool {
    if skill < 50 {
        roll >= 96
    } else {
        roll == 100
    }
}

fn is_critical(roll: u8) -> bool {
    roll == 1
}
```

### Pushed Rolls
- **One chance** to re-roll a failed check
- Must describe additional effort/risk
- If pushed roll fails, **consequences are worse**
- Cannot push: Combat, Sanity, Luck rolls

## Character Characteristics

### Eight Characteristics
Unlike D&D's six attributes, CoC has eight:

| Characteristic | Abbreviation | Generation | Description |
|---------------|--------------|------------|-------------|
| Strength | STR | 3d6 × 5 | Physical power |
| Constitution | CON | 3d6 × 5 | Health, vitality |
| Size | SIZ | (2d6+6) × 5 | Physical bulk |
| Dexterity | DEX | 3d6 × 5 | Agility, speed |
| Appearance | APP | 3d6 × 5 | Charisma, looks |
| Intelligence | INT | (2d6+6) × 5 | Reasoning ability |
| Power | POW | 3d6 × 5 | Willpower, magical aptitude |
| Education | EDU | (2d6+6) × 5 | Formal knowledge |

**Typical Range**: 15-90 (average human ~50)

### Derived Attributes

| Attribute | Formula | Description |
|-----------|---------|-------------|
| Hit Points | (CON + SIZ) ÷ 10 | Physical health |
| Sanity | POW | Starting mental stability |
| Magic Points | POW ÷ 5 | Magical energy |
| Luck | 3d6 × 5 | Fortune (spendable) |
| Move Rate | Based on STR, DEX, SIZ | Movement speed |

```rust
fn calculate_hp(con: u8, siz: u8) -> u8 {
    (con + siz) / 10
}

fn calculate_starting_sanity(pow: u8) -> u8 {
    pow  // Starting Sanity = POW
}

fn calculate_magic_points(pow: u8) -> u8 {
    pow / 5
}

fn calculate_move_rate(str: u8, dex: u8, siz: u8) -> u8 {
    if dex < siz && str < siz {
        7
    } else if str >= siz || dex >= siz {
        8
    } else if str > siz && dex > siz {
        9
    } else {
        8
    }
}
```

### Damage Bonus & Build

| STR + SIZ | Damage Bonus | Build |
|-----------|--------------|-------|
| 2-64 | -2 | -2 |
| 65-84 | -1 | -1 |
| 85-124 | None | 0 |
| 125-164 | +1d4 | +1 |
| 165-204 | +1d6 | +2 |
| 205-284 | +2d6 | +3 |
| 285-364 | +3d6 | +4 |
| 365+ | +4d6 | +5 |

## Skills

### Complete Skill List (with Base Values)

**Combat Skills**:
| Skill | Base % |
|-------|--------|
| Dodge | DEX/2 |
| Fighting (Brawl) | 25% |
| Fighting (specify) | 01% |
| Firearms (Handgun) | 20% |
| Firearms (Rifle/Shotgun) | 25% |
| Firearms (specify) | 01% |
| Throw | 20% |

**Investigation Skills**:
| Skill | Base % |
|-------|--------|
| Appraise | 05% |
| Library Use | 20% |
| Listen | 20% |
| Spot Hidden | 25% |
| Track | 10% |

**Social Skills**:
| Skill | Base % |
|-------|--------|
| Charm | 15% |
| Fast Talk | 05% |
| Intimidate | 15% |
| Persuade | 10% |
| Psychology | 10% |

**Knowledge Skills**:
| Skill | Base % |
|-------|--------|
| Accounting | 05% |
| Anthropology | 01% |
| Archaeology | 01% |
| Art/Craft (specify) | 05% |
| Cthulhu Mythos | 00% |
| History | 05% |
| Language (Other) | 01% |
| Language (Own) | EDU |
| Law | 05% |
| Medicine | 01% |
| Natural World | 10% |
| Navigate | 10% |
| Occult | 05% |
| Science (specify) | 01% |

**Practical Skills**:
| Skill | Base % |
|-------|--------|
| Climb | 20% |
| Drive Auto | 20% |
| Electrical Repair | 10% |
| First Aid | 30% |
| Jump | 20% |
| Locksmith | 01% |
| Mechanical Repair | 10% |
| Operate Heavy Machinery | 01% |
| Pilot (specify) | 01% |
| Ride | 05% |
| Sleight of Hand | 10% |
| Stealth | 20% |
| Survival (specify) | 10% |
| Swim | 20% |

### Occupation Skills
- Characters choose an **occupation** (e.g., Professor, Private Eye, Doctor)
- Occupation provides **skill points** = EDU × 4 (or EDU × 2 + other stat × 2)
- Points distributed among **8 occupation skills**

### Personal Interest Skills
- **INT × 2** points for any non-occupation skills
- Represents hobbies and personal knowledge

### Credit Rating
Special skill representing wealth and social status:

| Credit Rating | Lifestyle | Assets |
|--------------|-----------|--------|
| 0 | Penniless | None |
| 1-9 | Poor | ~$500 |
| 10-49 | Average | ~$10,000 |
| 50-89 | Affluent | ~$50,000 |
| 90-98 | Wealthy | ~$500,000 |
| 99 | Super Rich | Millions |

## Combat Mechanics

### Combat Flow
1. **Determine DEX order** (highest acts first)
2. **Declare actions**
3. **Resolve attacks** (skill roll vs Dodge/Fighting Back)
4. **Apply damage**

### Attack Resolution
- **Attacker rolls** Fighting or Firearms skill
- **Defender chooses**: Dodge or Fight Back
- **Compare success levels**: Higher level wins
- **Ties**: Defender wins (if dodging) or compare skill values

### Damage
```
Damage = Weapon Base + Damage Bonus (if melee)
```

**Common Weapons**:
| Weapon | Damage | Range | Attacks |
|--------|--------|-------|---------|
| Fist/Punch | 1d3 + DB | Touch | 1 |
| Knife | 1d4 + DB | Touch | 1 |
| .38 Revolver | 1d10 | 15 yards | 1 |
| Shotgun | 4d6/2d6/1d6 | 10/20/50 | 1 |

### Major Wound
If damage ≥ half max HP in one hit:
- Make a **CON roll** or fall unconscious
- Roll on major wound table for lasting effects

### Dying
- At 0 HP: Dying
- Make CON roll each round or lose 1 HP
- At negative HP equal to max HP: Dead

## Sanity System

### Starting Sanity
- **Initial**: Equal to POW
- **Maximum**: 99 - Cthulhu Mythos skill

### Sanity Rolls
When facing horror, roll d100:
- **Success** (≤ Sanity): Lose minimum Sanity
- **Failure** (> Sanity): Lose maximum Sanity

Example: "0/1d6" means lose 0 on success, 1d6 on failure

### Common Sanity Losses

| Encounter | Loss (Pass/Fail) |
|-----------|-----------------|
| Dead body | 0/1d3 |
| Grisly murder scene | 0/1d4 |
| Zombie | 0/1d8 |
| Deep One | 0/1d6 |
| Shoggoth | 1d6/1d20 |
| Great Old One | 1d10/1d100 |

### Temporary Insanity
If lose 5+ Sanity in one incident:
- Roll **INT**: Success = repress, Failure = bout of madness
- **Bout of Madness**: 1d10 rounds of temporary insanity

### Indefinite Insanity
If lose 20% of current Sanity in game hour:
- Develop **underlying insanity** (phobia, mania, etc.)
- Requires extended treatment to cure

### Cthulhu Mythos
- **Cannot be reduced** once gained
- **Reduces maximum Sanity** (Max SAN = 99 - Mythos)
- Gained from: reading tomes, casting spells, witnessing entities

## Magic System

### Magic Points
- **Equal to POW ÷ 5**
- Regenerate 1 MP per hour of rest
- Fully regenerate after 8 hours sleep

### Spells
- **Learning**: Requires time, Sanity, sometimes ritual
- **Casting**: Costs MP, sometimes Sanity, sometimes POW
- **Opposed**: Attacker's MP vs target's MP on resistance table

### Resistance Table
When opposing forces clash:
```
Base Chance = 50% + (Active - Passive) × 5
```
Example: Caster MP 12 vs Target MP 8:
- Chance = 50% + (12-8) × 5 = 70%

## Character Progression

### No Traditional Leveling
Characters don't gain XP and level up. Instead:

### Skill Improvement
During investigation:
1. GM tells player to **check** a successfully used skill
2. Between sessions, roll d100 for each checked skill
3. If roll > current skill: Add 1d10 to skill
4. If roll ≤ current skill: No improvement

```rust
fn improve_skill(current: u8) -> u8 {
    let roll = rand::random::<u8>() % 100 + 1;
    if roll > current {
        let gain = rand::random::<u8>() % 10 + 1;
        (current + gain).min(99)
    } else {
        current
    }
}
```

### Characteristic Improvement
- Rarely improves
- Sanity can be restored through therapy
- POW can increase through successful magic resistance

## Unique Features

### Luck Points
- **Spendable resource**: Spend Luck to modify rolls
- **1:1 ratio**: Spend 1 Luck to add 1 to roll (or subtract 1)
- **Reduces permanently** when spent
- Recovered between sessions (optional rule)

### Backstory Elements
Character creation includes:

| Element | Purpose |
|---------|---------|
| Ideology/Beliefs | Core values and worldview |
| Significant People | Important relationships |
| Meaningful Locations | Places of importance |
| Treasured Possessions | Valued items |
| Traits | Personality descriptors |
| Injuries & Scars | Physical marks |
| Phobias & Manias | Mental conditions |

### Pulp Cthulhu
Heroic variant with:
- **Higher HP**: (CON + SIZ) ÷ 5
- **Talents**: Special abilities
- **Hero points**: Luck-like mechanic
- More combat-capable characters

## Integration Considerations

### StatBlock Mapping
```rust
pub struct CocStatBlock {
    // Eight characteristics (percentile values)
    characteristics: HashMap<String, u8>,  // STR, CON, SIZ, DEX, APP, INT, POW, EDU

    // Skills (percentile values)
    skills: HashMap<String, u8>,

    // Current values (can change)
    current_hp: u8,
    max_hp: u8,
    current_sanity: u8,
    max_sanity: u8,
    current_mp: u8,
    current_luck: u8,

    // Cthulhu Mythos (special, reduces max sanity)
    mythos_skill: u8,

    // Backstory elements
    backstory: CocBackstory,

    // Skill improvement checks
    checked_skills: HashSet<String>,
}

pub struct CocBackstory {
    ideology: String,
    significant_people: Vec<String>,
    meaningful_locations: Vec<String>,
    treasured_possessions: Vec<String>,
    traits: Vec<String>,
    injuries: Vec<String>,
    phobias: Vec<String>,
    manias: Vec<String>,
}
```

### New Types Needed
```rust
pub enum CocSuccessLevel {
    CriticalSuccess,  // Roll of 01
    ExtremeSuccess,   // ≤ skill/5
    HardSuccess,      // ≤ skill/2
    RegularSuccess,   // ≤ skill
    Failure,          // > skill
    Fumble,           // 96-100 or 100
}

pub struct SanityCheck {
    pass_loss: DiceFormula,  // e.g., "0" or "1d3"
    fail_loss: DiceFormula,  // e.g., "1d6" or "1d20"
}
```

### Key Differences from D&D-style Games
1. **Roll-under vs Roll-over**: Lower is better
2. **Percentile skills**: 1-100 scale, not modifiers
3. **No HP scaling**: Characters remain fragile
4. **Sanity resource**: Unique mental health system
5. **No leveling**: Skill-based improvement only
6. **Investigation focus**: Combat is dangerous, not heroic
7. **Luck spending**: Permanent resource consumption
8. **Eight characteristics**: Different from D&D's six
9. **Opposed roll table**: Different from contested checks
