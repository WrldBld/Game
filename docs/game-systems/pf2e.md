# Pathfinder 2nd Edition (PF2e) System Reference

## Overview

Pathfinder 2e is a d20-based fantasy RPG with four degrees of success, a three-action economy, and a granular proficiency system. It builds on D&D traditions but with significant mechanical innovations.

## Core Mechanics

### Dice System
- **Primary Roll**: d20 + modifiers vs DC (Difficulty Class)
- **Check Formula**: `d20 + ability modifier + proficiency bonus + item bonus + circumstance bonus + status bonus`

### Degrees of Success
Unlike D&D 5e's binary success/failure, PF2e has four outcomes:

| Outcome | Condition |
|---------|-----------|
| Critical Success | Beat DC by 10+ OR natural 20 that succeeds |
| Success | Meet or beat DC |
| Failure | Below DC |
| Critical Failure | Miss DC by 10+ OR natural 1 that fails |

**Implementation Formula**:
```rust
fn determine_success(roll: i32, modifier: i32, dc: i32) -> DegreeOfSuccess {
    let total = roll + modifier;
    let diff = total - dc;

    let base_success = if diff >= 0 { DegreeOfSuccess::Success }
                       else { DegreeOfSuccess::Failure };

    // Apply +/- 10 rule
    let adjusted = if diff >= 10 { base_success.upgrade() }
                   else if diff <= -10 { base_success.downgrade() }
                   else { base_success };

    // Natural 20/1 adjustments
    if roll == 20 { adjusted.upgrade() }
    else if roll == 1 { adjusted.downgrade() }
    else { adjusted }
}
```

## Character Attributes

### Ability Scores
Same six abilities as D&D, but different calculation:
- **STR** (Strength), **DEX** (Dexterity), **CON** (Constitution)
- **INT** (Intelligence), **WIS** (Wisdom), **CHA** (Charisma)

### Ability Modifier Calculation
**Same as D&D 5e**: `modifier = floor((score - 10) / 2)`

| Score | Modifier |
|-------|----------|
| 1 | -5 |
| 8-9 | -1 |
| 10-11 | 0 |
| 12-13 | +1 |
| 14-15 | +2 |
| 16-17 | +3 |
| 18-19 | +4 |
| 20+ | +5+ |

### Key Ability by Class
Each class has a key ability score:

| Class | Key Ability |
|-------|-------------|
| Alchemist | INT |
| Barbarian | STR |
| Bard | CHA |
| Champion | STR or DEX |
| Cleric | WIS |
| Druid | WIS |
| Fighter | STR or DEX |
| Investigator | INT |
| Magus | STR or DEX |
| Monk | STR or DEX |
| Oracle | CHA |
| Ranger | STR or DEX |
| Rogue | DEX |
| Sorcerer | CHA |
| Summoner | CHA |
| Swashbuckler | DEX |
| Witch | INT |
| Wizard | INT |

## Proficiency System

### Proficiency Ranks
Five ranks provide increasing bonuses:

| Rank | Bonus | Training Requirements |
|------|-------|----------------------|
| Untrained | 0 | - |
| Trained | +2 + level | Basic training |
| Expert | +4 + level | Advanced training |
| Master | +6 + level | Mastery |
| Legendary | +8 + level | Ultimate mastery |

### Proficiency Bonus Formula
```rust
fn proficiency_bonus(level: u8, rank: ProficiencyRank) -> i32 {
    let rank_bonus = match rank {
        ProficiencyRank::Untrained => 0,
        ProficiencyRank::Trained => 2,
        ProficiencyRank::Expert => 4,
        ProficiencyRank::Master => 6,
        ProficiencyRank::Legendary => 8,
    };

    if rank == ProficiencyRank::Untrained {
        0  // Untrained doesn't add level
    } else {
        rank_bonus + level as i32
    }
}
```

### What Can Be Proficient In
- **Skills** (16 core skills + Lore)
- **Weapons** (by category: simple, martial, advanced, specific)
- **Armor** (unarmored, light, medium, heavy)
- **Saving Throws** (Fortitude, Reflex, Will)
- **Perception** (special: used for initiative)
- **Spells** (spell attacks and DCs)
- **Class DC** (used for class abilities)

## Skills

### Complete Skill List (16 Skills)

| Skill | Ability | Common Uses |
|-------|---------|-------------|
| Acrobatics | DEX | Balance, Tumble Through, Maneuver in Flight |
| Arcana | INT | Recall Knowledge (arcane), Identify Magic |
| Athletics | STR | Climb, Swim, Grapple, Shove, Trip, Jump |
| Crafting | INT | Craft items, Repair, Identify Alchemy |
| Deception | CHA | Lie, Feint, Create a Diversion |
| Diplomacy | CHA | Gather Information, Make an Impression, Request |
| Intimidation | CHA | Coerce, Demoralize |
| Lore | INT | Recall Knowledge (specific topic) |
| Medicine | WIS | Treat Wounds, First Aid, Treat Disease |
| Nature | WIS | Recall Knowledge (nature), Command an Animal |
| Occultism | INT | Recall Knowledge (occult), Identify Magic |
| Performance | CHA | Perform |
| Religion | WIS | Recall Knowledge (religion), Identify Magic |
| Society | INT | Recall Knowledge (society), Create Forgery, Subsist |
| Stealth | DEX | Hide, Sneak, Conceal an Object |
| Survival | WIS | Track, Sense Direction, Subsist |
| Thievery | DEX | Pick Lock, Pick Pocket, Disable Device |

### Lore Skills
- Subcategory of INT-based skills
- Specific topics: "Bardic Lore", "Underworld Lore", "Sailing Lore"
- Always Trained at minimum

