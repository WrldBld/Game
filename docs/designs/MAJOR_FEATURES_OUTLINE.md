# Major Features Outline

**Created:** 2026-01-10
**Status:** Draft
**Related Issues:** #24, #25, #26, #27, #29, #30

---

## Overview

WrldBldr is a TTRPG engine with visual novel presentation and tactical combat. This document outlines the major features currently blocked, their dependencies, and proposed designs.

### Feature Dependency Graph

```
                    ┌─────────────────────┐
                    │   Character Stats   │
                    │     System (#26)    │
                    └──────────┬──────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
        ▼                      ▼                      ▼
┌───────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Skill System  │    │  StatThreshold  │    │  Combat System  │
│    (#24)      │    │  Trigger (#26)  │    │     (#29)       │
└───────┬───────┘    └─────────────────┘    └────────┬────────┘
        │                                            │
        ▼                                            ▼
┌───────────────┐                          ┌─────────────────┐
│  Challenge    │                          │  CombatResult   │
│  Modifiers    │                          │  Trigger (#27)  │
└───────────────┘                          └─────────────────┘

┌─────────────────┐
│  Relationship   │
│ Level Tracking  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Relationship    │
│ Threshold (#25) │
└─────────────────┘

┌─────────────────────────────────────────────────────┐
│              Inventory System (Exists)              │
│                        +                            │
│              Character Leveling (New)               │
└─────────────────────────┬───────────────────────────┘
                          │
                          ▼
                ┌─────────────────┐
                │  Reward/XP      │
                │  System (#30)   │
                └─────────────────┘
```

---

## 1. Character Stats System

**Unlocks:** #24 (Skill Modifiers), #26 (StatThreshold Trigger), #29 (Combat)

### Design Principles

- **Rule System Agnostic**: Support D20 (D&D), D100 (CoC), Narrative (Fate)
- **Dynamic Schema**: Stats defined per-world, not hardcoded
- **Derived Stats**: Some stats computed from others (e.g., AC from DEX + armor)
- **Modifiers**: Temporary/permanent effects on stats

### Domain Model

```rust
/// A stat definition for a world (DM-defined schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatDefinition {
    pub id: StatDefinitionId,
    pub world_id: WorldId,
    pub name: String,                    // "Strength", "Sanity", "Refresh"
    pub abbreviation: String,            // "STR", "SAN", "REF"
    pub stat_type: StatType,
    pub min_value: Option<i32>,          // Floor (e.g., 0 for SAN)
    pub max_value: Option<i32>,          // Cap (e.g., 99 for SAN)
    pub default_value: i32,              // Starting value for new PCs
    pub display_order: u32,              // UI ordering
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatType {
    /// Core attribute (STR, DEX, CON, INT, WIS, CHA)
    Attribute,
    /// Derived from attributes (AC, Initiative, Spell DC)
    Derived { formula: String },
    /// Resource that depletes (HP, Sanity, Spell Slots)
    Resource { recovers: RecoveryType },
    /// Skill proficiency (Stealth, Perception, Intimidation)
    Skill { linked_attribute: StatDefinitionId },
    /// Save bonus (Fortitude, Reflex, Will)
    Save { linked_attribute: StatDefinitionId },
    /// Custom (Fate aspects, narrative qualities)
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryType {
    None,                               // Manual only
    PerRest { short: i32, long: i32 },  // D&D style
    PerSession { amount: i32 },         // Fate refresh
    PerTime { amount: i32, hours: u32 }, // Sanity recovery
}

/// A character's current stat values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterStats {
    pub character_id: CharacterId,
    pub values: HashMap<StatDefinitionId, StatValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatValue {
    pub base: i32,                       // Base value (from level/creation)
    pub current: i32,                    // Current (for resources)
    pub modifiers: Vec<StatModifier>,    // Active modifiers
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatModifier {
    pub id: StatModifierId,
    pub source: ModifierSource,
    pub amount: i32,
    pub modifier_type: ModifierType,
    pub expires_at: Option<GameTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModifierSource {
    Item(ItemId),
    Spell(String),
    Condition(String),       // "Poisoned", "Blessed"
    Environment(RegionId),   // Location-based effects
    Narrative(NarrativeEventId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModifierType {
    Flat,                    // +2 to STR
    Multiplier,              // ×1.5 damage
    Advantage,               // Roll twice, take higher
    Disadvantage,            // Roll twice, take lower
}
```

