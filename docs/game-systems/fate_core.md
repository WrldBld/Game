# FATE Core System Reference

## Overview

FATE Core is a narrative-focused RPG using Fudge dice (4dF) with a ladder-based result system. It emphasizes player agency, collaborative storytelling, and aspects as the central mechanic. Characters are defined more by descriptive phrases than numerical stats.

## Core Mechanics

### Dice System
- **Primary Roll**: 4dF (four Fudge dice)
- **Each Die**: Three faces: +1, 0, -1
- **Result Range**: -4 to +4 (bell curve centered on 0)
- **Roll Formula**: `4dF + Skill Rating vs Difficulty`

### Probability Distribution
| Result | Probability |
|--------|-------------|
| 0 | 23.46% |
| ±1 | 19.75% each |
| ±2 | 12.35% each |
| ±3 | 4.94% each |
| ±4 | 1.23% each |

### The Ladder
All values in FATE use a descriptive ladder:

| Rating | Descriptor |
|--------|------------|
| +8 | Legendary |
| +7 | Epic |
| +6 | Fantastic |
| +5 | Superb |
| +4 | Great |
| +3 | Good |
| +2 | Fair |
| +1 | Average |
| 0 | Mediocre |
| -1 | Poor |
| -2 | Terrible |

### Shifts and Outcomes
**Shifts** = Your Roll - Opposition (Difficulty or opposing roll)

| Shifts | Outcome | Effect |
|--------|---------|--------|
| < 0 | Failure | Don't achieve goal, may face consequence |
| 0 | Tie | Success at minor cost, or boost |
| 1-2 | Success | Achieve your goal |
| 3+ | Success with Style | Achieve goal + bonus (boost or extra effect) |

**Implementation**:
```rust
fn resolve_fate_roll(
    skill_rating: i32,
    dice_result: i32,  // Sum of 4dF
    difficulty: i32,
) -> FateOutcome {
    let total = skill_rating + dice_result;
    let shifts = total - difficulty;

    if shifts >= 3 {
        FateOutcome::SuccessWithStyle { shifts }
    } else if shifts >= 1 {
        FateOutcome::Success { shifts }
    } else if shifts == 0 {
        FateOutcome::Tie
    } else {
        FateOutcome::Failure { shifts }
    }
}
```

## Character Aspects

### Five Aspects
Every character has five aspects - descriptive phrases that define who they are:

| Aspect Type | Purpose | Example |
|-------------|---------|---------|
| High Concept | Core identity | "Wizard Private Investigator" |
| Trouble | Major complication | "Owes Favors to the Faerie Court" |
| Aspect 1-3 | Relationships, beliefs, possessions | "My Partner Has My Back" |

### Invoking Aspects
- **Cost**: 1 Fate Point
- **Benefit**: +2 to roll OR reroll all 4dF
- **When**: After rolling, when aspect is relevant

```rust
pub enum InvokeType {
    AddTwo,   // +2 to roll
    Reroll,   // Reroll all 4dF
}

fn invoke_aspect(fate_points: &mut u8, invoke_type: InvokeType) -> Result<InvokeType> {
    if *fate_points < 1 {
        return Err(Error::InsufficientFatePoints);
    }
    *fate_points -= 1;
    Ok(invoke_type)
}
```

### Compelling Aspects
- **Trigger**: GM or player offers a complication based on aspect
- **If Accepted**: Gain 1 Fate Point, complication happens
- **If Refused**: Pay 1 Fate Point to avoid

### Situation Aspects
- Created during play via Create Advantage action
- Temporary (scene-specific)
- Come with free invocations

## Skills

### Standard Skill List (18 Skills)
| Skill | Description |
|-------|-------------|
| Athletics | Running, jumping, climbing, dodging |
| Burglary | Breaking and entering, lockpicking |
| Contacts | Knowing people, gathering info via network |
| Crafts | Building and fixing things |
| Deceive | Lying, misdirection, false impressions |
| Drive | Operating vehicles |
| Empathy | Reading emotions, detecting lies |
| Fight | Close-quarters combat |
| Investigate | Finding clues, piecing together info |
| Lore | Academic knowledge, esoteric facts |
| Notice | Passive awareness, spotting details |
| Physique | Strength, endurance, toughness |
| Provoke | Intimidation, scaring, angering |
| Rapport | Building relationships, making friends |
| Resources | Wealth and material resources |
| Shoot | Ranged weapons and attacks |
| Stealth | Staying hidden, moving unseen |
| Will | Mental fortitude, resisting mental attacks |

