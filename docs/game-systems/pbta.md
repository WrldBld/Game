# Powered by the Apocalypse (PbtA) System Reference

## Overview

Powered by the Apocalypse (PbtA) is a family of narrative RPGs derived from Apocalypse World. The system uses 2d6 + stat with move-based resolution. Each game in the family (Apocalypse World, Dungeon World, Monster of the Week, etc.) adapts the core engine to its genre.

## Core Mechanics

### Dice System
- **Primary Roll**: 2d6 + stat modifier
- **Result Ranges**: 6-, 7-9, 10+
- **No Modifiers Stack**: Just stat + roll

### Three-Tier Outcomes
| Roll | Outcome | Description |
|------|---------|-------------|
| 10+ | **Full Success** | You do it without complications |
| 7-9 | **Partial Success** | You do it with a cost, complication, or reduced effect |
| 6- | **Miss** | GM makes a move (usually bad for you) |

```rust
fn resolve_pbta_roll(dice_sum: u8, stat_modifier: i32) -> PbtaOutcome {
    let total = dice_sum as i32 + stat_modifier;

    if total >= 10 {
        PbtaOutcome::FullSuccess
    } else if total >= 7 {
        PbtaOutcome::PartialSuccess
    } else {
        PbtaOutcome::Miss
    }
}
```

### The Fiction First
- Describe what you're doing in fiction
- GM determines if it triggers a move
- Roll only when a move triggers
- Results interpreted through fiction

## Stats (Vary by Game)

### Apocalypse World Stats
| Stat | Description |
|------|-------------|
| **Cool** | Calmness under pressure |
| **Hard** | Violence and intimidation |
| **Hot** | Seduction and manipulation |
| **Sharp** | Perception and cunning |
| **Weird** | Psychic and unnatural |

### Dungeon World Stats
| Stat | Description |
|------|-------------|
| **STR** | Physical power |
| **DEX** | Agility and reflexes |
| **CON** | Endurance and health |
| **INT** | Knowledge and reason |
| **WIS** | Perception and willpower |
| **CHA** | Force of personality |

### Monster of the Week Stats
| Stat | Description |
|------|-------------|
| **Charm** | Manipulation and social |
| **Cool** | Calm, collected, sneaky |
| **Sharp** | Thinking and noticing |
| **Tough** | Fighting and strength |
| **Weird** | Supernatural abilities |

### Stat Ranges
Most PbtA games use: **-1 to +3** (occasionally -2 or +4)

```rust
pub struct PbtaStats {
    pub stats: HashMap<String, i8>,  // Typically -2 to +4
}

impl PbtaStats {
    pub fn get_modifier(&self, stat: &str) -> i8 {
        self.stats.get(stat).copied().unwrap_or(0)
    }
}
```

## Moves

### What Are Moves?
Moves are the core mechanic - specific actions that trigger dice rolls:
- Have a **trigger** (fiction that activates them)
- Require a **roll** (usually 2d6 + stat)
- Have **outcomes** for 10+, 7-9, and sometimes 6-

### Basic Moves (Core to Most PbtA)

**Act Under Pressure** (Apocalypse World) / **Defy Danger** (Dungeon World):
```
When you act under fire/defy danger, roll +Cool/+relevant stat.
• 10+: You do it
• 7-9: You stumble, hesitate, or flinch - GM offers worse outcome, hard bargain, or ugly choice
• 6-: GM makes a move
```

**Go Aggro** / **Hack and Slash**:
```
When you attack an enemy in melee, roll +Hard/+STR.
• 10+: You deal your damage and avoid their attack
• 7-9: You deal damage but expose yourself to their attack
• 6-: GM makes a move
```

**Read a Situation** / **Discern Realities**:
```
When you assess a situation, roll +Sharp/+WIS.
• 10+: Ask 3 questions from the list
• 7-9: Ask 1 question
Questions: What happened here? What is about to happen?
What should I be on the lookout for? etc.
```