### Neo4j Schema

```cypher
// Stat definitions (per world)
(:World)-[:HAS_STAT_DEFINITION]->(:StatDefinition {
  id, name, abbreviation, stat_type, min_value, max_value, default_value
})

// Character stat values
(:Character)-[:HAS_STAT {
  stat_definition_id,
  base_value,
  current_value
}]->(:StatDefinition)

// Active modifiers
(:Character)-[:HAS_MODIFIER {
  id, source_type, source_id, amount, modifier_type, expires_at
}]->(:StatDefinition)
```

### Preset Templates

```rust
pub enum StatPreset {
    DnD5e,      // STR, DEX, CON, INT, WIS, CHA + skills + saves
    Pathfinder2e,
    CallOfCthulhu7e, // STR, CON, DEX, APP, INT, POW, SIZ, EDU + skills
    FateCore,   // Aspects, Stunts, Refresh, Stress tracks
    Custom,
}
```

---

## 2. Skill System (#24)

**Depends on:** Character Stats System
**Unlocks:** Challenge skill modifiers

### Design

Skills are a special case of stats with:
- Linked attribute (for modifier calculation)
- Proficiency tracking (trained/expert/master)
- Use in challenge resolution

```rust
/// Skill definition (extends StatDefinition with skill-specific data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub stat_definition_id: StatDefinitionId,
    pub linked_attribute_id: StatDefinitionId,
    pub proficiency_levels: Vec<ProficiencyLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProficiencyLevel {
    pub name: String,          // "Untrained", "Trained", "Expert", "Master"
    pub bonus: i32,            // 0, +2, +4, +6
}

/// Character's skill proficiency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSkill {
    pub character_id: CharacterId,
    pub skill_definition_id: StatDefinitionId,
    pub proficiency_level: u8,  // Index into proficiency_levels
}
```

### Challenge Integration

```rust
/// Updated challenge to use skills
pub struct Challenge {
    pub id: ChallengeId,
    // ... existing fields ...
    pub required_skill: Option<StatDefinitionId>,
    pub difficulty: ChallengeDifficulty,
}

/// Calculate roll modifier
pub fn calculate_modifier(
    character: &CharacterStats,
    skill: &SkillDefinition,
    proficiency: &CharacterSkill,
) -> i32 {
    let attribute_value = character.get_stat(skill.linked_attribute_id);
    let attribute_mod = (attribute_value - 10) / 2;  // D20 formula
    let proficiency_bonus = skill.proficiency_levels[proficiency.proficiency_level].bonus;

    attribute_mod + proficiency_bonus
}
```

---

## 3. Relationship Level Tracking (#25)

**Unlocks:** RelationshipThreshold trigger

### Current State

Relationships exist with `sentiment` (-1.0 to 1.0) and `relationship_type`. This is sufficient for triggers.

### Enhancement: Discrete Levels

For games that want discrete relationship levels (Stranger → Acquaintance → Friend → Close Friend → Romantic):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipLevelDefinition {
    pub id: RelationshipLevelId,
    pub world_id: WorldId,
    pub name: String,
    pub min_sentiment: f32,     // -1.0 to 1.0 threshold
    pub display_order: u32,
}

