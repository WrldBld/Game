# UI Mockups for Game System Character Sheets

## Overview

Each game system requires specialized UI components to properly display and interact with its unique mechanics. This document provides wireframe specifications for each supported system.

---

## D&D 5th Edition

### Character Sheet Layout
```
┌─────────────────────────────────────────────────────────────────┐
│  CHARACTER NAME                              Level X [Class]    │
│  Race | Background | Alignment                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────┐  ┌─────────────────────────────┐  │
│  │  ABILITY SCORES         │  │  COMBAT STATS               │  │
│  │  ┌─────┐ ┌─────┐        │  │                             │  │
│  │  │ STR │ │ DEX │        │  │  AC: [__]  Initiative: [__] │  │
│  │  │ 16  │ │ 14  │        │  │  Speed: 30ft                │  │
│  │  │ +3  │ │ +2  │        │  │                             │  │
│  │  └─────┘ └─────┘        │  │  HP: [__] / [__]            │  │
│  │  ┌─────┐ ┌─────┐        │  │  ████████░░░░               │  │
│  │  │ CON │ │ INT │        │  │  Temp HP: [__]              │  │
│  │  │ 15  │ │ 10  │        │  │                             │  │
│  │  │ +2  │ │ +0  │        │  │  Hit Dice: d10 [5/5]        │  │
│  │  └─────┘ └─────┘        │  │  Death Saves: ○○○ / ●●●     │  │
│  │  ┌─────┐ ┌─────┐        │  └─────────────────────────────┘  │
│  │  │ WIS │ │ CHA │        │                                   │
│  │  │ 12  │ │ 8   │        │  ┌─────────────────────────────┐  │
│  │  │ +1  │ │ -1  │        │  │  PROFICIENCIES              │  │
│  │  └─────┘ └─────┘        │  │  Prof Bonus: +3             │  │
│  └─────────────────────────┘  │  Passive Perception: 14     │  │
│                               └─────────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│  SAVING THROWS          │  SKILLS                              │
│  ○ STR +3               │  ● Acrobatics (DEX)     +5          │
│  ● DEX +5               │  ○ Animal Handling (WIS) +1          │
│  ● CON +5               │  ○ Arcana (INT)          +0          │
│  ○ INT +0               │  ● Athletics (STR)       +6          │
│  ○ WIS +1               │  ...                                 │
│  ○ CHA -1               │                                      │
├─────────────────────────────────────────────────────────────────┤
│  ATTACKS & SPELLCASTING                                         │
│  ┌──────────────┬────────┬────────────┐                        │
│  │ Longsword    │ +6     │ 1d8+3 slsh │                        │
│  │ Shortbow     │ +5     │ 1d6+2 prc  │                        │
│  └──────────────┴────────┴────────────┘                        │
├─────────────────────────────────────────────────────────────────┤
│  SPELL SLOTS          SPELLS KNOWN/PREPARED                     │
│  1st: ●●●●○           [Cure Wounds] [Bless] [Shield of Faith]  │
│  2nd: ●●●○○           [Aid] [Lesser Restoration]               │
│  3rd: ●●○○○           [Revivify] [Spirit Guardians]            │
└─────────────────────────────────────────────────────────────────┘
```

### Key Components
- **Ability Score Boxes**: Score + modifier, clickable for rolls
- **HP Bar**: Visual with current/max, temp HP field
- **Saving Throws**: Checkboxes for proficiency
- **Skills**: Proficiency indicators, calculated modifiers
- **Spell Slots**: Fillable circles, track used/available

---

## Pathfinder 2e