**Read a Person**:
```
When you read a person, roll +Sharp.
• 10+: Ask 3 questions, take +1 forward when acting on answers
• 7-9: Ask 1 question
Questions: Is your character telling the truth?
What does your character intend to do?
What does your character wish I'd do? etc.
```

**Manipulate/Seduce**:
```
When you try to manipulate someone, roll +Hot/+CHA.
• 10+: They do what you want if you give them what they want
• 7-9: They'll do it, but need concrete assurance, a bribe, or a promise
• 6-: GM makes a move
```

### Move Structure
```rust
pub struct PbtaMove {
    pub id: String,
    pub name: String,
    pub trigger: String,           // "When you..."
    pub stat: String,              // Stat to roll with
    pub full_success: String,      // 10+ outcome
    pub partial_success: String,   // 7-9 outcome
    pub miss: Option<String>,      // 6- (often "GM makes a move")
    pub options: Vec<MoveOption>,  // For moves with choices
}

pub struct MoveOption {
    pub text: String,
    pub available_at: Vec<Outcome>, // Which results can pick this
}
```

### Playbook Moves
Each character type (playbook) has unique moves:
- Start with 2-3 playbook moves
- Gain more through advancement
- Define what makes each playbook special

## Playbooks (Character Types)

### What's a Playbook?
A playbook is a character archetype that defines:
- **Stats Array**: How stats are distributed
- **Moves**: Special abilities unique to this type
- **Look**: Appearance options
- **Gear**: Starting equipment
- **Advancement**: How they grow

### Example: Dungeon World Playbooks
| Playbook | Role | Key Moves |
|----------|------|-----------|
| Fighter | Warrior | Bend Bars Lift Gates, Signature Weapon |
| Wizard | Spellcaster | Spellbook, Cast a Spell, Ritual |
| Cleric | Divine agent | Deity, Cast a Spell, Turn Undead |
| Thief | Stealth expert | Backstab, Tricks of the Trade |
| Ranger | Wilderness expert | Animal Companion, Hunt and Track |
| Bard | Social expert | Arcane Art, Charming and Open |
| Druid | Shapeshifter | Shapeshifter, Born of the Soil |
| Paladin | Holy warrior | Lay on Hands, Quest |

### Example: Monster of the Week Playbooks
| Playbook | Concept |
|----------|---------|
| The Chosen | Destined hero |
| The Crooked | Criminal with a heart |
| The Divine | Angelic warrior |
| The Expert | Knowledgeable mentor |
| The Flake | Conspiracy theorist |
| The Mundane | Normal person |
| The Professional | Agency operative |
| The Spell-Slinger | Practicing wizard |
| The Spooky | Dark-powered |
| The Wronged | Vengeance seeker |

## Harm System

### Harm Tracks (Vary by Game)

**Apocalypse World** (Harm Clock):
- 6-segment clock
- At 6:00: Rolling +Harm
- Past 6:00: Dying
- At 12:00: Dead

**Dungeon World** (HP-based):
- HP = Constitution + Class Base
- Damage reduces HP
- At 0 HP: Last Breath move

**Monster of the Week** (Harm Track):
- 7 boxes (0-7 harm)
- 4+ harm: Unstable (need help)
- 7 harm: Dying
- 8+ harm: Dead

```rust
pub enum HarmSystem {
    Clock {
        segments: u8,
        current: u8,
    },
    HitPoints {
        max: u8,
        current: u8,
    },
    HarmTrack {
        boxes: u8,
        filled: u8,
        unstable_at: u8,
    },
}
```

### Healing
Varies by game but usually:
- Rest heals some harm
- Moves like "Heal" or "First Aid"
- Conditions may need specific treatment

## Forward and Ongoing

### +1 Forward
- Bonus to your **next** roll (one time)
- Then it's gone
- "Take +1 forward to Act Under Fire"

### +1 Ongoing
- Bonus to **all** rolls for a condition
- Until condition ends
- "Take +1 ongoing while in your sanctuary"