// Derived from sentiment
pub fn get_relationship_level(
    sentiment: f32,
    definitions: &[RelationshipLevelDefinition],
) -> &RelationshipLevelDefinition {
    definitions
        .iter()
        .filter(|d| sentiment >= d.min_sentiment)
        .max_by(|a, b| a.min_sentiment.partial_cmp(&b.min_sentiment).unwrap())
        .unwrap()
}
```

### RelationshipThreshold Trigger

```rust
pub enum TriggerCondition {
    // ... existing triggers ...
    RelationshipThreshold {
        npc_id: CharacterId,
        min_sentiment: Option<f32>,      // Numeric threshold
        min_level: Option<String>,       // Named level ("Friend")
    },
}
```

---

## 4. Combat System (#29)

**Depends on:** Character Stats System
**Unlocks:** #27 (CombatResult trigger)

### Design Philosophy

- **Tactical but Simple**: Grid-based positioning, but not hex-level complexity
- **Visual Novel Integration**: Combat UI overlays on visual novel backdrop
- **DM Controlled**: DM can override any roll, add narrative elements
- **Rule System Flexible**: Core mechanics adaptable to D20/D100/narrative

### Combat Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                        COMBAT ENCOUNTER                         │
├─────────────────────────────────────────────────────────────────┤
│  1. INITIATION                                                  │
│     - Trigger (narrative event, player action, NPC approach)   │
│     - DM confirms/customizes participant list                   │
│     - Initiative rolled or set                                  │
│                                                                 │
│  2. TURN STRUCTURE                                              │
│     Per combatant (in initiative order):                       │
│     ┌─────────────────────────────────────────────────────────┐│
│     │  a. Movement Phase                                       ││
│     │     - Select position on tactical grid                   ││
│     │     - Range/adjacency affects action options             ││
│     │                                                          ││
│     │  b. Action Phase                                         ││
│     │     - Attack (melee/ranged, weapon selection)            ││
│     │     - Ability (spells, special attacks)                  ││
│     │     - Item (use consumable, equip)                       ││
│     │     - Interact (environment, objects)                    ││
│     │     - Other (dodge, ready, help)                         ││
│     │                                                          ││
│     │  c. Resolution                                           ││
│     │     - Dice roll (challenge system)                       ││
│     │     - Damage/effect calculation                          ││
│     │     - Target state update                                ││
│     │                                                          ││
│     │  d. DM Intervention (optional)                           ││
│     │     - Modify results                                     ││
│     │     - Add narrative color                                ││
│     │     - Trigger events                                     ││
│     └─────────────────────────────────────────────────────────┘│
│                                                                 │
│  3. END CONDITIONS                                              │
│     - All enemies defeated                                      │
│     - All PCs incapacitated                                     │
│     - Objective achieved (escape, survive X rounds)             │
│     - DM ends combat                                            │
│                                                                 │
│  4. AFTERMATH                                                   │
│     - XP/rewards distributed                                    │
│     - CombatResult trigger evaluated                            │
│     - Narrative transition                                      │
└─────────────────────────────────────────────────────────────────┘
```

### Domain Model

```rust
/// Active combat encounter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatEncounter {
    pub id: CombatEncounterId,
    pub world_id: WorldId,
    pub region_id: RegionId,
    pub state: CombatState,
    pub participants: Vec<CombatParticipant>,
    pub turn_order: Vec<CharacterId>,
    pub current_turn: usize,
    pub round: u32,
    pub grid: Option<TacticalGrid>,
    pub started_at: GameTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombatState {
    Preparing,       // Setting up participants
    Active,          // Combat in progress
    Paused,          // DM paused for narrative
    Ended(CombatOutcome),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatParticipant {
    pub character_id: CharacterId,
    pub side: CombatSide,
    pub initiative: i32,
    pub position: Option<GridPosition>,
    pub conditions: Vec<CombatCondition>,
    pub actions_remaining: ActionBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombatSide {
    Player,
    Enemy,
    Neutral,
    Ally,  // NPC fighting with players
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalGrid {
    pub width: u32,
    pub height: u32,
    pub terrain: Vec<Vec<TerrainType>>,
    pub objects: Vec<GridObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerrainType {
    Open,
    DifficultTerrain,
    Obstacle,
    Cover { amount: CoverType },
    Hazard { damage_type: String, amount: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatAction {
    pub action_type: CombatActionType,
    pub actor: CharacterId,
    pub targets: Vec<CharacterId>,
    pub position: Option<GridPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombatActionType {
    Move { to: GridPosition },
    Attack { weapon_id: Option<ItemId>, attack_type: AttackType },
    UseAbility { ability_id: AbilityId },
    UseItem { item_id: ItemId },
    Interact { target: String },
    Dodge,
    Ready { trigger: String, action: Box<CombatActionType> },
    Help { ally: CharacterId },
    Disengage,
    Hide,
    EndTurn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombatOutcome {
    Victory { xp_earned: u32, loot: Vec<ItemId> },
    Defeat,
    Retreat,
    Surrender,
    Objective { name: String, success: bool },
    Interrupted { reason: String },
}
```

