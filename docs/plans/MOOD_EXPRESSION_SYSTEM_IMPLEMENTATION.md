# Mood & Expression System - Implementation Plan

**Created**: 2025-12-27
**Status**: PLANNING COMPLETE - Ready for Implementation
**Priority**: P3.1 (Low - Polish)
**Estimated Effort**: 30-35 hours (4-5 days)
**Tracking**: This document tracks implementation progress

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Three-Tier Emotional Model](#three-tier-emotional-model)
3. [Current State Analysis](#current-state-analysis)
4. [Proposed System Design](#proposed-system-design)
5. [UI Mockups](#ui-mockups)
6. [Data Model Changes](#data-model-changes)
7. [Protocol Changes](#protocol-changes)
8. [Implementation Phases](#implementation-phases)
9. [File Change Summary](#file-change-summary)
10. [System Documentation Updates](#system-documentation-updates)
11. [Testing Strategy](#testing-strategy)
12. [Rollback Plan](#rollback-plan)
13. [Progress Tracking](#progress-tracking)

---

## Executive Summary

### Problem Statement

Currently, the codebase conflates two distinct concepts under "mood":
1. **NPC-to-PC relationship disposition** - How an NPC fundamentally views a PC (stored as `MoodLevel` on `DISPOSITION_TOWARD` edge)
2. **Transient emotional expression** - What emotion an NPC shows during dialogue (not implemented)

Additionally, there's no concept of NPC **mood** - their current emotional state independent of any PC relationship.

### Solution

Implement a **three-tier emotional model**:

1. **Disposition** (Persistent NPC→PC Relationship) - Renamed from `MoodLevel` to `DispositionLevel`
2. **Mood** (Semi-Persistent NPC State) - New concept, set during staging, persists until next staging
3. **Expression** (Transient Dialogue State) - Inline markers in dialogue that change sprites during typewriter

### Key Features

- **Clear terminology**: Disposition (relationship), Mood (emotional state), Expression (visual)
- **Disposition persists** in Neo4j per NPC-PC pair
- **Mood set during staging** by DM, cached until next staging
- **Expression markers** in dialogue: `*happy*` or `*excited|happy*`
- **Mood affects default expression**: Anxious NPC defaults to worried expression
- **LLM context includes both** disposition and mood for richer responses
- **LLM tool calls**: `change_disposition` and `change_mood` (both require DM approval)
- **Expression sheet generation** via ComfyUI for per-character sprites

---

## Three-Tier Emotional Model

### Tier 1: Disposition (Persistent NPC→PC Relationship)

| Attribute | Value |
|-----------|-------|
| **Scope** | Per NPC-PC pair |
| **Persistence** | Long-term, stored in Neo4j edge `DISPOSITION_TOWARD` |
| **Changes** | Slowly, based on accumulated interactions |
| **Values** | Friendly, Neutral, Suspicious, Hostile, Grateful, Respectful, Dismissive |
| **Purpose** | How the NPC *emotionally feels* about this specific PC (subjective stance) |
| **Changed by** | LLM tool call `change_disposition` (requires DM approval), DM manual override, accumulated relationship points |
| **Example** | "Marcus is suspicious of you after you tried to steal from him last week" |

#### Disposition vs RelationshipLevel

The system maintains **two separate dimensions** for NPC-PC relationships:

| Dimension | Question Answered | Example Values | Example |
|-----------|-------------------|----------------|---------|
| **RelationshipLevel** | "How well do they know each other?" (social distance) | Stranger, Acquaintance, Friend, Ally, Rival, Enemy, Nemesis | "Marcus is an Acquaintance" |
| **DispositionLevel** | "How does the NPC feel about this PC?" (emotional stance) | Hostile, Suspicious, Dismissive, Neutral, Respectful, Friendly, Grateful | "Marcus feels Friendly toward you" |

**Why both?** This allows nuanced combinations:
- "Friendly Stranger" - warm first impression, just met
- "Suspicious Ally" - close relationship, but currently doubts the PC
- "Hostile Acquaintance" - knows the PC, actively dislikes them
- "Grateful Enemy" - opposing factions, but PC did them a favor

**Data model**: Both are stored on the same `DISPOSITION_TOWARD` edge:
```rust
pub struct NpcDispositionState {
    pub npc_id: CharacterId,
    pub pc_id: PlayerCharacterId,
    pub disposition: DispositionLevel,    // Emotional stance (renamed from mood)
    pub relationship: RelationshipLevel,  // Social distance (unchanged)
    pub sentiment: f32,                   // Fine-grained score (-1.0 to 1.0)
    pub disposition_reason: Option<String>,
    pub relationship_points: i32,
}
```

### Tier 2: Mood (Semi-Persistent NPC State)

| Attribute | Value |
|-----------|-------|
| **Scope** | Per NPC (not per-PC) |
| **Persistence** | Per-staging, cached until next staging; Character entity has `default_mood` |
| **Changes** | Between stagings, or during scenes via LLM tool calls |
| **Values** | Happy, Calm, Anxious, Excited, Melancholic, Irritated, Alert, Bored, Fearful, Hopeful, Curious, Contemplative, Amused, Weary, Confident, Nervous |
| **Purpose** | NPC's current emotional state affecting their behavior and default expression |
| **Changed by** | DM sets during staging approval, LLM tool call `change_mood` (requires DM approval), DM manual override |
| **Affects** | Default expression (mood→expression mapping), dialogue tone, LLM context |
| **Example** | "Marcus is anxious because the town is under threat" |

### Tier 3: Expression (Transient Dialogue State)

| Attribute | Value |
|-----------|-------|
| **Scope** | Per dialogue moment |
| **Persistence** | None - changes during typewriter playback |
| **Changes** | Rapidly, multiple times per dialogue via inline markers |
| **Values** | Character-specific sprite names: neutral, happy, sad, angry, surprised, afraid, thoughtful, suspicious, etc. |
| **Purpose** | Visual feedback during dialogue, sprite swapping |
| **Changed by** | Inline markers in dialogue text `*curious*`, `*nervous|afraid*` |
| **Example** | `*curious* "You seek the Heartstone?" *suspicious* "But why?"` |

### How They Interact

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         LLM CONTEXT                                         │
│                                                                             │
│  "Marcus the Merchant:                                                      │
│   - Relationship with player: ACQUAINTANCE (they've met a few times)        │
│   - Disposition toward player: FRIENDLY (likes and trusts the player)       │
│   - Current mood: ANXIOUS (because the town is under threat)                │
│   - Available expressions: neutral, happy, sad, angry, suspicious..."       │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      LLM GENERATES DIALOGUE                                 │
│                                                                             │
│  *worried* "I'm glad you're here, friend." *relieved|happy*                │
│  "Perhaps you can help us with the threat from the north?"                  │
│                                                                             │
│  (Relationship: acquaintance → familiar but not close                       │
│   Disposition: friendly → calls player "friend", trusts them                │
│   Mood: anxious → worried expression, talks about threat                    │
│   Expressions: worried→relieved during dialogue)                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Mood → Expression Mapping (Default Behavior)

Mood provides context to the LLM, which chooses appropriate expressions. The LLM is not forced to use specific expressions - it uses mood as a **bias** for its choices.

Example LLM prompt context:
```
NPC's current mood is ANXIOUS. This should influence your expression choices.
Available expressions: neutral, happy, sad, angry, suspicious, thoughtful, afraid, excited
```

The LLM might naturally gravitate toward `*worried*`, `*nervous|afraid*`, etc. when the mood is anxious.

---

## Current State Analysis

### Existing "Mood" System (Actually Disposition)

**Location**: `crates/domain/src/value_objects/mood.rs`

```rust
pub enum MoodLevel {
    Friendly, Neutral, Suspicious, Hostile, Afraid, 
    Grateful, Annoyed, Curious, Melancholic
}

pub struct NpcMoodState {
    pub npc_id: CharacterId,
    pub pc_id: PlayerCharacterId,
    pub mood: MoodLevel,              // Actually disposition
    pub relationship: RelationshipLevel,
    pub sentiment: f32,
    pub mood_reason: Option<String>,  // disposition_reason
    pub relationship_points: i32,
}
```

**Storage**: Neo4j edge `(npc:Character)-[:DISPOSITION_TOWARD]->(pc:PlayerCharacter)`

**Current Usage**:
- DM can set via `SetNpcMood` WebSocket request
- Tracked but **not displayed visually** in VN UI
- Used in LLM context for dialogue generation

### Existing Staging System

**Location**: `crates/domain/src/entities/staging.rs`

```rust
pub struct StagedNpc {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: String,
    // NOTE: No mood field currently exists!
}
```

**Gap**: Staging does not include NPC mood. DM cannot set mood during staging approval.

### Existing DirectorialContext

**Location**: `crates/protocol/src/messages.rs`

```rust
pub struct DirectorialContext {
    pub scene_notes: String,
    pub tone: String,
    pub npc_motivations: Vec<NpcMotivationData>,
    pub forbidden_topics: Vec<String>,
}

pub struct NpcMotivationData {
    pub character_id: String,
    pub mood: String,           // Free-form string for DM guidance
    pub immediate_goal: String,
    pub secret_agenda: Option<String>,
}
```

**Issues identified**:
1. `mood` field is free-form string, not connected to staging system
2. The field name "mood" conflicts with our new `MoodState` concept
3. Values in `npc_motivation.rs` UI include motivational states ("Greedy", "Dutiful", "Conflicted") rather than emotional moods

**Resolution**: 
- Rename `mood` to `emotional_guidance` (free-form DM guidance text)
- Rename UI dropdown from "Mood" to "Demeanor" 
- Keep as free-form to allow nuanced guidance like "Conflicted about revealing secrets"

### Existing Dialogue System

**Location**: `crates/player-ui/src/presentation/state/dialogue_state.rs`

```rust
pub struct DialogueState {
    pub speaker_name: Signal<String>,
    pub speaker_id: Signal<Option<String>>,
    pub full_text: Signal<String>,
    pub displayed_text: Signal<String>,
    pub is_typing: Signal<bool>,
    // ... no expression signals
}
```

**Gap**: No expression tracking, no marker parsing, no sprite switching.

### Existing Character Sprites

**Location**: `crates/player-ui/src/presentation/components/visual_novel/character_sprite.rs`

**Current behavior**:
- Renders single static sprite from `sprite_asset` URL
- No expression swapping
- `CharacterData.emotion: Option<String>` exists in protocol but is **unused**

---

## Proposed System Design

### Marker Format Specification

#### Syntax

```
*word*           → mood and expression are the same
*mood|expression* → custom mood displayed, uses expression sprite
*action text*    → transient action (not in expression vocabulary)
```

#### Examples

| Marker | Mood Displayed | Expression Used | Type |
|--------|---------------|-----------------|------|
| `*happy*` | happy | happy | mood |
| `*excited|happy*` | excited | happy | mood with mapping |
| `*nervous|afraid*` | nervous | afraid | mood with mapping |
| `*sighs*` | — | — | action |
| `*slams fist on table*` | — | — | action |

#### Classification Logic

```
Is it multi-word? (contains space after trimming)
  └─ YES → Action (gray)
  └─ NO → Is it pipe format (*mood|expression*)?
           └─ YES → Is expression in available_expressions?
                     └─ YES → Valid mood (green)
                     └─ NO → Fallback mood (red)
           └─ NO → Is word in available_expressions?
                     └─ YES → Valid mood (green)
                     └─ NO → Is word in character's known_actions?
                               └─ YES → Action (gray)
                               └─ NO → Fallback mood (red)
```

### Default Expression Vocabulary

Standard set all characters start with:
- neutral (default)
- happy
- sad
- angry
- surprised
- afraid
- thoughtful
- suspicious

Characters can add custom expressions via the Expression Config Editor.

### Terminology Clarification

The codebase currently conflates several concepts under "mood". This plan introduces clear terminology:

| Old Term | New Term | Scope | Purpose |
|----------|----------|-------|---------|
| `MoodLevel` (domain) | `DispositionLevel` | Per NPC-PC | How NPC feels about PC |
| `NpcMoodState` | `NpcDispositionState` | Per NPC-PC | Full disposition data |
| `SetNpcMood` (protocol) | `SetNpcDisposition` | Per NPC-PC | Change disposition |
| `NpcMoodPanel` (UI) | `NpcDispositionPanel` | Per NPC-PC | DM manages dispositions |
| `NpcMotivationData.mood` | `NpcMotivationData.emotional_guidance` | Per NPC (scene) | Free-form DM guidance |
| "Mood" label in npc_motivation.rs | "Demeanor" label | Per NPC (scene) | UI dropdown for DM |
| (new) `MoodState` | `MoodState` | Per NPC | NPC's emotional state |

### Disposition Values

New `DispositionLevel` enum (replacing `MoodLevel` for relationship):
- Hostile
- Suspicious
- Dismissive
- Neutral (default)
- Respectful
- Friendly
- Grateful

### Mood Values

New `MoodState` enum:
- Happy
- Calm
- Anxious
- Excited
- Melancholic
- Irritated
- Alert
- Bored
- Fearful
- Hopeful
- Curious
- Contemplative
- Amused
- Weary
- Confident
- Nervous

---

## UI Mockups

### Mockup 1: Character Sprite with Mood Badge (Visual Novel Scene)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           SCENE BACKDROP                                     │
│                                                                              │
│    ┌──────────────┐                              ┌──────────────┐           │
│    │              │                              │              │           │
│    │  [PC Sprite] │                              │ [NPC Sprite] │           │
│    │   neutral    │                              │   curious    │◄─ Expression
│    │              │                              │              │   changes
│    │              │                              │              │   during
│    │              │                              │    ┌─────┐   │   dialogue
│    │              │                              │    │*cur │◄──┼── Mood badge
│    │              │                              │    │ious*│   │   (gold text,
│    └──────────────┘                              └────┴─────┴───┘   dark bg)
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│ ┌────────────────────────────────────────────────────────────────────────┐   │
│ │  Marcus the Merchant                                      *curious*    │◄──┼─ Mood tag
│ ├────────────────────────────────────────────────────────────────────────┤   │  next to name
│ │                                                                        │   │
│ │  "You've come a long way, traveler. *narrows eyes* But I wonder...    │◄──┼─ Action marker
│ │   what brings you to these forgotten ruins?"▌                          │   │  shown inline
│ │                                                                        │   │  (italicized)
│ └────────────────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Key Elements**:
- **Mood badge** on character sprite (top-right corner): `*curious*` in gold text on dark background
- **Mood tag** next to speaker name in dialogue box
- **Action markers** inline in dialogue text (italicized)
- Sprite image changes based on expression (e.g., `marcus_curious.png`)

### Mockup 2: Dialogue Box with Mood Tag and Action Display

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ┌─────────────────────────────────────────────┬─────────────────────────┐  │
│  │  Elara the Sage                              │  *thoughtful*          │  │
│  └─────────────────────────────────────────────┴─────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │                                                                        │  │
│  │  *pauses, considering* "The ancient texts speak of a power that      │  │
│  │  lies dormant beneath these stones..." *leans forward* *excited*      │  │
│  │  "Could it be that you seek the Heartstone?"▌                         │  │
│  │                                                                        │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  [Yes, I seek the Heartstone]                                         │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  [What can you tell me about it?]                                     │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  ┌────────────────────────────────────────────────────┐ ┌───────────┐ │  │
│  │  │ Type your response... (use *mood* for expressions) │ │   Send    │ │  │
│  │  └────────────────────────────────────────────────────┘ └───────────┘ │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Mockup 3: DM Approval Popup with Editable Dialogue & Live Validation

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │                         Approval Required                              │  │
│  ├────────────────────────────────────────────────────────────────────────┤  │
│  │                                                                        │  │
│  │  Action from: Marcus the Merchant                                     │  │
│  │  Relationship: Acquaintance | Disposition: Friendly | Mood: Anxious   │  │
│  │  Available expressions: neutral, happy, sad, angry, suspicious,       │  │
│  │                         thoughtful, afraid, excited                   │  │
│  │                                                                        │  │
│  ├────────────────────────────────────────────────────────────────────────┤  │
│  │  DIALOGUE (editable)                                                  │  │
│  │  ┌──────────────────────────────────────────────────────────────────┐  │  │
│  │  │                                                                  │  │  │
│  │  │  *curious* "You seek the Heartstone?" *narrows eyes* *nervous*  │  │  │
│  │  │   ───────                              ────────────   ───────    │  │  │
│  │  │   green ✓                              gray           red ⚠     │  │  │
│  │  │                                                        ↑        │  │  │
│  │  │                               ┌─────────────────────────────┐   │  │  │
│  │  │                               │ "nervous" not available.    │   │  │  │
│  │  │                               │ Will fallback to default.   │   │  │  │
│  │  │                               │ Tip: *nervous|afraid*       │   │  │  │
│  │  │                               └─────────────────────────────┘   │  │  │
│  │  │                                                                  │  │  │
│  │  │  "I've seen your kind before." *crosses arms*                   │  │  │
│  │  │                                 ─────────────                    │  │  │
│  │  │                                 gray (action)                    │  │  │
│  │  │                                                                  │  │  │
│  │  └──────────────────────────────────────────────────────────────────┘  │  │
│  │                                                                        │  │
│  │  EXPRESSION TIMELINE                                                  │  │
│  │  ┌──────────────────────────────────────────────────────────────────┐  │  │
│  │  │                                                                  │  │  │
│  │  │  ┌────────┐      ┌────────┐      ┌────────┐                     │  │  │
│  │  │  │curious │  ──► │   —    │  ──► │neutral │                     │  │  │
│  │  │  │ [img]  │      │*narrows│      │ [img]  │                     │  │  │
│  │  │  │   ✓    │      │ eyes*  │      │   ⚠    │                     │  │  │
│  │  │  └────────┘      └────────┘      └────────┘                     │  │  │
│  │  │   start           action         fallback                       │  │  │
│  │  │                                  (nervous)                      │  │  │
│  │  │                                                                  │  │  │
│  │  └──────────────────────────────────────────────────────────────────┘  │  │
│  │                                                                        │  │
│  │  VALIDATION                                                           │  │
│  │  ┌──────────────────────────────────────────────────────────────────┐  │  │
│  │  │  ✓ Valid: curious                                               │  │  │
│  │  │  ⚠ Fallback: nervous → neutral (default)                        │  │  │
│  │  │  ○ Actions: narrows eyes, crosses arms                          │  │  │
│  │  └──────────────────────────────────────────────────────────────────┘  │  │
│  │                                                                        │  │
│  ├────────────────────────────────────────────────────────────────────────┤  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                │  │
│  │  │     Send     │  │  Regenerate  │  │    Reject    │                │  │
│  │  │   (green)    │  │    (blue)    │  │    (red)     │                │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘                │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Key Elements**:
- **Disposition & Mood display**: Shows NPC's disposition toward PC and current mood
- **Editable textarea**: DM can freely edit text including markers
- **Colored underlines**: Green (valid), Red (fallback), Gray (action)
- **Hover tooltips**: Shows why invalid, suggests pipe format fix
- **Expression timeline**: Visual sprite sequence preview
- **Validation summary**: Quick overview of parsed markers
- **Buttons**: Send (approve), Regenerate (ask LLM again), Reject

### Mockup 4: Staging Approval with NPC Mood Selection

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  🎭 Stage the Scene                                              [X]        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  📍 The Bar Counter                                                         │
│     Rusty Anchor Tavern                                                     │
│  🕐 Day 3, Evening (7:30 PM)                                                │
│                                                                             │
│  ─── NPCs to Stage ─────────────────────────────────────────────────────── │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ [✓] Marcus the Bartender                                            │   │
│  │     Works here (Evening shift)                                      │   │
│  │     Mood: [▼ Anxious     ] ← DM sets mood for this staging         │   │
│  │           ┌─────────────┐                                           │   │
│  │           │ Happy       │                                           │   │
│  │           │ Calm        │                                           │   │
│  │           │ Anxious  ✓  │                                           │   │
│  │           │ Irritated   │                                           │   │
│  │           │ Alert       │                                           │   │
│  │           └─────────────┘                                           │   │
│  │     Hidden: [ ]                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ [✓] Old Sal                                                         │   │
│  │     Frequents (Often, Evening)                                      │   │
│  │     Mood: [▼ Melancholic ]                                          │   │
│  │     Hidden: [ ]                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ─── Cache Duration ────────────────────────────────────────────────────── │
│  Valid for: [▼ 3 hours ] (until 10:30 PM game time)                         │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │                        ✓ Approve Staging                              │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Key Elements**:
- **Per-NPC mood dropdown**: DM sets mood for each NPC in this staging
- **Mood persists** with staging cache until expiry or next staging

### Mockup 5: Character Editor - Expression Configuration Tab

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  Character: Marcus the Merchant                                    [×] Close │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  [Basics]  [Character Sheet]  [Motivations]  [Assets]  [Expressions]        │
│  ─────────────────────────────────────────────────────────────────────────── │
│                                                                              │
│  EXPRESSION CONFIGURATION                                                    │
│  ────────────────────────────────────────────────────────────────────────── │
│                                                                              │
│  Default Mood: [▼ Calm            ] (when no staging override)              │
│                                                                              │
│  Available Expressions                                                       │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐   │  │
│  │  │neutral │ │ happy  │ │  sad   │ │ angry  │ │suspici │ │thought │   │  │
│  │  │ [img]  │ │ [img]  │ │ [img]  │ │ [img]  │ │  ous   │ │  ful   │   │  │
│  │  │   ✓    │ │   ✓    │ │   ✓    │ │   ✓    │ │ [img]  │ │ [img]  │   │  │
│  │  └────────┘ └────────┘ └────────┘ └────────┘ │   ✓    │ │   ✓    │   │  │
│  │                                              └────────┘ └────────┘   │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐                                    │  │
│  │  │afraid  │ │excited │ │  [+]   │ ← Add custom expression            │  │
│  │  │ [img]  │ │ [img]  │ │  Add   │                                    │  │
│  │  │   ✓    │ │   ✓    │ │        │                                    │  │
│  │  └────────┘ └────────┘ └────────┘                                    │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  Default Expression: [▼ neutral            ]                                 │
│                                                                              │
│  ────────────────────────────────────────────────────────────────────────── │
│                                                                              │
│  CHARACTER ACTIONS (for action markers)                                      │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  [sighs] [× ] [laughs nervously] [× ] [strokes beard] [× ] [+ Add]    │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│  Help: Actions appear inline during dialogue (e.g., *sighs heavily*)        │
│                                                                              │
│  ────────────────────────────────────────────────────────────────────────── │
│                                                                              │
│  EXPRESSION SHEET GENERATION                                                 │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  Status: No expression sheet generated                                │  │
│  │                                                                        │  │
│  │  [  Generate Expression Sheet  ]                                       │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│                                              [Cancel]  [Save Changes]        │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Mockup 6: DM NPC Disposition Panel (Renamed from Mood Panel)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  NPC States                                                                  │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  Marcus the Merchant                                                         │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  Current Mood: Anxious                     [Change Mood ▼]            │  │
│  │                                                                        │  │
│  │  ─── Relationships with PCs ───────────────────────────────────────── │  │
│  │                                                                        │  │
│  │  Toward Kira:                                                         │  │
│  │  ├─ Relationship: Acquaintance  (social distance)                     │  │
│  │  ├─ Disposition: [▼ Friendly ]  (emotional stance)                    │  │
│  │  └─ Reason: "Helped him find his lost shipment"                       │  │
│  │                                                                        │  │
│  │  Toward Aldric:                                                       │  │
│  │  ├─ Relationship: Stranger                                            │  │
│  │  ├─ Disposition: [▼ Suspicious ]                                      │  │
│  │  └─ Reason: "Caught him snooping in the back room"                    │  │
│  │                                                                        │  │
│  │  Note: Relationship = how well they know each other                   │  │
│  │        Disposition = how NPC feels about them                         │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  Old Sal                                                                     │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  Current Mood: Melancholic                 [Change Mood ▼]            │  │
│  │                                                                        │  │
│  │  Toward Kira:                                                         │  │
│  │  ├─ Relationship: Stranger                                            │  │
│  │  ├─ Disposition: [▼ Neutral ]                                         │  │
│  │  └─ (No specific reason)                                              │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Key Elements**:
- **Mood**: NPC's current emotional state (per-NPC, not per-PC)
- **Relationship**: Social distance - how well they know each other (unchanged from existing system)
- **Disposition**: Emotional stance - how the NPC feels about this PC (renamed from "mood")
- Both Relationship and Disposition are per NPC-PC pair

### Mockup 7: Player Input with Marker Validation

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  ⚠ Warning: Expression 'nervous' not available for your character.    │  │
│  │            Will use default expression.                               │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐  │
│  │  Available expressions: neutral, happy, sad, angry, afraid            │  │
│  │  Tip: Use *mood|expression* to map custom moods: *nervous|afraid*     │  │
│  └────────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────────┐   │
│  │ *nervous|afraid* I... I don't know what you mean.         │ │  Send  │   │
│  └───────────────────────────────────────────────────────────┘ └────────┘   │
│                                                                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Data Model Changes

### Renamed: DispositionLevel (was MoodLevel)

**File**: `crates/domain/src/value_objects/disposition.rs` (renamed from mood.rs)

```rust
use serde::{Deserialize, Serialize};

/// Disposition level - how an NPC emotionally feels about a specific PC
/// 
/// This is SEPARATE from RelationshipLevel (social distance).
/// - RelationshipLevel: How well they know each other (Stranger → Ally)
/// - DispositionLevel: How the NPC feels about the PC (Hostile → Grateful)
/// 
/// Both are stored on the same DISPOSITION_TOWARD edge, allowing combinations like
/// "Suspicious Ally" or "Friendly Stranger".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispositionLevel {
    Hostile,      // Actively wants to harm/hinder the PC
    Suspicious,   // Distrusts, wary of the PC
    Dismissive,   // Doesn't care about the PC, ignores them
    #[default]
    Neutral,      // No strong feelings either way
    Respectful,   // Regards the PC positively, professional
    Friendly,     // Likes the PC, warm toward them
    Grateful,     // Owes the PC, deeply appreciative
}

/// NPC disposition state toward a specific PC
/// 
/// Combines two dimensions:
/// - disposition: Emotional stance (how they feel)
/// - relationship: Social distance (how well they know each other)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDispositionState {
    pub npc_id: CharacterId,
    pub pc_id: PlayerCharacterId,
    pub disposition: DispositionLevel,    // Emotional stance (renamed from mood)
    pub relationship: RelationshipLevel,  // Social distance (unchanged)
    pub sentiment: f32,                   // Fine-grained score (-1.0 to 1.0)
    pub disposition_reason: Option<String>,  // renamed from mood_reason
    pub relationship_points: i32,
}
```

### New: MoodState

**File**: `crates/domain/src/value_objects/mood.rs` (repurposed)

```rust
use serde::{Deserialize, Serialize};

/// Mood state - NPC's current emotional state (semi-persistent, per-staging)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoodState {
    Happy,
    #[default]
    Calm,
    Anxious,
    Excited,
    Melancholic,
    Irritated,
    Alert,
    Bored,
    Fearful,
    Hopeful,
    Curious,
    Contemplative,
    Amused,
    Weary,
    Confident,
    Nervous,
}

impl MoodState {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Happy => "Happy",
            Self::Calm => "Calm",
            // ... etc
        }
    }
}
```

### New: ExpressionConfig

**File**: `crates/domain/src/value_objects/expression_config.rs` (NEW)

```rust
use serde::{Deserialize, Serialize};

/// Expression configuration for a character
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ExpressionConfig {
    /// Available expression names (e.g., ["neutral", "happy", "sad"])
    pub expressions: Vec<String>,
    
    /// Custom actions this character uses (e.g., ["sighs", "laughs nervously"])
    pub actions: Vec<String>,
    
    /// Default expression when no mood specified or fallback needed
    pub default_expression: String,
}

impl ExpressionConfig {
    pub fn new() -> Self {
        Self {
            expressions: vec![
                "neutral".to_string(),
                "happy".to_string(),
                "sad".to_string(),
                "angry".to_string(),
                "surprised".to_string(),
                "afraid".to_string(),
                "thoughtful".to_string(),
                "suspicious".to_string(),
            ],
            actions: Vec::new(),
            default_expression: "neutral".to_string(),
        }
    }
}
```

### Modified: Character Entity

**File**: `crates/domain/src/entities/character.rs`

```rust
pub struct Character {
    // ... existing fields ...
    
    /// Default mood when not overridden by staging
    #[serde(default)]
    pub default_mood: MoodState,
    
    /// Expression configuration for this character
    #[serde(default)]
    pub expression_config: ExpressionConfig,
}
```

### Modified: StagedNpc

**File**: `crates/domain/src/entities/staging.rs`

```rust
pub struct StagedNpc {
    pub character_id: CharacterId,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub is_hidden_from_players: bool,
    pub reasoning: String,
    
    /// NEW: Mood for this staging (overrides character.default_mood)
    pub mood: MoodState,
}
```

---

## Protocol Changes

### Renamed Messages

| Old Name | New Name |
|----------|----------|
| `SetNpcMood` | `SetNpcDisposition` |
| `NpcMoodChanged` | `NpcDispositionChanged` |
| `GetNpcMoods` | `GetNpcDispositions` |
| `NpcMoodsResponse` | `NpcDispositionsResponse` |
| `NpcMoodData` | `NpcDispositionData` |

### DirectorialContext Changes

```rust
pub struct NpcMotivationData {
    pub character_id: String,
    pub emotional_guidance: String,  // Renamed from 'mood' - free-form DM guidance
    pub immediate_goal: String,
    pub secret_agenda: Option<String>,
}
```

Note: `emotional_guidance` remains a free-form string (not an enum) to allow nuanced DM guidance like "Conflicted about revealing secrets" or "Determined to protect the village at any cost".

### New Messages

```rust
// Set NPC's current mood (not disposition)
RequestPayload::SetNpcMood {
    npc_id: String,
    mood: String,  // MoodState variant
    reason: Option<String>,
}

// NPC mood changed (broadcast to DMs)
ServerMessage::NpcMoodChanged {
    npc_id: String,
    npc_name: String,
    mood: String,
    reason: Option<String>,
}
```

### Modified: StagedNpcInfo

```rust
pub struct StagedNpcInfo {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub is_present: bool,
    pub reasoning: String,
    pub is_hidden_from_players: bool,
    
    /// NEW: Mood for this staging
    pub mood: String,
    
    /// NEW: Default mood from character
    pub default_mood: String,
}
```

### Modified: ApprovedNpcInfo

```rust
pub struct ApprovedNpcInfo {
    pub character_id: String,
    pub is_present: bool,
    pub reasoning: Option<String>,
    pub is_hidden_from_players: bool,
    
    /// NEW: Mood DM selected for this NPC
    pub mood: String,
}
```

### Modified: CharacterData

```rust
pub struct CharacterData {
    pub id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub position: CharacterPosition,
    pub is_speaking: bool,
    
    /// Current expression for sprite lookup
    pub current_expression: Option<String>,
    
    /// Current expression mood text to display
    pub current_mood: Option<String>,
    
    /// NPC's overall mood state (from staging)
    pub mood_state: Option<String>,
    
    /// Disposition toward current PC (if applicable)
    pub disposition: Option<String>,
    
    /// Available expressions for this character
    pub available_expressions: Vec<String>,
    
    /// Known actions for this character
    pub available_actions: Vec<String>,
    
    /// Default expression for fallback
    pub default_expression: String,
}
```

---

## Implementation Phases

### Phase 0: Disposition Rename Refactor (Prerequisite) (3-4 hours)

**Status**: Not Started

**Purpose**: Rename `MoodLevel` → `DispositionLevel` throughout codebase to clarify terminology before adding new Mood concept.

**Files to modify**:

*Domain layer*:
- `crates/domain/src/value_objects/mood.rs` → rename file to `disposition.rs`
- `crates/domain/src/value_objects/mod.rs` - update exports

*Protocol layer*:
- `crates/protocol/src/messages.rs` - rename message types, rename `NpcMotivationData.mood` → `emotional_guidance`
- `crates/protocol/src/requests.rs` - rename request types

*Engine layer*:
- `crates/engine-app/src/application/services/mood_service.rs` → rename file to `disposition_service.rs`
- `crates/engine-app/src/application/services/mod.rs` - update exports
- `crates/engine-adapters/src/infrastructure/persistence/mood_repository.rs` → rename if exists
- `crates/engine-adapters/src/infrastructure/websocket/handlers/*.rs` - update handlers

*Player UI layer*:
- `crates/player-ui/src/presentation/components/dm_panel/npc_mood_panel.rs` → rename file to `npc_disposition_panel.rs`
- `crates/player-ui/src/presentation/components/dm_panel/npc_motivation.rs` - rename "Mood" → "Demeanor", `MOOD_OPTIONS` → `DEMEANOR_OPTIONS`
- `crates/player-ui/src/presentation/components/dm_panel/mod.rs` - update exports
- `crates/player-ui/src/presentation/state/game_state.rs` - rename signals
- `crates/player-ui/src/presentation/handlers/session_message_handler.rs` - update handlers

**Tasks**:
- [ ] Rename `MoodLevel` enum to `DispositionLevel`
- [ ] Rename `NpcMoodState` to `NpcDispositionState`
- [ ] Rename `mood` field to `disposition` in `NpcDispositionState`
- [ ] Rename `mood_reason` to `disposition_reason`
- [ ] Keep `relationship: RelationshipLevel` field unchanged (separate dimension)
- [ ] Rename `MoodService` to `DispositionService`
- [ ] Rename protocol messages: `SetNpcMood` → `SetNpcDisposition`, etc.
- [ ] Rename UI component: `NpcMoodPanel` → `NpcDispositionPanel`
- [ ] Update all imports and usages
- [ ] Update Neo4j edge property names (if stored as "mood" → "disposition")
- [ ] Add documentation comments explaining Disposition vs Relationship distinction

**DirectorialContext cleanup** (resolve terminology conflicts):
- [ ] Rename `NpcMotivationData.mood` → `emotional_guidance` (free-form DM guidance)
- [ ] Update `npc_motivation.rs` UI label from "Mood" to "Demeanor"
- [ ] Keep `MOOD_OPTIONS` in `npc_motivation.rs` as motivational states (Greedy, Dutiful, etc.) - rename constant to `DEMEANOR_OPTIONS`
- [ ] Ensure `npc_mood_panel.rs` (now `npc_disposition_panel.rs`) uses `DispositionLevel` values

**UI consistency fixes**:
- [ ] `npc_disposition_panel.rs`: Update `MOOD_OPTIONS` → use `DispositionLevel::all()` values
- [ ] `npc_motivation.rs`: Rename constant `MOOD_OPTIONS` → `DEMEANOR_OPTIONS`
- [ ] `npc_motivation.rs`: Rename UI label "Mood" → "Demeanor"

**Verification**:
```bash
cargo check --workspace
cargo test --workspace
# Verify no remaining references to old names:
rg "MoodLevel" --type rust  # Should only find new MoodState references
rg "SetNpcMood" --type rust  # Should find new SetNpcMood (for MoodState), not old
rg "NpcMotivationData.*mood" --type rust  # Should find emotional_guidance instead
```

---

### Phase 1: New Mood System (Domain & Protocol) (2-3 hours)

**Status**: Not Started

**Files to create/modify**:
- NEW: `crates/domain/src/value_objects/mood.rs` (new MoodState enum)
- NEW: `crates/domain/src/value_objects/expression_config.rs`
- NEW: `crates/domain/src/value_objects/dialogue_markers.rs`
- MOD: `crates/domain/src/entities/character.rs` - add `default_mood`, `expression_config`
- MOD: `crates/domain/src/entities/player_character.rs` - add `expression_config`
- MOD: `crates/domain/src/entities/staging.rs` - add `mood` to `StagedNpc`
- MOD: `crates/domain/src/value_objects/mod.rs`
- MOD: `crates/protocol/src/messages.rs` - add mood-related messages
- MOD: `crates/protocol/src/requests.rs` - add `SetNpcMood` request

**Tasks**:
- [ ] Create `MoodState` enum with all mood values
- [ ] Create `ExpressionConfig` struct
- [ ] Create `DialogueMarker` types and parser
- [ ] Add `default_mood` and `expression_config` to Character
- [ ] Add `expression_config` to PlayerCharacter
- [ ] Add `mood` to `StagedNpc`
- [ ] Add protocol messages for mood changes
- [ ] Unit tests for dialogue marker parser

**Verification**:
```bash
cargo check -p wrldbldr-domain
cargo check -p wrldbldr-protocol
cargo test -p wrldbldr-domain
```

---

### Phase 2: Staging System Mood Integration (2-3 hours)

**Status**: Not Started

**Files to modify**:
- MOD: `crates/protocol/src/messages.rs` - update `StagedNpcInfo`, `ApprovedNpcInfo`
- MOD: `crates/engine-adapters/src/infrastructure/persistence/staging_repository.rs`
- MOD: `crates/engine-adapters/src/infrastructure/websocket/handlers/staging.rs`
- MOD: `crates/engine-app/src/application/services/staging_service.rs`
- MOD: `crates/player-ui/src/presentation/components/dm_panel/staging_approval.rs`

**Tasks**:
- [ ] Add `mood` and `default_mood` to `StagedNpcInfo`
- [ ] Add `mood` to `ApprovedNpcInfo`
- [ ] Persist mood on `INCLUDES_NPC` edge in Neo4j
- [ ] Load mood when fetching staging
- [ ] Update staging approval handler to accept mood
- [ ] Update staging approval UI with mood dropdown per NPC

**Verification**:
```bash
cargo check --workspace
```

---

### Phase 3: Persistence & Repository Updates (2 hours)

**Status**: Not Started

**Files to modify**:
- MOD: `crates/engine-adapters/src/infrastructure/persistence/character_repository.rs`
- MOD: `crates/engine-adapters/src/infrastructure/persistence/player_character_repository.rs`
- NEW: `crates/engine-app/src/application/services/mood_service.rs` (new service for NPC mood)

**Tasks**:
- [ ] Persist `default_mood` and `expression_config` on Character node
- [ ] Persist `expression_config` on PlayerCharacter node
- [ ] Create `MoodService` for NPC mood operations (distinct from `DispositionService`)
- [ ] Add `SetNpcMood` handler to WebSocket

**Verification**:
```bash
cargo check --workspace
```

---

### Phase 4: LLM Prompt Updates (1.5 hours)

**Status**: Not Started

**Files to modify**:
- MOD: `crates/domain/src/value_objects/prompt_templates.rs`
- MOD: `crates/engine-app/src/application/services/llm/prompt_builder.rs`
- MOD: `crates/engine-adapters/src/infrastructure/websocket/websocket_helpers.rs`

**Tasks**:
- [ ] Update prompt template with marker instructions
- [ ] Include both disposition AND mood in LLM context
- [ ] Add `{available_expressions}`, `{available_actions}` placeholders
- [ ] Add `change_disposition` tool definition (rename from `change_mood`)
- [ ] Add `change_mood` tool definition (new, for mood changes)

**Verification**:
```bash
cargo check --workspace
```

---

### Phase 5: Expression Sheet Generation (3 hours)

**Status**: Not Started

**Files to create/modify**:
- NEW: `crates/engine-app/src/application/services/expression_sheet_service.rs`
- MOD: `crates/engine-app/src/application/services/mod.rs`
- MOD: `crates/engine-adapters/src/infrastructure/http/asset_routes.rs`

**Tasks**:
- [ ] Create `ExpressionSheetService`
- [ ] Add endpoint for queueing expression sheet generation
- [ ] Create post-processing to slice grid into individual sprites
- [ ] Store sliced sprites with expression name in filename

**Verification**:
```bash
cargo check --workspace
```

---

### Phase 6: Typewriter with Expression Changes (3-4 hours)

**Status**: Not Started

**Files to modify**:
- MOD: `crates/player-ui/src/presentation/state/dialogue_state.rs`
- MOD: `crates/player-ui/src/presentation/components/visual_novel/dialogue_box.rs`

**Tasks**:
- [ ] Add expression signals to `DialogueState`
- [ ] Parse markers from `full_text` when dialogue starts
- [ ] Update signals during typewriter animation
- [ ] Display mood tag in speaker header
- [ ] Display action markers inline (italicized)

**Verification**:
```bash
cargo check -p wrldbldr-player-ui
dx build --platform web
```

---

### Phase 7: Character Sprite Updates (2 hours)

**Status**: Not Started

**Files to modify**:
- MOD: `crates/player-ui/src/presentation/components/visual_novel/character_sprite.rs`
- MOD: `crates/player-ui/styles/input.css`

**Tasks**:
- [ ] Build expression-specific sprite URL
- [ ] Add mood badge overlay
- [ ] CSS for mood badge styling
- [ ] Fallback to base sprite

**Verification**:
```bash
dx build --platform web
```

---

### Phase 8: Player Input Validation (1.5 hours)

**Status**: Not Started

**Files to modify**:
- MOD: `crates/player-ui/src/presentation/components/visual_novel/choice_menu.rs`

**Tasks**:
- [ ] Parse markers from player input
- [ ] Validate against PC's expressions
- [ ] Show warning for unavailable expressions
- [ ] Suggest pipe format fix

**Verification**:
```bash
dx build --platform web
```

---

### Phase 9: Expression Config Editor (3-4 hours)

**Status**: Not Started

**Files to create/modify**:
- NEW: `crates/player-ui/src/presentation/components/creator/expression_config_editor.rs`
- MOD: `crates/player-ui/src/presentation/components/creator/character_form.rs`
- MOD: `crates/player-ui/src/presentation/components/creator/mod.rs`

**Tasks**:
- [ ] Create `ExpressionConfigEditor` component
- [ ] Default mood dropdown
- [ ] Expression grid with thumbnails
- [ ] Actions tag input
- [ ] Wire into character form as tab

**Verification**:
```bash
dx build --platform web
```

---

### Phase 10: Expression Sheet Generation UI (2-3 hours)

**Status**: Not Started

**Files to create/modify**:
- NEW: `crates/player-ui/src/presentation/components/creator/expression_sheet_modal.rs`
- NEW: `crates/player-ui/src/presentation/components/creator/expression_sheet_selection.rs`

**Tasks**:
- [ ] Create generation modal
- [ ] Create post-generation selection UI
- [ ] Expression mapping dropdowns

**Verification**:
```bash
dx build --platform web
```

---

### Phase 11: DM Approval Marker Support (2 hours)

**Status**: Not Started

**Files to modify**:
- MOD: `crates/player-ui/src/presentation/components/dm_panel/approval_popup.rs`

**Tasks**:
- [ ] Replace readonly dialogue with editable textarea
- [ ] Parse and highlight markers
- [ ] Show expression timeline
- [ ] Validation summary
- [ ] Disable Send on syntax errors

**Verification**:
```bash
dx build --platform web
```

---

### Phase 12: Testing & Polish (2-3 hours)

**Status**: Not Started

**Tasks**:
- [ ] End-to-end test with LLM
- [ ] Test staging mood selection
- [ ] Test disposition vs mood separation
- [ ] Test expression changes
- [ ] Fix styling issues
- [ ] Update system documentation

**Verification**:
```bash
cargo check --workspace
cargo test --workspace
cargo xtask arch-check
dx build --platform web
```

---

## File Change Summary

### New Files (12 files)

| File | Purpose |
|------|---------|
| `crates/domain/src/value_objects/disposition.rs` | DispositionLevel, NpcDispositionState (renamed from mood.rs) |
| `crates/domain/src/value_objects/mood.rs` | MoodState enum (repurposed) |
| `crates/domain/src/value_objects/expression_config.rs` | ExpressionConfig struct |
| `crates/domain/src/value_objects/dialogue_markers.rs` | Parser and marker types |
| `crates/engine-app/src/application/services/mood_service.rs` | NPC mood operations |
| `crates/engine-app/src/application/services/expression_sheet_service.rs` | Expression sheet generation |
| `crates/player-ui/src/presentation/components/creator/expression_config_editor.rs` | Expression config UI |
| `crates/player-ui/src/presentation/components/creator/expression_sheet_modal.rs` | Generation modal |
| `crates/player-ui/src/presentation/components/creator/expression_sheet_selection.rs` | Post-generation selection |
| `crates/player-ui/src/presentation/components/dm_panel/npc_disposition_panel.rs` | Renamed from npc_mood_panel.rs |

### Modified Files (20+ files)

| File | Changes |
|------|---------|
| `crates/domain/src/entities/character.rs` | Add `default_mood`, `expression_config` |
| `crates/domain/src/entities/player_character.rs` | Add `expression_config` |
| `crates/domain/src/entities/staging.rs` | Add `mood` to `StagedNpc` |
| `crates/domain/src/value_objects/mod.rs` | Export new modules |
| `crates/domain/src/value_objects/prompt_templates.rs` | Add marker instructions |
| `crates/protocol/src/messages.rs` | Rename disposition messages, add mood messages, update CharacterData, rename `NpcMotivationData.mood` → `emotional_guidance` |
| `crates/protocol/src/requests.rs` | Rename `SetNpcMood` → `SetNpcDisposition`, add new `SetNpcMood` |
| `crates/player-ui/src/presentation/components/dm_panel/npc_motivation.rs` | Rename "Mood" → "Demeanor", `MOOD_OPTIONS` → `DEMEANOR_OPTIONS` |
| `crates/engine-app/src/application/services/mood_service.rs` | Rename to `disposition_service.rs` |
| `crates/engine-app/src/application/services/llm/prompt_builder.rs` | Include disposition and mood |
| `crates/engine-adapters/src/infrastructure/persistence/character_repository.rs` | Persist new fields |
| `crates/engine-adapters/src/infrastructure/persistence/staging_repository.rs` | Persist staging mood |
| `crates/engine-adapters/src/infrastructure/websocket/handlers/staging.rs` | Handle mood in staging |
| `crates/engine-adapters/src/infrastructure/websocket/handlers/misc.rs` | Rename disposition handlers |
| `crates/player-ui/src/presentation/state/dialogue_state.rs` | Add expression signals |
| `crates/player-ui/src/presentation/state/game_state.rs` | Rename disposition signals |
| `crates/player-ui/src/presentation/components/visual_novel/dialogue_box.rs` | Mood tag, action display |
| `crates/player-ui/src/presentation/components/visual_novel/character_sprite.rs` | Expression sprites, mood badge |
| `crates/player-ui/src/presentation/components/visual_novel/choice_menu.rs` | Player input validation |
| `crates/player-ui/src/presentation/components/creator/character_form.rs` | Add Expressions tab |
| `crates/player-ui/src/presentation/components/dm_panel/staging_approval.rs` | Add mood dropdown |
| `crates/player-ui/src/presentation/components/dm_panel/approval_popup.rs` | Editable dialogue, validation |
| `crates/player-ui/src/presentation/handlers/session_message_handler.rs` | Handle disposition/mood messages |
| `crates/player-ui/styles/input.css` | Mood badge, marker styling |

---

## System Documentation Updates

The following system docs need updating after implementation:

### 1. dialogue-system.md

**Updates needed**:
- Add section on "Expression Markers" explaining `*mood*` and `*mood|expression*` syntax
- Update "Context Categories" to distinguish disposition vs mood in NPC context
- Add US-DLG-017: Expression markers in dialogue
- Add US-DLG-018: Mood tag display in dialogue box
- Update DM Approval Popup mockup with marker validation UI
- Update Player Dialogue View mockup with expression changes

### 2. character-system.md

**Updates needed**:
- Add section on "Three-Tier Emotional Model" (Disposition, Mood, Expression)
- Clarify **Disposition vs Relationship** distinction:
  - RelationshipLevel = social distance (Stranger → Ally)
  - DispositionLevel = emotional stance (Hostile → Grateful)
  - Both stored on same edge, allowing combinations like "Suspicious Ally"
- Add `ExpressionConfig` to data model
- Add `default_mood` field documentation
- Update "Character Form" mockup with Expressions tab
- Add new user stories for expression configuration

### 3. npc-system.md

**Updates needed**:
- Clarify that NPC Mood Panel is now "NPC Disposition Panel"
- Add section explaining mood (NPC emotional state) vs disposition (NPC→PC relationship)
- Update implementation status for mood/disposition split

### 4. staging-system.md

**Updates needed**:
- Add "NPC Mood" section explaining per-NPC mood in staging
- Update `StagedNpc` data model with `mood` field
- Update "DM Staging Approval Popup" mockup with mood dropdowns
- Add mood to `ApprovedNpcInfo` in API section
- Explain mood persistence (staging cache until expiry)

### 5. New: expression-system.md (Optional)

Consider creating a dedicated system doc for the expression system:
- Expression marker syntax
- ExpressionConfig structure
- Expression sheet generation workflow
- Typewriter integration
- Player input validation

---

## Testing Strategy

### Unit Tests

1. **Dialogue Marker Parser** (domain)
   - Valid mood markers
   - Pipe format
   - Actions
   - Syntax errors

2. **ExpressionConfig** (domain)
   - Default expressions
   - `has_expression()` matching
   - `resolve_expression()` fallback

3. **DispositionLevel/MoodState** (domain)
   - Serialization
   - Display names
   - Default values

### Integration Tests

1. **Staging with mood** (engine-adapters)
   - Save staging with NPC mood
   - Load staging, verify mood populated

2. **Character with expression config** (engine-adapters)
   - Save character with expression_config
   - Load character, verify config populated

### Manual Testing

1. **Staging flow with mood**
   - Create staging, set NPC moods
   - Verify mood persists in cache
   - Verify LLM receives mood context

2. **Dialogue expression changes**
   - NPC dialogue with markers
   - Verify sprite changes during typewriter
   - Verify mood badge updates

3. **Disposition vs Mood distinction**
   - Set disposition (friendly)
   - Set mood (anxious)
   - Verify both appear correctly in LLM context and UI

---

## Rollback Plan

### Partial Rollback

1. **Expression markers optional**: Parse but treat as literal text
2. **Mood optional**: Fall back to disposition for LLM context
3. **Staging mood optional**: Use character.default_mood

### Full Rollback

1. Revert disposition rename (restore MoodLevel name)
2. Remove MoodState enum
3. Remove expression_config fields
4. Keep staging without mood field

### Data Safety

- All new fields use Default trait
- Neo4j schema changes are additive
- No breaking changes to existing data

---

## Progress Tracking

### Phase Status

| Phase | Description | Status | Hours | Notes |
|-------|-------------|--------|-------|-------|
| 0 | Disposition Rename Refactor | Not Started | 3-4h | Prerequisite |
| 1 | New Mood System (Domain & Protocol) | Not Started | 2-3h | |
| 2 | Staging System Mood Integration | Not Started | 2-3h | |
| 3 | Persistence & Repository Updates | Not Started | 2h | |
| 4 | LLM Prompt Updates | Not Started | 1.5h | |
| 5 | Expression Sheet Generation | Not Started | 3h | |
| 6 | Typewriter with Expressions | Not Started | 3-4h | |
| 7 | Character Sprite Updates | Not Started | 2h | |
| 8 | Player Input Validation | Not Started | 1.5h | |
| 9 | Expression Config Editor | Not Started | 3-4h | |
| 10 | Expression Sheet Gen UI | Not Started | 2-3h | |
| 11 | DM Approval Marker Support | Not Started | 2h | |
| 12 | Testing & Polish | Not Started | 2-3h | |

**Total Estimated**: 30-35 hours

### Completion Checklist

- [ ] Phase 0: Disposition rename complete
- [ ] Phase 1-4: Core system complete
- [ ] Phase 5-7: Expression visuals complete
- [ ] Phase 8-11: UI complete
- [ ] Phase 12: Testing complete
- [ ] All tests passing
- [ ] WASM build succeeds
- [ ] Desktop build succeeds
- [ ] Architecture check passes
- [ ] System documentation updated

### Change Log

| Date | Change |
|------|--------|
| 2025-12-27 | Added DirectorialContext cleanup: `NpcMotivationData.mood` → `emotional_guidance`, UI "Mood" → "Demeanor" |
| 2025-12-27 | Added UI consistency fixes to Phase 0 (npc_motivation.rs, npc_disposition_panel.rs) |
| 2025-12-27 | Clarified Disposition vs Relationship distinction (both stored on same edge) |
| 2025-12-27 | Revised plan with three-tier emotional model (Disposition/Mood/Expression) |
| 2025-12-27 | Added Phase 0 for disposition rename refactor |
| 2025-12-27 | Added staging system mood integration |
| 2025-12-27 | Added system documentation update requirements |
| 2025-12-27 | Initial plan created |

---

## Open Questions

1. **ComfyUI Workflow**: Need a workflow that generates expression grids. May need to create or find one.

2. **Expression Slicing**: Server-side slicing (individual PNGs) vs client-side (CSS sprite positioning). Current plan: server-side.

3. **Neo4j property names**: Should we rename stored "mood" properties to "disposition" in Neo4j, or keep for backward compatibility?

---

## Dependencies

### External
- ComfyUI server with expression sheet workflow
- Neo4j for persistence

### Internal
- Existing staging system
- Existing asset generation infrastructure
- Existing character form components
- Existing dialogue state management

---

## References

- [CONSOLIDATED_IMPLEMENTATION_PLAN.md](../progress/CONSOLIDATED_IMPLEMENTATION_PLAN.md) - P3.1 entry
- [dialogue-system.md](../systems/dialogue-system.md) - Dialogue system overview
- [staging-system.md](../systems/staging-system.md) - Staging system overview
- [character-system.md](../systems/character-system.md) - Character entity details
- [npc-system.md](../systems/npc-system.md) - NPC system overview
- [asset-system.md](../systems/asset-system.md) - Asset generation system