### -1 Forward/Ongoing
- Penalty works the same way
- "Take -1 forward from your wound"

```rust
pub struct RollModifiers {
    pub forward: i8,        // Consumed after next roll
    pub ongoing: Vec<OngoingModifier>,
}

pub struct OngoingModifier {
    pub value: i8,
    pub condition: String,
    pub applicable_to: Vec<String>,  // Stats or moves
}
```

## Hold

### What is Hold?
Some moves grant "hold" - currency to spend on options:
- "On a 10+, hold 3. On a 7-9, hold 1."
- "Spend hold 1-for-1 to ask questions"
- Hold persists until spent or situation changes

```rust
pub struct MoveHold {
    pub move_id: String,
    pub amount: u8,
    pub options: Vec<HoldOption>,
}

pub struct HoldOption {
    pub cost: u8,
    pub effect: String,
}
```

## GM Principles and Moves

### MC/GM Principles
Core guidelines for running PbtA:
- Make the world seem real
- Make the characters' lives not boring
- Play to find out what happens
- Address yourself to the characters, not players
- Make your move, but never speak its name
- Be a fan of the players' characters

### GM Moves (Hard Moves on 6-)
When players miss, GM makes a move:
- Separate them
- Put someone in a spot
- Announce future badness
- Deal damage
- Take away their stuff
- Make them buy with cost
- Turn their move back on them
- Offer opportunity with or without cost
- Tell consequences and ask
- Make a threat real

### Soft vs Hard Moves
- **Soft Move**: Sets up future danger, gives warning
- **Hard Move**: Immediate consequence, no warning
- On 6-: GM can make a hard move
- On 7-9: Usually soft move as complication

## Advancement

### Experience (XP)
Different games track XP differently:

**Apocalypse World**:
- Mark XP when you roll highlighted stats
- Mark XP from session-end questions
- At 5 XP: Advance

**Dungeon World**:
- Mark XP on miss (6-)
- Session-end XP from bonds and questions
- XP to level = current level + 7

**Monster of the Week**:
- Mark XP from experience moves
- At 5 XP: Advance

### Advancement Options (Typical)
- Get +1 to a stat (max +3)
- Get a new move from your playbook
- Get a move from another playbook
- Get an ally or special equipment
- Change to a new playbook (advanced option)

```rust
pub struct PbtaAdvancement {
    pub xp_current: u8,
    pub xp_to_advance: u8,
    pub advancements_taken: Vec<Advancement>,
}

pub enum Advancement {
    StatIncrease { stat: String },
    NewMove { move_id: String },
    CrossPlaybookMove { playbook: String, move_id: String },
    Special { description: String },
}
```

## Bonds/Relationships

### What Are Bonds?
Connections between PCs that:
- Define starting relationships
- Provide mechanical benefits
- Can be resolved for XP

### Bond Examples (Dungeon World)
- "_____ has my back when things go wrong."
- "I worry about ___'s ability to survive."
- "_____ knows incriminating details about me."

### Bond Mechanics
- Start with 2-4 bonds with other PCs
- When you resolve a bond: Mark XP, write new bond
- Bonds affect Aid/Interfere moves

## Conditions (Some Games)

### Alternative to Harm
Some PbtA games use conditions instead of harm:

**Masks** (Teen Superhero) Conditions:
- Afraid, Angry, Guilty, Hopeless, Insecure
- Each gives -2 to specific actions
- Cleared through roleplay

**Monsterhearts** Conditions:
- Each string gives someone else power over you
- Traded and spent for various effects

## Common Moves Across PbtA

### Aid/Interfere
```
When you help or hinder another PC, roll +Bond/Relationship.
• 10+: They take +1 or -2 to their roll (your choice)
• 7-9: They take +1 or -2, but you expose yourself to danger
```

### End of Session
```
At session end, consider:
• Did we learn something new and important about the world?
• Did we overcome a notable monster or enemy?
• Did we loot memorable treasure?
Mark XP for each "yes"
```