### Character Sheet Layout
```
┌─────────────────────────────────────────────────────────────────┐
│  CHARACTER NAME                              Level X [Class]    │
│  Ancestry | Heritage | Background                               │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────┐  ┌─────────────────────────────┐  │
│  │  ABILITY SCORES         │  │  DEFENSES                   │  │
│  │  STR: 18 (+4)           │  │                             │  │
│  │  DEX: 14 (+2)           │  │  AC: 22 (10+2+4+6)          │  │
│  │  CON: 16 (+3)           │  │     [T] [E] [M] [L]         │  │
│  │  INT: 10 (+0)           │  │                             │  │
│  │  WIS: 12 (+1)           │  │  Fortitude: +12 [E]         │  │
│  │  CHA: 8  (-1)           │  │  Reflex:    +8  [T]         │  │
│  └─────────────────────────┘  │  Will:      +10 [E]         │  │
│                               │                             │  │
│  ┌─────────────────────────┐  │  Speed: 25 ft               │  │
│  │  PERCEPTION             │  │  Perception: +10 [E]        │  │
│  │  +10 [Expert]           │  └─────────────────────────────┘  │
│  │  DC: 20                 │                                   │
│  └─────────────────────────┘  ┌─────────────────────────────┐  │
│                               │  HIT POINTS                 │  │
│  ┌─────────────────────────┐  │  [78] / [78]                │  │
│  │  CLASS DC               │  │  ████████████████████       │  │
│  │  21 [Expert]            │  │  Dying: ○○○○                │  │
│  └─────────────────────────┘  │  Wounded: 0                 │  │
│                               │  Doomed: 0                  │  │
│                               └─────────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│  PROFICIENCY LEGEND: [U]ntrained [T]rained [E]xpert [M]aster [L]egendary │
├─────────────────────────────────────────────────────────────────┤
│  SKILLS (Proficiency + Level + Ability Mod)                     │
│  ┌────────────────────┬─────┬────────┬─────────────────────┐   │
│  │ Acrobatics (DEX)   │ [T] │ +10    │ 2 + 5 + 2 + 1 item  │   │
│  │ Arcana (INT)       │ [U] │ +0     │ 0 + 0 + 0           │   │
│  │ Athletics (STR)    │ [E] │ +15    │ 4 + 5 + 4 + 2 item  │   │
│  │ Crafting (INT)     │ [T] │ +7     │ 2 + 5 + 0           │   │
│  │ Deception (CHA)    │ [U] │ -1     │ 0 + 0 - 1           │   │
│  └────────────────────┴─────┴────────┴─────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  ACTIONS (3 per turn)  ◆ ◆ ◆                                   │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  STRIKES                                                  │  │
│  │  ┌────────────┬───────┬───────┬─────────────┬─────────┐  │  │
│  │  │ Weapon     │ Prof  │ Atk   │ Damage      │ Traits  │  │  │
│  │  ├────────────┼───────┼───────┼─────────────┼─────────┤  │  │
│  │  │ Longsword  │ [M]   │ +17   │ 2d8+4 S     │ Versatile│  │  │
│  │  │ -1st: +17  -2nd: +12  -3rd: +7 (MAP)                 │  │  │
│  │  └────────────┴───────┴───────┴─────────────┴─────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│  HERO POINTS: ●●○                                               │
├─────────────────────────────────────────────────────────────────┤
│  CONDITIONS                                                      │
│  ┌──────────┬───┐  ┌──────────┬───┐  ┌──────────┬───┐         │
│  │Frightened│ 2 │  │Clumsy    │ 0 │  │Drained   │ 0 │         │
│  └──────────┴───┘  └──────────┴───┘  └──────────┴───┘         │
└─────────────────────────────────────────────────────────────────┘
```

### Key Components
- **Proficiency Badges**: [U] [T] [E] [M] [L] visual indicators
- **Four Degrees Display**: Roll results show Critical/Success/Failure/Critical Failure
- **Three Actions**: Visual action economy tracker
- **MAP Display**: Multiple Attack Penalty calculator
- **Conditions with Values**: Numeric severity for each condition
- **Hero Points**: Visual tracker (max 3)

---

## Call of Cthulhu 7e