### Key Skill Actions
- **Recall Knowledge**: Identify creatures, remember facts
- **Treat Wounds**: 10-minute activity to heal (Medicine)
- **Demoralize**: Frighten enemies (Intimidation)

## Combat Mechanics

### Three-Action Economy
Each turn, characters get **3 actions** plus **1 reaction**:
- Most activities cost 1 action (Strike, Step, Interact)
- Some cost 2 actions (many spells, special activities)
- Some cost 3 actions (some powerful abilities)

**Action Types**:
- **Single Action** (◆): Standard action
- **Two Actions** (◆◆): Takes 2 of your 3 actions
- **Three Actions** (◆◆◆): Entire turn
- **Free Action** (◇): No action cost
- **Reaction** (⤾): Once per round, triggered

### Multiple Attack Penalty (MAP)
Subsequent attacks in a turn get penalties:

| Attack | Penalty | With Agile Weapon |
|--------|---------|-------------------|
| First | 0 | 0 |
| Second | -5 | -4 |
| Third+ | -10 | -8 |

### Attack Roll Formula
```
Attack Roll = d20 + ability modifier + proficiency bonus + item bonus
```

### AC Calculation
```
AC = 10 + DEX modifier + proficiency bonus + armor item bonus + armor's DEX cap
```

### Damage Formula
```
Damage = weapon dice + ability modifier + weapon specialization + runes
```

## Magic System

### Spell Traditions
Four traditions, each with a defining trait:

| Tradition | Key Ability | Typical Casters |
|-----------|-------------|-----------------|
| Arcane | INT | Wizard, Magus, Witch |
| Divine | WIS/CHA | Cleric, Champion |
| Occult | CHA/INT | Bard, Psychic |
| Primal | WIS | Druid, Ranger |

### Spell Slots
Similar to D&D, with cantrips scaling automatically:

| Level | Cantrip Damage | Spell Levels Available |
|-------|---------------|------------------------|
| 1 | 1d4/1d6 | 1st |
| 3 | 2d4/2d6 | 1st-2nd |
| 5 | 3d4/3d6 | 1st-3rd |
| 7 | 4d4/4d6 | 1st-4th |
| ... | ... | ... |
| 19 | 10d4/10d6 | 1st-10th |

### Heightening Spells
Spells can be cast at higher levels for increased effect:
- **Heightened (+X)**: Scales every X levels
- **Heightened (Xth)**: Specific bonuses at level X

### Spell DC and Attack
```
Spell DC = 10 + spellcasting ability modifier + proficiency bonus + item bonus
Spell Attack = d20 + spellcasting ability modifier + proficiency bonus + item bonus
```

### Focus Spells
- Separate pool (Focus Points, max 3)
- Refocus activity restores 1 point (10 minutes)
- Powerful class-specific abilities

## Character Progression

### Leveling (1-20)
At each level, characters gain:
- **Hit Points**: Class HP + CON modifier
- **Proficiency increases** (automatic and chosen)
- **Feats** (multiple types)
- **Ability Boosts** (at levels 5, 10, 15, 20)
- **Skill Increases** (every level from 3+)

### Feat Types

| Feat Type | Gained At |
|-----------|-----------|
| Ancestry Feats | 1, 5, 9, 13, 17 |
| Class Feats | 1, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20 |
| Skill Feats | 2, 4, 6, 8, 10, 12, 14, 16, 18, 20 |
| General Feats | 3, 7, 11, 15, 19 |

## Unique Features

### Hero Points
- Start each session with 1
- Max of 3
- Can spend 1 to reroll a check
- Can spend all to avoid death

### Bulk System
Encumbrance simplified:
- Items have Bulk values (L = light, 1, 2, etc.)
- 10 L items = 1 Bulk
- **Encumbered**: Carrying 5 + STR mod Bulk
- **Maximum**: 10 + STR mod Bulk

### Conditions
Standardized condition system with severity levels:
- **Clumsy 1-4**: Penalty to DEX-based checks
- **Drained 1-4**: Penalty to CON-based checks, reduces max HP
- **Enfeebled 1-4**: Penalty to STR-based checks
- **Frightened 1-4**: Penalty to all checks
- **Sickened 1-4**: Penalty to all checks
- **Stupefied 1-4**: Penalty to INT/WIS/CHA-based checks

### Dying and Wounded
- **Dying 1-4**: At 0 HP, must make recovery checks
- **Wounded**: Increases dying value when knocked out
- **Dying 4**: Character dies
- **Recovery Check**: DC 10 + dying value

## Integration Considerations

### StatBlock Mapping
```rust
pub struct Pf2eStatBlock {
    // Ability scores (same as D&D)
    abilities: HashMap<String, i32>,  // STR, DEX, CON, INT, WIS, CHA

    // Proficiency ranks (new)
    proficiencies: HashMap<String, ProficiencyRank>,

    // Level (critical for calculations)
    level: u8,

    // Conditions with values
    conditions: HashMap<String, u8>,

    // Hero Points
    hero_points: u8,

    // Bulk carried
    bulk: f32,
}
```

### New Types Needed
```rust
pub enum ProficiencyRank {
    Untrained,
    Trained,
    Expert,
    Master,
    Legendary,
}

pub enum DegreeOfSuccess {
    CriticalSuccess,
    Success,
    Failure,
    CriticalFailure,
}
```

### Key Differences from D&D 5e
1. **Proficiency is level-dependent** (level + rank bonus vs flat bonus)
2. **Four degrees of success** vs binary
3. **Three actions per turn** vs action/bonus action/movement
4. **Multiple Attack Penalty** vs flat Extra Attack
5. **Conditions have numeric values** vs binary conditions
6. **Heightening spells** vs upcasting
7. **Focus spells** (separate resource) vs spell slots only