### Skill Pyramid
Characters distribute skills in a pyramid structure:
- 1 skill at +4 (Great)
- 2 skills at +3 (Good)
- 3 skills at +2 (Fair)
- 4 skills at +1 (Average)
- All others at +0 (Mediocre)

```rust
pub struct SkillPyramid {
    pub skills: HashMap<SkillId, i32>,
    pub max_rating: i32,  // typically 4
}

impl SkillPyramid {
    pub fn validate(&self) -> Result<(), PyramidError> {
        let mut counts: HashMap<i32, u32> = HashMap::new();
        for rating in self.skills.values() {
            *counts.entry(*rating).or_insert(0) += 1;
        }

        // Pyramid: Each level N needs at least N skills at level (N-1)
        for level in 1..=self.max_rating {
            let count_at_level = counts.get(&level).copied().unwrap_or(0);
            let count_below = counts.get(&(level - 1)).copied().unwrap_or(0);

            if count_below < count_at_level {
                return Err(PyramidError::InvalidShape { level });
            }
        }
        Ok(())
    }
}
```

### FATE Accelerated (Variant)
Uses 6 **Approaches** instead of 18 skills:
- **Careful**: Cautious, methodical actions
- **Clever**: Thinking fast, solving puzzles
- **Flashy**: Dramatic, attention-grabbing
- **Forceful**: Brute strength, willpower
- **Quick**: Speed, reaction time
- **Sneaky**: Misdirection, stealth

Rated +0 to +3, distributed as: one +3, two +2, two +1, one +0

## Four Actions

### Overcome
**Purpose**: Get past obstacles, solve problems
**Outcomes**:
- Fail: Don't overcome, or succeed at serious cost
- Tie: Succeed at minor cost
- Success: Overcome the obstacle
- Style: Overcome + create a boost

### Create Advantage
**Purpose**: Create or discover aspects, get free invocations
**Outcomes**:
- Fail: Don't create aspect, or opponent gets free invocation
- Tie: Create aspect with 1 free invocation (or boost existing)
- Success: Create aspect with 1 free invocation
- Style: Create aspect with 2 free invocations

### Attack
**Purpose**: Harm opponent (vs. Defend)
**Outcomes**:
- Fail/Tie: No effect
- Success: Deal shifts as stress
- Style: Deal shifts + create a boost

### Defend
**Purpose**: Counter Attack or Create Advantage
**Roll**: Opposes attacker's roll

```rust
pub enum FateAction {
    Overcome {
        skill: SkillId,
        difficulty: i32,
    },
    CreateAdvantage {
        skill: SkillId,
        target: AdvantageTarget,
    },
    Attack {
        skill: SkillId,
        target: CharacterId,
    },
    Defend {
        skill: SkillId,
    },
}

pub enum AdvantageTarget {
    NewAspect { name: String },
    ExistingAspect { aspect_id: String },
    DiscoverAspect,
}
```

## Stunts

### What Stunts Do
Three types of effects:

1. **Add Bonus**: +2 to skill in specific situation
   - "Because I'm a Combat Veteran, I get +2 to Fight when outnumbered"

2. **Add Action**: Do something normally impossible
   - "Because I Read People, I can use Empathy to defend against Deceive"

3. **Rule Exception**: Break a game rule in specific way
   - "Once per session, I can clear a mild consequence immediately"

### Stunts and Refresh
- **Starting Stunts**: 3
- **Maximum Stunts**: 5
- **Refresh Trade-off**: Extra stunts reduce Refresh

```
Refresh = 3 - (Stunts - 3)  // if Stunts > 3

Examples:
- 3 stunts = 3 Refresh
- 4 stunts = 2 Refresh
- 5 stunts = 1 Refresh (minimum)
```

## Fate Points

### Refresh
- Starting Fate Points each session
- Default: 3
- Reduced by extra stunts
- Minimum: 1

### Uses
1. **Invoke Aspect**: +2 or reroll (costs 1 FP)
2. **Accept Compel**: Gain 1 FP
3. **Refuse Compel**: Pay 1 FP
4. **Declare Story Detail**: Add minor fact to scene (costs 1 FP, GM approval)
5. **Power a Stunt**: Some stunts cost FP

```rust
pub struct FatePointTracker {
    pub current: u8,
    pub refresh: u8,
}

impl FatePointTracker {
    pub fn start_session(&mut self) {
        if self.current < self.refresh {
            self.current = self.refresh;
        }
        // Note: If current > refresh, you keep the excess
    }

    pub fn spend(&mut self) -> Result<()> {
        if self.current < 1 {
            return Err(Error::InsufficientFatePoints);
        }
        self.current -= 1;
        Ok(())
    }

    pub fn gain(&mut self) {
        self.current += 1;
    }
}
```