### Character Sheet Layout
```
┌─────────────────────────────────────────────────────────────────┐
│  INVESTIGATOR NAME                              Occupation       │
│  Age: __  Birthplace: __________  Residence: __________         │
├─────────────────────────────────────────────────────────────────┤
│  CHARACTERISTICS                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │   STR     CON     SIZ     DEX     APP     INT     POW     EDU│
│  │   [55]    [60]    [65]    [50]    [45]    [70]    [55]   [80]│
│  │    27      30      32      25      22      35      27     40 │
│  │    11      12      13      10       9      14      11     16 │
│  │   (Half)  (Half)  (Half)  (Half)  (Half)  (Half)  (Half) (1/5)│
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  DERIVED ATTRIBUTES                                              │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  HP: [12] / [12]        Sanity: [55] / [99]               │ │
│  │  ████████████████       SAN: ████████████░░░░░░░░░░░░░░   │ │
│  │                         Max SAN: 99 - Mythos (0) = 99     │ │
│  │  Magic Points: [11]     Luck: [55]                        │ │
│  │  ███████████░░░         ███████████░░░░░░░░░              │ │
│  │                                                           │ │
│  │  Damage Bonus: None     Build: 0     Move: 8              │ │
│  └───────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│  SKILLS                    Regular | Hard  | Extreme           │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  ○ Accounting           05%      02%     01%             │  │
│  │  ○ Anthropology         01%      00%     00%             │  │
│  │  ● Art/Craft (Photo)    45%      22%     09%             │  │
│  │  ○ Climb                20%      10%     04%             │  │
│  │  ● Credit Rating        35%      17%     07%             │  │
│  │  ○ Cthulhu Mythos       00%      --      --    [LOCKED]  │  │
│  │  ● Dodge                30%      15%     06%             │  │
│  │  ● Fast Talk            55%      27%     11%   ☑ CHECK   │  │
│  │  ● Library Use          65%      32%     13%             │  │
│  │  ● Listen               45%      22%     09%             │  │
│  │  ...                                                      │  │
│  └──────────────────────────────────────────────────────────┘  │
│  ● = Occupation skill   ☑ = Marked for improvement             │
├─────────────────────────────────────────────────────────────────┤
│  COMBAT                                                         │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Weapon          Skill   Damage     Range    Attacks      │   │
│  │  .38 Revolver    35%     1d10       15 yds   1            │   │
│  │  Fist/Punch      50%     1d3+DB     Touch    1            │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  SANITY CHECK RESULT                                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Roll: [__]  vs Sanity: [55]                             │   │
│  │  ○ Success (lose min)  ○ Failure (lose max)              │   │
│  │  Bout of Madness: ○ Yes ○ No  (lost 5+ in one incident)  │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  BACKSTORY                                                       │
│  Ideology: _________________________________________________    │
│  Significant People: ________________________________________    │
│  Meaningful Locations: ______________________________________    │
│  Treasured Possessions: _____________________________________    │
│  Phobias & Manias: __________________________________________    │
└─────────────────────────────────────────────────────────────────┘
```

### Key Components
- **Percentile Display**: Each stat shows Full / Half / Fifth values
- **Sanity Meter**: Visual with max SAN affected by Mythos
- **Skill Improvement Checks**: Checkboxes for session-end rolls
- **Luck Track**: Spendable and depletable
- **Sanity Check Dialog**: Quick reference for SAN rolls
- **Cthulhu Mythos**: Special locked skill that reduces max SAN

---

## FATE Core