### Combat Actions as Challenges

Combat uses the existing challenge system:

```rust
/// Attack resolution uses ChallengeResolution
pub async fn resolve_attack(
    &self,
    attacker: CharacterId,
    defender: CharacterId,
    attack: AttackType,
) -> AttackResult {
    // Create ad-hoc challenge for the attack
    let attack_challenge = Challenge {
        name: format!("Attack: {}", attack.name),
        difficulty: calculate_defense(defender),
        rule_system: self.world_settings.rule_system,
        required_skill: attack.required_skill,
        // ...
    };

    // Use existing challenge resolution
    let roll_result = self.challenge_service
        .resolve(attacker, attack_challenge)
        .await;

    match roll_result.outcome {
        ChallengeOutcome::Success => {
            let damage = calculate_damage(attacker, attack, roll_result.roll);
            AttackResult::Hit { damage }
        }
        ChallengeOutcome::CriticalSuccess => {
            let damage = calculate_critical_damage(attacker, attack);
            AttackResult::CriticalHit { damage }
        }
        ChallengeOutcome::Failure => AttackResult::Miss,
        ChallengeOutcome::CriticalFailure => AttackResult::Fumble,
        _ => AttackResult::Miss,
    }
}
```

### UI Mockup

```
┌─────────────────────────────────────────────────────────────────┐
│  [Tavern Backdrop - darkened]                                   │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  TACTICAL GRID (overlaid)                                 │ │
│  │  ┌───┬───┬───┬───┬───┬───┬───┬───┐                       │ │
│  │  │   │   │ E │   │   │   │   │   │  E = Enemy           │ │
│  │  ├───┼───┼───┼───┼───┼───┼───┼───┤  P = Player          │ │
│  │  │   │ E │   │   │   │   │   │   │  A = Ally            │ │
│  │  ├───┼───┼───┼───┼───┼───┼───┼───┤  ░ = Cover           │ │
│  │  │   │   │   │░░░│   │   │   │   │  ▓ = Obstacle        │ │
│  │  ├───┼───┼───┼───┼───┼───┼───┼───┤                       │ │
│  │  │   │   │   │   │   │ P │ A │   │                       │ │
│  │  └───┴───┴───┴───┴───┴───┴───┴───┘                       │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ TURN: Marcus (You)  |  Round 2  |  HP: 24/32              ││
│  │                                                             ││
│  │ [Move] [Attack] [Ability] [Item] [Dodge] [End Turn]        ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                 │
│  ┌────────────────────────┐  ┌────────────────────────────────┐│
│  │ Initiative Order       │  │ Selected: Goblin #1            ││
│  │ ▶ Marcus (You)   15   │  │ HP: 8/12                       ││
│  │   Goblin #1      12   │  │ AC: 13                          ││
│  │   Elara (Ally)   10   │  │ Status: None                    ││
│  │   Goblin #2       8   │  │                                  ││
│  └────────────────────────┘  └────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. CombatResult Trigger (#27)

**Depends on:** Combat System (#29)

### Design

```rust
pub enum TriggerCondition {
    // ... existing triggers ...
    CombatResult {
        encounter_id: Option<CombatEncounterId>,  // Specific or any
        outcome: CombatOutcomeRequirement,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CombatOutcomeRequirement {
    Victory,
    Defeat,
    Any,
    SurvivedRounds(u32),
    DefeatedEnemy(CharacterId),
    UsedAbility(AbilityId),
    TookNoDamage,
}
```

---

## 6. Reward/XP System (#30)

**Depends on:** Inventory (exists), Character Leveling (new)

### XP Sources

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XpAward {
    pub amount: u32,
    pub source: XpSource,
    pub awarded_at: GameTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum XpSource {
    Combat { encounter_id: CombatEncounterId },
    Challenge { challenge_id: ChallengeId },
    Quest { event_id: NarrativeEventId },
    Roleplay { description: String },
    Discovery { lore_id: LoreId },
    DmGrant { reason: String },
}
```

### Leveling

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelProgression {
    pub world_id: WorldId,
    pub xp_thresholds: Vec<u32>,  // XP needed for each level
    pub level_benefits: Vec<LevelBenefit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelBenefit {
    pub level: u32,
    pub stat_increases: Vec<(StatDefinitionId, i32)>,
    pub new_abilities: Vec<AbilityId>,
    pub skill_points: u32,
}
```

### Loot Tables

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootTable {
    pub id: LootTableId,
    pub name: String,
    pub entries: Vec<LootEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootEntry {
    pub item_id: ItemId,
    pub quantity_min: u32,
    pub quantity_max: u32,
    pub weight: u32,        // Relative probability
    pub conditions: Vec<LootCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LootCondition {
    MinLevel(u32),
    MaxLevel(u32),
    FlagSet(String),
    TimeOfDay(TimeOfDay),
}
```

---

## 7. Implementation Priority

### Phase 1: Foundation (Stat System)
1. StatDefinition schema and CRUD
2. CharacterStats with modifiers
3. Preset templates (D&D 5e, CoC 7e)
4. UI for stat management

### Phase 2: Skills & Relationships
1. Skill definitions linked to stats
2. Proficiency tracking
3. Challenge integration with skills
4. RelationshipLevel definitions
5. RelationshipThreshold trigger

### Phase 3: Combat Core
1. CombatEncounter entity
2. Turn order management
3. Basic actions (move, attack, end turn)
4. Combat resolution via challenge system
5. Combat UI overlay

### Phase 4: Combat Polish
1. Tactical grid
2. Positioning and range
3. Abilities and items in combat
4. Conditions and status effects
5. CombatResult trigger

### Phase 5: Progression
1. XP tracking
2. Level progression
3. Loot tables
4. Reward integration with combat/events

---

## 8. Estimated Scope

| Feature | Complexity | Estimate |
|---------|------------|----------|
| Stat System | Medium | Core foundation |
| Skill System | Small | Extends stats |
| Relationship Levels | Small | Extends existing |
| Combat System | Large | Major feature |
| CombatResult Trigger | Small | Uses combat |
| Reward/XP System | Medium | Uses combat + stats |

### Recommended Order
1. **Stat System** - Unlocks everything else
2. **Skill System** - Quick win, improves challenges
3. **Relationship Levels** - Quick win, enables trigger
4. **Combat System** - Major undertaking, needs dedicated focus
5. **Reward/XP** - Polish feature after combat works

---

## Appendix: Alternative Combat Approaches

### Option A: Full Tactical (Current Design)
- Grid-based positioning
- Range and movement matter
- Cover and terrain
- **Pros**: Deep tactical play
- **Cons**: Complex UI, slower pacing

### Option B: Abstract Combat
- No grid, just turn order
- Targets selected from list
- Range abstracted to melee/ranged/out-of-range
- **Pros**: Simpler, faster, more narrative
- **Cons**: Less tactical depth

### Option C: Hybrid
- Simple positioning (front/back row)
- Engagement tracking (who is fighting whom)
- Environmental actions as narrative beats
- **Pros**: Best of both
- **Cons**: Design complexity

**Recommendation**: Start with Option B (abstract), add tactical grid later as optional enhancement.