### Last Breath (Dungeon World)
```
When you're dying, you glimpse what lies beyond.
Roll +nothing.
• 10+: You stabilize, barely alive
• 7-9: Death offers a bargain - take it or pass on
• 6-: Your fate is sealed, the end is near
```

## Integration Considerations

### StatBlock Mapping
```rust
pub struct PbtaStatBlock {
    // Stats (vary by game, typically -2 to +3)
    pub stats: HashMap<String, i8>,

    // Harm/HP system
    pub harm: HarmSystem,

    // Modifiers
    pub forward: i8,
    pub ongoing: Vec<OngoingModifier>,

    // Hold from various moves
    pub hold: HashMap<String, u8>,

    // XP and advancement
    pub xp: u8,
    pub xp_threshold: u8,

    // Bonds/Relationships
    pub bonds: Vec<Bond>,
}

pub struct Bond {
    pub target_character: String,
    pub description: String,
    pub resolved: bool,
}
```

### Move Registry
```rust
pub struct MoveRegistry {
    pub basic_moves: Vec<PbtaMove>,      // Available to all
    pub playbook_moves: HashMap<String, Vec<PbtaMove>>,  // By playbook
    pub advanced_moves: Vec<PbtaMove>,   // Unlocked at higher levels
}

impl MoveRegistry {
    pub fn get_available_moves(&self, character: &Character) -> Vec<&PbtaMove> {
        let mut moves: Vec<&PbtaMove> = self.basic_moves.iter().collect();

        if let Some(playbook_moves) = self.playbook_moves.get(&character.playbook) {
            moves.extend(playbook_moves.iter());
        }

        moves
    }
}
```

### Game Variant Configuration
```rust
pub struct PbtaVariant {
    pub name: String,                    // "Dungeon World", "Monster of the Week"
    pub stat_names: Vec<String>,         // ["STR", "DEX", "CON", "INT", "WIS", "CHA"]
    pub stat_range: (i8, i8),            // (-1, 3) typically
    pub harm_system: HarmSystemType,
    pub basic_moves: Vec<PbtaMove>,
    pub playbooks: Vec<PlaybookDefinition>,
    pub advancement_type: AdvancementType,
}

pub enum HarmSystemType {
    Clock { segments: u8 },
    HitPoints { formula: String },
    HarmTrack { boxes: u8, unstable: u8 },
    Conditions { list: Vec<String> },
}

pub enum AdvancementType {
    FiveXP,           // Most PbtA
    LevelBased,       // Dungeon World
    MilestoneMarks,   // Some variants
}
```

## Key Differences from D20 Systems

| Aspect | D20 Systems | PbtA |
|--------|-------------|------|
| Dice | d20 + modifiers | 2d6 + stat |
| Target | Variable DC | Fixed (6-/7-9/10+) |
| GM Roll | Yes | Never |
| Actions | Defined list | Triggered by fiction |
| Outcomes | Binary (pass/fail) | Three tiers |
| Damage | HP pools | Harm/Conditions |
| Initiative | Roll for order | Fiction determines |
| Skills | Extensive list | Moves replace skills |
| Advancement | XP and leveling | Varies by game |

## Popular PbtA Games

| Game | Genre | Notable Mechanics |
|------|-------|-------------------|
| **Apocalypse World** | Post-apocalyptic | Original PbtA, Harm clocks |
| **Dungeon World** | Fantasy adventure | D&D-like classes, HP |
| **Monster of the Week** | Monster hunting | Hunter playbooks, Use Magic |
| **Masks** | Teen superheroes | Conditions, Labels |
| **Monsterhearts** | Supernatural teen drama | Strings, darkest self |
| **Urban Shadows** | Urban fantasy | Debts, factions |
| **The Sprawl** | Cyberpunk | Countdown clocks, Corps |
| **Bluebeard's Bride** | Horror | Shared character, rooms |
| **Fellowship** | Epic fantasy | Overlord mechanics |

Each adapts the core 2d6+stat, three-tier outcome engine to its specific genre and themes.