### Character Sheet Layout
```
┌─────────────────────────────────────────────────────────────────┐
│  CHARACTER NAME                                                  │
├─────────────────────────────────────────────────────────────────┤
│  ASPECTS                                                         │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  High Concept: "Wizard Private Investigator"             │   │
│  │  ┌──────┐                                                │   │
│  │  │INVOKE│  +2 or Reroll when investigating magic        │   │
│  │  └──────┘                                                │   │
│  ├─────────────────────────────────────────────────────────┤   │
│  │  Trouble: "Owes Favors to the Faerie Court"              │   │
│  │  ┌───────┐                                               │   │
│  │  │COMPEL │  The Fae call in a favor at bad time         │   │
│  │  └───────┘                                               │   │
│  ├─────────────────────────────────────────────────────────┤   │
│  │  Aspect: "My Partner Has My Back"                        │   │
│  │  Aspect: "Books Contain All Answers"                     │   │
│  │  Aspect: "Former Police Detective"                       │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  SKILLS (Pyramid)                                                │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  +4 Great      [Investigate]                             │   │
│  │  +3 Good       [Lore] [Will]                             │   │
│  │  +2 Fair       [Contacts] [Notice] [Rapport]             │   │
│  │  +1 Average    [Athletics] [Empathy] [Shoot] [Stealth]   │   │
│  │  +0 Mediocre   [All Others]                              │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  FATE POINTS                              REFRESH: 3             │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Current: ●●●○○                                          │   │
│  │  [Spend]  [Gain]                                         │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  STUNTS                                                          │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  1. "The Truth Is Out There"                             │   │
│  │     +2 to Investigate when searching for occult clues    │   │
│  │                                                          │   │
│  │  2. "Scholarly Network"                                  │   │
│  │     Use Lore instead of Contacts for academic circles    │   │
│  │                                                          │   │
│  │  3. "Wards and Protections"                              │   │
│  │     Once per session, ignore supernatural attack         │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  STRESS                                                          │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Physical: □ □ □ □    (4 boxes from Physique +2)        │   │
│  │  Mental:   □ □ □      (3 boxes from Will +3)            │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  CONSEQUENCES                                                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Mild (-2):     [____________________________]          │   │
│  │  Moderate (-4): [____________________________]          │   │
│  │  Severe (-6):   [____________________________]          │   │
│  │                                                          │   │
│  │  Each consequence is an aspect that can be invoked!      │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  SITUATION ASPECTS (Scene)                                       │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  "Dark Alley" [●●] free invokes                          │   │
│  │  "On Fire" [●] free invoke                               │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Key Components
- **Aspect Cards**: Draggable cards with Invoke/Compel buttons
- **Skill Pyramid**: Visual pyramid showing skill distribution
- **Fate Point Tracker**: Spend/Gain buttons with refresh display
- **Stress Checkboxes**: Click to mark/unmark
- **Consequence Slots**: Text fields that become aspects
- **Situation Aspects Panel**: Scene-specific aspects with free invokes

---

## Blades in the Dark

### Character Sheet Layout
```
┌─────────────────────────────────────────────────────────────────┐
│  CHARACTER NAME                    Playbook: LURK               │
│  Alias: ____________              Crew: THE SHADOWS              │
├─────────────────────────────────────────────────────────────────┤
│  ACTIONS                                                         │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  INSIGHT [3]           PROWESS [4]        RESOLVE [2]    │   │
│  │  ─────────────         ─────────────      ─────────────  │   │
│  │  Hunt    ●○○○          Finesse  ●●○○      Attune  ●○○○   │   │
│  │  Study   ●○○○          Prowl    ●●●○      Command ○○○○   │   │
│  │  Survey  ●○○○          Skirmish ●○○○      Consort ●○○○   │   │
│  │  Tinker  ○○○○          Wreck    ○○○○      Sway    ○○○○   │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  STRESS          ○○○○○○○○○                   TRAUMA  ○○○○       │
│                  [_________|]                 Conditions:       │
│                  Current: 3/9                 ○ Cold  ○ Haunted │
│                                               ○ Obsessed ○ etc │
├─────────────────────────────────────────────────────────────────┤
│  HARM                                                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  3 [SEVERE]     [________________________]               │   │
│  │  2 [MODERATE]   [____________] [____________]            │   │
│  │  1 [LESSER]     [____________] [____________]            │   │
│  │                                                          │   │
│  │  Healing Clock: ○○○○○○                                   │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  ARMOR  □ □  Standard        LOAD: [Normal - 5]                 │
│         □    Heavy           Items: ●●●○○                       │
│         □    Special (vs supernatural)                          │
├─────────────────────────────────────────────────────────────────┤
│  SPECIAL ABILITIES                                               │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  ● INFILTRATOR                                           │   │
│  │    You are not affected by quality/tier when sneaking    │   │
│  │                                                          │   │
│  │  ● SHADOW                                                │   │
│  │    You blend in anywhere you can see a shadow            │   │
│  │                                                          │   │
│  │  ○ [Available ability slot]                              │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  ACTION ROLL                                                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Position: [Controlled] [Risky] [Desperate]              │   │
│  │  Effect:   [Zero] [Limited] [Standard] [Great]           │   │
│  │                                                          │   │
│  │  Dice Pool: 3d6                                          │   │
│  │  [+1d Push] [+1d Devil's Bargain] [+1d Assist]           │   │
│  │                                                          │   │
│  │  Results: [4] [6] [2]  →  SUCCESS                        │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  CLOCKS                                                          │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │   "Pick Lock"        "Guards Coming"      "Project X"    │   │
│  │      ◐                   ◑                    ◔          │   │
│  │     3/6                 2/4                  1/8         │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  FLASHBACK                                                       │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  [Trigger Flashback]                                     │   │
│  │  Stress Cost: ○ Simple (0)  ○ Complex (1)  ○ Elaborate (2)│   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Key Components
- **Action Dots**: Visual representation of 0-4 rating
- **Attribute Totals**: Auto-calculated from action sums
- **Stress Bar**: Visual track with trauma threshold
- **Harm Boxes**: Text fields at each severity level
- **Position/Effect Selector**: Radio buttons for roll context
- **Load Tracker**: Dynamic item declaration during play
- **Clock Widgets**: Visual pie-chart progress trackers
- **Flashback Button**: Quick access with stress cost selector

---

## Powered by the Apocalypse

### Character Sheet Layout
```
┌─────────────────────────────────────────────────────────────────┐
│  CHARACTER NAME                    Playbook: THE BATTLEBABE     │
│  Look: ambiguous, scrounged wear, scarred face                  │
├─────────────────────────────────────────────────────────────────┤
│  STATS                                                           │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │    COOL      HARD      HOT       SHARP     WEIRD        │   │
│  │    +3        +1        +1        +1        -1           │   │
│  │   ┌───┐     ┌───┐     ┌───┐     ┌───┐     ┌───┐        │   │
│  │   │ ● │     │ ● │     │ ● │     │ ● │     │ ○ │        │   │
│  │   │ ● │     │ ● │     │ ● │     │ ● │     │ ○ │        │   │
│  │   │ ● │     │ ○ │     │ ○ │     │ ○ │     │ ○ │        │   │
│  │   │ ○ │     │ ○ │     │ ○ │     │ ○ │     │ ● │        │   │
│  │   └───┘     └───┘     └───┘     └───┘     └───┘        │   │
│  │ Highlighted: COOL, WEIRD                                 │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  HARM                                                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │   □ □ □ □ □ □   Countdown      ○ Stabilized             │   │
│  │   3 6 9 9 1 1   to midnight    ○ Shattered (-1 hard)    │   │
│  │                 2                                        │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  MOVES                                                           │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  BASIC MOVES                                             │   │
│  │  ┌───────────────────────────────────────────────────┐  │   │
│  │  │ ACT UNDER FIRE                          [+Cool]   │  │   │
│  │  │ When you do something under fire, roll+cool.      │  │   │
│  │  │ • 10+: You do it                                  │  │   │
│  │  │ • 7-9: You stumble, hesitate, or flinch           │  │   │
│  │  │ [ROLL]                                            │  │   │
│  │  └───────────────────────────────────────────────────┘  │   │
│  │  ┌───────────────────────────────────────────────────┐  │   │
│  │  │ GO AGGRO                                [+Hard]   │  │   │
│  │  │ When you go aggro on someone, roll+hard...        │  │   │
│  │  │ [ROLL]                                            │  │   │
│  │  └───────────────────────────────────────────────────┘  │   │
│  │                                                          │   │
│  │  PLAYBOOK MOVES                                          │   │
│  │  ┌───────────────────────────────────────────────────┐  │   │
│  │  │ ● DANGEROUS & SEXY                      [+Hot]    │  │   │
│  │  │   When you enter a charged situation...           │  │   │
│  │  │ [ROLL]                                            │  │   │
│  │  └───────────────────────────────────────────────────┘  │   │
│  │  ┌───────────────────────────────────────────────────┐  │   │
│  │  │ ● ICE COLD                              [+Cool]   │  │   │
│  │  │   When you act under fire, on 10+ you can...      │  │   │
│  │  └───────────────────────────────────────────────────┘  │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  HOLD                                                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Read a Person: ●●○  (2 hold remaining)                  │   │
│  │  Seduce/Manipulate: ●○○ (1 hold remaining)               │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  MODIFIERS                                                       │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Forward: +1 (from Read a Person)                        │   │
│  │  Ongoing: +1 (while in my territory)                     │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  EXPERIENCE                     ○○○○○ → [ADVANCE]              │
│                                 (3/5)                           │
├─────────────────────────────────────────────────────────────────┤
│  HX (History with other PCs)                                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Rico the Gunlugger:   +2                                │   │
│  │  Frost the Skinner:    +1                                │   │
│  │  Doc the Angel:        -1                                │   │
│  └─────────────────────────────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│  GEAR                                                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  • Custom firearms (2-harm close reload loud)            │   │
│  │  • Leather wear (1-armor)                                │   │
│  │  • Oddments worth 2-barter                               │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Key Components
- **Stat Display**: Visual (-2 to +3) with highlighting
- **Move Cards**: Expandable cards with trigger text and roll button
- **Three-Tier Results**: Clear 6-/7-9/10+ outcome display
- **Hold Tracker**: Per-move hold counters
- **Forward/Ongoing Panel**: Active modifiers
- **XP Track**: 5-segment advancement tracker
- **History (Hx)**: Relationship values with other PCs
- **Highlighted Stats**: Mark for session XP

---

## Cross-System Components

### Universal Roll Dialog
```
┌─────────────────────────────────────────────────────────────────┐
│  ROLL: [Action/Skill Name]                                      │
├─────────────────────────────────────────────────────────────────┤
│  System: [Auto-detected from character]                         │
│                                                                 │
│  Base:     +5 (Skill modifier)                                  │
│  Bonuses:  +2 (Aspect invoke)                                   │
│  Penalties: -1 (Wounded)                                        │
│  ─────────────────                                              │
│  Total:    +6                                                   │
│                                                                 │
│  [Roll Dice]                                                    │
│                                                                 │
│  Result: 14                                                     │
│  Outcome: SUCCESS                                               │
│                                                                 │
│  [Apply Result] [Cancel]                                        │
└─────────────────────────────────────────────────────────────────┘
```

### System Indicator Badge
Each character sheet should have a clear system indicator:
```
┌───────────────────┐
│ D&D 5e           │  ← Blue badge
│ PF2e             │  ← Red badge
│ CoC 7e           │  ← Green badge
│ FATE Core        │  ← Orange badge
│ Blades           │  ← Purple badge
│ PbtA             │  ← Yellow badge
└───────────────────┘
```

---

## Implementation Notes

### Component Library Requirements
1. **Checkbox/Pip Components**: For proficiency, stress, XP
2. **Progress Bars**: For HP, SAN, stress tracks
3. **Clock Widgets**: For Blades progress clocks
4. **Card Components**: For aspects, moves, features
5. **Dice Display**: Animated roll results
6. **Modal Dialogs**: For rolls, invocations, flashbacks
7. **Drag-and-Drop**: For rearranging aspects, inventory
8. **Collapsible Sections**: For large character sheets

### Accessibility
- High contrast modes for all systems
- Screen reader support for all components
- Keyboard navigation for all interactive elements
- Color-blind friendly indicators (icons + colors)

### Responsive Design
- Desktop: Full sheet layout
- Tablet: Collapsible sections
- Mobile: Tab-based navigation between sections