## Stress and Consequences

### Stress Tracks
- **Physical Stress**: 2 boxes (+ more from Physique)
- **Mental Stress**: 2 boxes (+ more from Will)

**Extra Boxes from Skills**:
| Skill Rating | Stress Boxes |
|--------------|--------------|
| Mediocre (+0) | 2 boxes |
| Average/Fair (+1/+2) | 3 boxes |
| Good/Great (+3/+4) | 4 boxes |

### Taking Hits
When you take stress (shifts of damage):
1. Check a stress box equal to or greater than shifts
2. OR take a consequence
3. OR be Taken Out

**Stress clears at end of scene**

### Consequences
| Severity | Shifts Absorbed | Recovery Time |
|----------|-----------------|---------------|
| Mild (-2) | 2 | End of scene |
| Moderate (-4) | 4 | End of session |
| Severe (-6) | 6 | End of scenario |

- Each character has: 1 Mild, 1 Moderate, 1 Severe slot
- Consequence is an aspect (can be invoked/compelled)
- Opponent names the consequence
- First invocation against you is free

```rust
pub struct Consequence {
    pub severity: ConsequenceSeverity,
    pub aspect_text: String,
    pub free_invoke_used: bool,
}

pub enum ConsequenceSeverity {
    Mild,     // -2, heals end of scene
    Moderate, // -4, heals end of session
    Severe,   // -6, heals end of scenario
}

impl Consequence {
    pub fn shifts_absorbed(&self) -> i32 {
        match self.severity {
            ConsequenceSeverity::Mild => 2,
            ConsequenceSeverity::Moderate => 4,
            ConsequenceSeverity::Severe => 6,
        }
    }
}
```

### Being Taken Out
- Occurs when you can't absorb stress
- Opponent narrates your defeat
- Alternative: **Concede** before being Taken Out (you narrate your exit, gain FP)

## Character Creation

### Summary Steps
1. **High Concept**: Core identity aspect
2. **Trouble**: Major complication aspect
3. **Phase One**: First adventure (creates 1 aspect)
4. **Phase Two**: Crossing paths with another PC (creates 1 aspect)
5. **Phase Three**: Another crossing (creates 1 aspect)
6. **Skills**: Distribute in pyramid
7. **Stunts**: Choose 1-3 stunts
8. **Refresh**: Calculate (3 minus extra stunts)
9. **Stress & Consequences**: Mark boxes based on Physique/Will

### Advancement (Milestones)

**Minor Milestone** (every session):
- Switch two skill ratings
- Rename one aspect
- Purchase stunt (if refresh available)

**Significant Milestone** (end of scenario):
- All minor milestone options
- +1 skill (obeying pyramid)
- OR take new stunt and reduce refresh

**Major Milestone** (campaign conclusion):
- All significant milestone options
- +1 refresh
- Rename high concept (if appropriate)
- Increase skill cap (if relevant)

## Integration Considerations

### StatBlock Mapping
```rust
pub struct FateStatBlock {
    // Skills (ladder values)
    skills: HashMap<String, i32>,

    // Approaches (for FAE variant)
    approaches: Option<HashMap<String, i32>>,

    // Stress tracks
    physical_stress: Vec<bool>,  // Checkboxes
    mental_stress: Vec<bool>,

    // Consequences
    mild_consequence: Option<Consequence>,
    moderate_consequence: Option<Consequence>,
    severe_consequence: Option<Consequence>,

    // Fate Points
    current_fate_points: u8,
    refresh: u8,
}
```

### Aspect System
```rust
pub struct FateAspect {
    pub id: String,
    pub aspect_type: AspectType,
    pub text: String,
    pub free_invokes: u8,
}

pub enum AspectType {
    HighConcept,
    Trouble,
    Character,
    Situation,
    Consequence(ConsequenceSeverity),
    Boost,  // One-time, disappears after use
}
```

### Key Differences from D20 Systems
1. **Narrative First**: Fiction determines mechanics, not vice versa
2. **No HP/Damage**: Stress + Consequences instead
3. **Aspects, Not Stats**: Descriptive phrases define capabilities
4. **Player Narrative Control**: Fate Points buy story influence
5. **Collaborative**: Players contribute to world-building
6. **Bounded Values**: Ladder -2 to +8, not scaling numbers
7. **Pyramid Structure**: Skill distribution is constrained
8. **Double-Edged Aspects**: Can help or hinder equally
