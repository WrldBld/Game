# Sprint 4: UX Polish

**Created**: 2025-12-25  
**Status**: **COMPLETE**  
**Estimated Effort**: ~16 hours (2-3 sessions)  
**Actual Effort**: ~16 hours (3 sessions)

---

## Overview

Sprint 4 focuses on UX polish improvements for the Director/DM experience.

| Task | Effort | Status | Priority |
|------|--------|--------|----------|
| P2.4: Split Party Warning UX | 1h | **COMPLETE** | High |
| P2.2: Location Preview Modal | 2h | **COMPLETE** | High |
| P2.1: View-as-Character Mode | 4h | **COMPLETE** | High |
| P2.5: Style Reference for Asset Generation | 3h | **COMPLETE** | Medium |
| **PREREQ**: Remove session_id from Story Events | 3.5h | **COMPLETE** | High |
| P2.3: Story Arc Visual Timeline | 6h | **COMPLETE** | Medium |

**Note**: P1.1 Phases 5-6 (Region Items LLM Context + UI) were discovered to be **already implemented**.

---

## Task 1: P2.4 Split Party Warning UX (1h)

### Goal
Show persistent banner in Director view when party is split across locations.

### Current State
- `SplitPartyLocation` struct exists in protocol
- `ServerMessage::SplitPartyNotification` handled in message handler (logs only)
- `PCLocationsWidget` shows warning in PC Management modal (not visible in main view)

### UI Mockup

```
┌─────────────────────────────────────────────────────────────────────┐
│ ⚠️ Party Split Across 3 Locations                              [▼] │
├─────────────────────────────────────────────────────────────────────┤
│  📍 Tavern: Aldric, Mira                                            │
│  📍 Market Square: Theron                                           │
│  📍 Castle Gates: Lyra, Kael                                        │
└─────────────────────────────────────────────────────────────────────┘
```

- Amber/yellow warning color
- Collapsible (starts expanded, can collapse to just header)
- Shows location name + PC names at each location
- Appears at top of Director panel

### Implementation

1. **Add state to GameState** (`game_state.rs`)
   ```rust
   pub split_party_locations: Signal<Vec<SplitPartyLocation>>,
   ```

2. **Update message handler** (`session_message_handler.rs`)
   ```rust
   ServerMessage::SplitPartyNotification { locations } => {
       game_state.split_party_locations.set(locations);
   }
   ```

3. **Create SplitPartyBanner component** (NEW file)
   - Props: `locations: Vec<SplitPartyLocation>`
   - Collapsible state
   - Location list with PC names

4. **Add to Director view** (`director/content.rs`)
   - Render at top when `split_party_locations` is non-empty

### Files
- `crates/player-ui/src/presentation/state/game_state.rs`
- `crates/player-ui/src/presentation/handlers/session_message_handler.rs`
- NEW: `crates/player-ui/src/presentation/components/dm_panel/split_party_banner.rs`
- `crates/player-ui/src/presentation/views/director/content.rs`
- `crates/player-ui/src/presentation/components/dm_panel/mod.rs`

---

## Task 2: P2.2 Location Preview Modal (2h)

### Goal
Show location details when clicking preview button in LocationNavigator.

### Current State
- `LocationNavigator` has `on_preview: EventHandler<String>` callback
- Preview button exists but handler shows TODO comment
- `LocationService.get_location()` available

### UI Mockup

```
┌─────────────────────────────────────────────────────────────────────┐
│ Location Preview                                                [×] │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                     [Backdrop Image]                         │   │
│  │                        (if any)                              │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  🏰 The Golden Dragon Tavern                                        │
│  Type: Tavern                                                       │
│                                                                     │
│  A warm and welcoming establishment known for its fine ales         │
│  and traveling bards. The hearth burns bright even in winter.       │
│                                                                     │
│  ─────────────────────────────────────────────────────────────────  │
│                                                                     │
│  📍 Regions                                                         │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ Main Hall        The central gathering area with tables     │   │
│  │ Private Rooms    Upstairs chambers for rent                 │   │
│  │ Kitchen          Behind the bar, off-limits to patrons      │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  🔗 Connections                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ → Market Square (exit through front door)                   │   │
│  │ → Back Alley (exit through kitchen)                         │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  👥 NPCs Present                                                    │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ Barkeep Marta    Owner    Main Hall                         │   │
│  │ Traveling Bard   Visitor  Main Hall                         │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Implementation

1. **Create LocationPreviewModal component** (NEW file)
   - Fetch location data via `LocationService`
   - Display backdrop, name, type, description
   - List regions with descriptions
   - Show connections to other locations
   - List NPCs currently present

2. **Wire up in Director view**
   - Add `preview_location_id: Signal<Option<String>>`
   - Handle `on_preview` callback
   - Render modal when ID is set

### Files
- NEW: `crates/player-ui/src/presentation/components/dm_panel/location_preview_modal.rs`
- `crates/player-ui/src/presentation/views/director/content.rs`
- `crates/player-ui/src/presentation/components/dm_panel/mod.rs`

---

## Task 3: P2.1 View-as-Character Mode (4h)

### Goal
DM can preview scene from a specific PC's perspective (read-only).

### Current State
- `show_character_perspective` signal exists
- `CharacterPerspectiveViewer` component placeholder exists
- `PCManagementPanel` has "View as" buttons
- `on_view_as_character` callback receives character_id

### UI Mockup

**Director View with "View as" button:**
```
┌─────────────────────────────────────────────────────────────────────┐
│ PC Management                                                       │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐ │
│  │ [Avatar] Aldric the Bold                                      │ │
│  │          Fighter • Level 5 • Tavern - Main Hall               │ │
│  │          [Select] [View As 👁]                                │ │
│  └───────────────────────────────────────────────────────────────┘ │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

**View-as-Character Mode:**
```
┌─────────────────────────────────────────────────────────────────────┐
│ 👁 Viewing as: Aldric the Bold (Read-only)              [Exit View] │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────────────────────────────────┬───────────────────┐   │
│  │                                         │                   │   │
│  │           [Scene Preview]               │  Location:        │   │
│  │                                         │  Tavern           │   │
│  │    What Aldric sees in the scene        │  Main Hall        │   │
│  │                                         │                   │   │
│  │                                         │  NPCs Visible:    │   │
│  │                                         │  • Barkeep Marta  │   │
│  │                                         │  • Traveling Bard │   │
│  │                                         │                   │   │
│  ├─────────────────────────────────────────┤  Items:           │   │
│  │                                         │  • Mug of Ale     │   │
│  │  [Conversation Log - Read Only]         │                   │   │
│  │                                         │  ─────────────    │   │
│  │  Aldric: "What news from the road?"     │                   │   │
│  │  Bard: "Dark tidings, friend..."        │  [Actions         │   │
│  │                                         │   Disabled]       │   │
│  │                                         │                   │   │
│  └─────────────────────────────────────────┴───────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

- Purple/blue banner at top indicating view mode
- All action buttons disabled/hidden
- Shows filtered view (only what that PC can see)
- "Exit View" button returns to Director mode

### Implementation

1. **Add view mode state** (`game_state.rs`)
   ```rust
   pub enum ViewMode {
       Director,
       ViewingAsCharacter { character_id: String, character_name: String },
   }
   
   pub view_mode: Signal<ViewMode>,
   ```

2. **Implement CharacterPerspectiveViewer**
   - Reuse PC view layout
   - Filter NPCs/items to what PC sees
   - Disable all action buttons
   - Add view mode banner with exit button

3. **Wire up view mode switching**
   - "View as" button sets `view_mode`
   - Director renders `CharacterPerspectiveViewer` when in view mode
   - Exit button resets to `Director`

### Files
- `crates/player-ui/src/presentation/state/game_state.rs`
- `crates/player-ui/src/presentation/views/director/content.rs`
- `crates/player-ui/src/presentation/components/dm_panel/pc_management.rs`

---

## Task 4: P2.5 Style Reference for Asset Generation (3h)

### Goal
"Use as Style Reference" button sets a world-wide default style for asset generation, persisted across sessions.

### Current State
- `AssetThumbnail` has button but handler is None
- `GenerateAssetModal` has `style_reference_id` signal and selector UI
- Style reference not persisted

### UI Mockup

**Asset Gallery with Style Reference:**
```
┌─────────────────────────────────────────────────────────────────────┐
│ Asset Gallery                                    [Style: Portrait1] │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐               │
│  │ ★       │  │         │  │         │  │         │               │
│  │  IMG1   │  │  IMG2   │  │  IMG3   │  │  IMG4   │               │
│  │         │  │         │  │         │  │         │               │
│  ├─────────┤  ├─────────┤  ├─────────┤  ├─────────┤               │
│  │Portrait1│  │Portrait2│  │Backdrop │  │Sprite1  │               │
│  │ [Menu▼] │  │ [Menu▼] │  │ [Menu▼] │  │ [Menu▼] │               │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘               │
│                                                                     │
│  ★ = Current style reference                                       │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘

Context Menu:
┌─────────────────────┐
│ View Full Size      │
│ Download            │
│ ─────────────────── │
│ ★ Use as Style Ref  │  ← Sets this as world default
│ ─────────────────── │
│ Delete              │
└─────────────────────┘
```

**Generate Asset Modal with Style:**
```
┌─────────────────────────────────────────────────────────────────────┐
│ Generate Portrait                                               [×] │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Character: [Aldric the Bold        ▼]                              │
│                                                                     │
│  Prompt: [A noble warrior with weathered features...             ]  │
│                                                                     │
│  Style Reference: [Portrait1 (World Default)        ▼] [Clear]     │
│                   Using world style reference                       │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              [Style Reference Preview]                       │   │
│  │                    (thumbnail)                               │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│                                              [Cancel]  [Generate]   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Implementation

1. **Add style reference to world settings** (backend)
   - Add `style_reference_asset_id: Option<String>` to world settings
   - API endpoint to get/set world style reference

2. **Add state for style reference** (UI)
   - `world_style_reference: Signal<Option<StyleReference>>`
   - Load from world settings on session start

3. **Wire AssetThumbnail callback**
   - "Use as Style Reference" updates world settings via API
   - Show star/indicator on current reference asset
   - Toast notification on change

4. **Apply in GenerateAssetModal**
   - Pre-populate style reference from world default
   - Show "(World Default)" indicator
   - Allow override per-generation

### Files
- `crates/engine-adapters/src/infrastructure/http/settings_routes.rs` (add style_reference)
- `crates/player-ui/src/presentation/state/` (style state)
- `crates/player-ui/src/presentation/components/creator/asset_gallery.rs`
- `crates/player-ui/src/presentation/components/creator/generate_asset_modal.rs`
- `crates/player-app/src/application/services/settings_service.rs`

---

## Task 5: P2.3 Story Arc Visual Timeline (6h)

### Goal
Create a horizontal, zoomable/pannable visual timeline of story events as a new sub-tab.

### Current State
- `TimelineView` exists as a vertical list with filtering
- `StoryArcSubTab` enum has: Timeline, NarrativeEvents, EventChains
- Need to add "Visual" sub-tab with horizontal timeline

### UI Mockup

**Sub-tab Navigation:**
```
┌─────────────────────────────────────────────────────────────────────┐
│ Story Arc                                                           │
├─────────────────────────────────────────────────────────────────────┤
│ [Timeline] [Visual] [Narrative Events] [Event Chains]              │
│            ~~~~~~~~                                                 │
└─────────────────────────────────────────────────────────────────────┘
```

**Visual Timeline View:**
```
┌─────────────────────────────────────────────────────────────────────┐
│ Visual Timeline                                    [−] [100%] [+]   │
│                                                    Zoom: ◀ ●──── ▶  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ◀ Pan                                                      Pan ▶   │
│                                                                     │
│  │ Session 1                    │ Session 2                         │
│  │ Dec 20                       │ Dec 22                            │
│  ▼                              ▼                                   │
│  ════════════════════════════════════════════════════════════════   │
│                                                                     │
│      ●──────●────●─────────●────────────●──────●────────────●──▶   │
│      │      │    │         │            │      │            │       │
│      │      │    │         │            │      │            │       │
│     ┌┴┐    ┌┴┐  ┌┴┐       ┌┴┐          ┌┴┐    ┌┴┐          ┌┴┐     │
│     │▶│    │💬│  │🎲│       │🚶│          │⚔│    │💡│          │📝│     │
│     └─┘    └─┘  └─┘       └─┘          └─┘    └─┘          └─┘     │
│   Session  Talk  Roll    Move       Combat   Info        DM        │
│   Start    NPC   Fail    Location            Reveal      Marker    │
│                                                                     │
│  ════════════════════════════════════════════════════════════════   │
│                         │                                           │
│                         ▲                                           │
│                      Current                                        │
│                      Position                                       │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│ Hover: 🎲 Challenge - "Pick the lock" - Failed (Roll: 8)           │
│        Dec 20, 14:32 • Aldric                                       │
└─────────────────────────────────────────────────────────────────────┘
```

**Event Node Detail (on click):**
```
┌─────────────────────────────────────────────────────────────────────┐
│ Challenge Attempted                                             [×] │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  🎲 Pick the lock                                                   │
│                                                                     │
│  Dec 20, 14:32 (Game Time: Midday)                                 │
│                                                                     │
│  Aldric attempted to pick the lock on the chest.                    │
│  The mechanism proved too complex for his skills.                   │
│                                                                     │
│  ─────────────────────────────────────────────────────────────────  │
│                                                                     │
│  Skill: Lockpicking                                                 │
│  Roll: 🎲 8                                                         │
│  Outcome: Failure                                                   │
│                                                                     │
│  Tags: #chest #lockpicking #aldric                                  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Features
- Horizontal time axis with session/date markers
- Event nodes with type-specific icons (reuse existing colors)
- Zoom: slider + buttons to change time scale
- Pan: drag or arrow buttons to scroll timeline
- Current position marker (if in active session)
- Hover tooltip with event summary
- Click to open full detail modal (reuse `EventDetailModal`)
- Cluster nearby events to prevent overlap

### Implementation

1. **Add Visual sub-tab** (`story_arc/mod.rs`)
   ```rust
   pub enum StoryArcSubTab {
       Timeline,
       Visual,  // NEW
       NarrativeEvents,
       EventChains,
   }
   ```

2. **Create VisualTimeline component** (NEW file)
   - Horizontal scrollable container
   - Time axis with markers
   - Event nodes positioned by timestamp
   - Zoom/pan state and controls

3. **Timeline data processing**
   - Group events by session/date for markers
   - Calculate node positions based on zoom level
   - Handle clustering for dense event periods

4. **Interactivity**
   - Hover state for tooltips
   - Click handler for detail modal
   - Keyboard shortcuts (+/- for zoom, arrows for pan)

### Files
- `crates/player-ui/src/presentation/views/story_arc/mod.rs` (add Visual variant)
- `crates/player-ui/src/presentation/views/story_arc/content.rs` (render Visual tab)
- NEW: `crates/player-ui/src/presentation/components/story_arc/visual_timeline.rs`
- `crates/player-ui/src/presentation/components/story_arc/mod.rs`

---

## Session Plan

### Session 1 (~6-7h): Quick Wins + View Mode
1. P2.4: Split Party Warning (1h)
2. P2.2: Location Preview Modal (2h)
3. P2.1: View-as-Character Mode (4h)

### Session 2 (~6h): Style Reference + Timeline Start
1. P2.5: Style Reference (3h)
2. P2.3: Visual Timeline (3h) - Component + basic rendering

### Session 3 (~3h): Timeline Completion
1. P2.3: Visual Timeline (3h) - Zoom/pan, polish, integration

---

## Discoveries

### P1.1 Phases 5-6: Already Implemented!

During planning, discovered that Region Items LLM Context (Phase 5) and UI (Phase 6) are **already fully implemented**:

**Phase 5 (LLM Context):**
- `RegionItemContext` value object exists
- `SceneContext.region_items` populated during prompt building
- Prompt includes "VISIBLE ITEMS IN AREA:" section

**Phase 6 (Player UI):**
- `RegionItemData` in protocol
- `SceneChanged` message includes `region_items`
- `GameState.region_items` signal exists
- `RegionItemsPanel` component with pickup buttons
- `ActionPanel` shows items count badge
- Full pickup flow via WebSocket

**Action**: Update `US-REGION-ITEMS.md` to mark Phases 5-6 as complete.

---

## Progress Log

| Date | Task | Status | Notes |
|------|------|--------|-------|
| 2025-12-25 | Sprint 4 Planning | Complete | Created this document |
| 2025-12-25 | P2.4: Split Party Warning | **COMPLETE** | Added state to GameState, SplitPartyBanner component, wired to Director |
| 2025-12-25 | P2.2: Location Preview Modal | **COMPLETE** | LocationPreviewModal with regions, connections; wired to LocationNavigator |
| 2025-12-25 | P2.1: View-as-Character Mode | **COMPLETE** | ViewMode enum, ViewAsCharacterMode component, read-only perspective view |
| 2025-12-25 | P2.5: Style Reference | **COMPLETE** | Backend + UI persistence, world default style reference |
| 2025-12-25 | Session ID Refactor | **COMPLETE** | Remove session_id from story events (prerequisite for Visual Timeline) |
| 2025-12-25 | P2.3: Visual Timeline | **COMPLETE** | Horizontal zoomable timeline with clustering, filters, zoom/pan |

### Session 1 Summary (2025-12-25)

**Completed 3 high-priority tasks:**

1. **P2.4: Split Party Warning UX**
   - Added `split_party_locations: Signal<Vec<SplitPartyLocation>>` to GameState
   - Updated message handler to populate state from `SplitPartyNotification`
   - Created `SplitPartyBanner` component with collapsible location list
   - Wired banner to Director view (top of left panel)

2. **P2.2: Location Preview Modal**
   - Created `LocationPreviewModal` component
   - Fetches full location data via LocationService
   - Displays: name, type, description, backdrop, regions, connections, hidden secrets
   - Wired `on_preview` callback in LocationNavigator to open modal

3. **P2.1: View-as-Character Mode**
   - Added `ViewMode` enum (Director | ViewingAsCharacter) to GameState
   - Added helper methods: `start_viewing_as()`, `stop_viewing_as()`, `is_viewing_as_character()`
   - Updated `CharacterPerspectiveViewer` to pass `ViewAsData` with ID and name
   - Created `ViewAsCharacterMode` component showing read-only perspective
   - Blue banner with "Exit View" button, shows NPCs/items visible to character

**Files Created:**
- `crates/player-ui/src/presentation/components/dm_panel/split_party_banner.rs`
- `crates/player-ui/src/presentation/components/dm_panel/location_preview_modal.rs`

**Files Modified:**
- `crates/player-ui/src/presentation/state/game_state.rs` (ViewMode, split party state)
- `crates/player-ui/src/presentation/state/mod.rs` (export ViewMode)
- `crates/player-ui/src/presentation/handlers/session_message_handler.rs`
- `crates/player-ui/src/presentation/views/director/content.rs` (all features wired)
- `crates/player-ui/src/presentation/components/dm_panel/character_perspective.rs` (ViewAsData)
- `crates/player-ui/src/presentation/components/dm_panel/mod.rs`

**Remaining for Sprint 4:**
- ~~P2.5: Style Reference (3h) - Backend + UI persistence~~ **COMPLETE**
- ~~Session ID Refactor (3.5h) - Remove session scoping from story events~~ **COMPLETE**
- ~~P2.3: Visual Timeline (6h) - New sub-tab with horizontal zoomable timeline~~ **COMPLETE**

**Sprint 4 is now COMPLETE!**

### Session 2 Summary (2025-12-25)

**Completed P2.5: Style Reference for Asset Generation**

1. **Backend (Domain)**
   - Added `style_reference_asset_id: Option<String>` to `AppSettings`
   - Added to `Default` impl and `from_env()`
   - Added metadata entry for UI rendering (category: "Assets")

2. **Frontend DTO**
   - Mirrored `style_reference_asset_id: Option<String>` in player-app DTO

3. **Asset Gallery Wiring**
   - Wired `on_use_as_reference` callback to save to world settings
   - Added purple border + star indicator for current world style reference
   - Pre-populates `GenerateAssetModal` with world default style reference
   - Shows "(World Default)" indicator when using persisted reference
   - Added "Change" and "Clear" buttons for style reference

4. **Director Generate Modal**
   - Same improvements: loads world default, shows indicator, Change/Clear buttons

**Files Created:** None (all modifications)

**Files Modified:**
- `crates/domain/src/value_objects/settings.rs`
- `crates/player-app/src/application/dto/settings.rs`
- `crates/player-ui/src/presentation/components/creator/asset_gallery.rs`
- `crates/player-ui/src/presentation/components/dm_panel/director_generate_modal.rs`

---

## Prerequisite: Remove session_id from Story Events

### Background

Story events were incorrectly scoped to sessions (a bad design decision where worlds might be recyclable - they aren't). Story events should be **world-scoped only**.

**Analysis revealed:**
- Domain entity `StoryEvent` already has `world_id` as required field
- `session_id` is stored as a graph edge (`OCCURRED_IN_SESSION`)
- Backend already supports world-scoped queries (`list_by_world_paginated`)
- Frontend DTO has `session_id: String` but backend returns `Option<String>` - **mismatch bug**

### What We're Removing

Remove `session_id` from story events only. Keep it for:
- `PlayerCharacter.session_id` - Still needed (binds PC to active game)
- `SessionState` / connection management - Still needed
- Queue items - Still needed (routes messages to correct session)

### Implementation Plan

See: `docs/plans/REFACTOR-STORY-EVENT-SESSION-ID.md`

---

## Task 5: P2.3 Story Arc Visual Timeline (6h) - UPDATED

### Prerequisite
Complete the session_id refactor first to ensure clean world-scoped queries.

### Goal
Create a horizontal, zoomable/pannable visual timeline of story events as a new sub-tab.

### Design Decisions (from discussion)

1. **Clustering**: Stack vertically up to 3 events, then "+N more" with expandability on click
2. **Zoom range**: 0.25x to 4.0x
3. **Keyboard navigation**: Arrow keys for pan, +/- for zoom
4. **Filters**: Support same filters as vertical timeline; filtered-out events shown greyed and clustered together regardless of time distance

### Implementation Phases

#### Phase 1: Add "Visual" Sub-tab (~30 min)

**Files:**
- `crates/player-ui/src/presentation/views/story_arc/mod.rs`
- `crates/player-ui/src/presentation/views/story_arc/content.rs`

**Changes:**
1. Add `Visual` variant to `StoryArcSubTab`
2. Update `from_str()` to handle `"visual"` → `Visual`
3. Add tab link with icon "📊"
4. Add match arm to render `VisualTimeline` component

#### Phase 2: Create VisualTimeline Component (~2.5 hours)

**New file:** `crates/player-ui/src/presentation/components/story_arc/visual_timeline.rs`

**State:**
```rust
let mut zoom_level: Signal<f32> = use_signal(|| 1.0);  // 0.25 - 4.0
let mut scroll_offset: Signal<f32> = use_signal(|| 0.0);
let mut hovered_event: Signal<Option<StoryEventData>> = use_signal(|| None);
let mut selected_event: Signal<Option<StoryEventData>> = use_signal(|| None);
let mut expanded_cluster: Signal<Option<usize>> = use_signal(|| None);
let mut filters = use_signal(TimelineFilterState::default);
```

**Structure:**
```
┌─────────────────────────────────────────────────────────────┐
│ Header: Zoom Controls + Filter Toggle                       │
├─────────────────────────────────────────────────────────────┤
│ Filter Panel (collapsible, reuse TimelineFilters)           │
├─────────────────────────────────────────────────────────────┤
│ [◀]  ┌─────────────────────────────────────────────┐  [▶]  │
│      │  Date markers at top                         │       │
│      │  ════════════════════════════════════════    │       │
│      │  Event nodes (filtered shown in color,       │       │
│      │  non-matching greyed and clustered)          │       │
│      │  ════════════════════════════════════════    │       │
│      └─────────────────────────────────────────────┘       │
├─────────────────────────────────────────────────────────────┤
│ Tooltip/Info Bar (shows on hover)                           │
└─────────────────────────────────────────────────────────────┘
```

#### Phase 3: Time Axis & Date Markers (~45 min)

- Parse timestamps to position events horizontally
- Group events by date for markers (not session - sessions removed)
- Show date markers at top of timeline
- Render time axis with tick marks based on zoom level

#### Phase 4: Event Nodes & Clustering (~1 hour)

**Node types:**
1. **Visible nodes**: Events matching current filter (full color, clickable)
2. **Filtered-out nodes**: Events not matching filter (greyed, clustered, show count)

**Clustering logic:**
```rust
struct TimelineCluster {
    events: Vec<StoryEventData>,
    x: f32,
    is_filtered_out: bool,
    is_expanded: bool,
}
```

- Events within 20px get clustered
- Visible events: stack up to 3, then "+N more" (expandable on click)
- Filtered-out events: always cluster with count badge, regardless of time distance

#### Phase 5: Interactivity (~1 hour)

**Zoom:**
- [-] / [+] buttons (step 0.25)
- Reset button
- Range: 0.25x to 4.0x

**Pan/scroll:**
- [◀] / [▶] buttons
- Drag to pan
- Wheel for horizontal scroll
- Keyboard: Left/Right arrows

**Hover:**
- Show tooltip with event summary, timestamp, characters

**Click:**
- Open `EventDetailModal` (reuse from `timeline_view.rs`)
- For "+N more" cluster: expand to show all events

**Keyboard:**
- Arrow keys: pan left/right
- +/-: zoom in/out
- Escape: close expanded cluster or modal

### Files Summary

| File | Action | Description |
|------|--------|-------------|
| `views/story_arc/mod.rs` | Modify | Add `Visual` variant |
| `views/story_arc/content.rs` | Modify | Add tab link + render arm |
| `components/story_arc/mod.rs` | Modify | Export `visual_timeline` |
| **NEW** `components/story_arc/visual_timeline.rs` | Create | Main component (~500 lines) |
| `components/story_arc/timeline_view.rs` | Modify | Make helpers public for reuse |

---

### Session 3 Summary (2025-12-25)

**Completed Session ID Refactor + P2.3 Visual Timeline**

#### 1. Session ID Refactor (Prerequisite)

Removed `session_id` from story events across the entire stack:

**Backend Changes:**
- `crates/engine-app/src/application/services/story_event_service.rs` - Removed `session_id` param from all `record_*` methods
- `crates/engine-app/src/application/dto/story_event.rs` - Removed `session_id` from DTOs
- `crates/engine-ports/src/outbound/repository_port.rs` - Removed `list_by_session`, `set_session`, `get_session`
- `crates/engine-adapters/src/infrastructure/persistence/story_event_repository.rs` - Removed session edge methods
- `crates/engine-adapters/src/infrastructure/http/story_event_routes.rs` - Removed session_id from `create_dm_marker`
- `crates/engine-app/src/application/services/dm_approval_queue_service.rs` - Updated `record_dialogue_exchange` call
- `crates/engine-app/src/application/services/narrative_event_approval_service.rs` - Updated `record_narrative_event_triggered` call

**Frontend Changes:**
- `crates/player-app/src/application/dto/world_snapshot.rs` - Removed `session_id` from `StoryEventData`
- `crates/player-app/src/application/services/story_event_service.rs` - Removed `session_id` from `list_story_events` and `create_dm_marker`
- `crates/player-ui/src/presentation/components/story_arc/timeline_view.rs` - Removed `session_id` from props and service calls
- `crates/player-ui/src/presentation/components/story_arc/add_dm_marker.rs` - Removed `session_id` from props and service calls

#### 2. P2.3 Visual Timeline

Created a new horizontal zoomable/pannable timeline view:

**Features Implemented:**
- New "Visual" sub-tab in Story Arc mode (icon: 📊)
- Horizontal timeline with date markers at top
- Event nodes positioned by timestamp with type-specific colors and icons
- Clustering: events within 30px are grouped, stacking up to 3 events before "+N more"
- Collapsible filter panel (reuses existing TimelineFilters component)
- Zoom controls: -/+ buttons (0.25x to 4.0x range) with reset button
- Pan controls: ◀/▶ buttons for horizontal scrolling
- Hover tooltip showing event summary, timestamp, character count
- Click to open event detail modal
- Filtered-out events shown greyed with opacity 40%

**Files Created:**
- `crates/player-ui/src/presentation/components/story_arc/visual_timeline.rs` (~700 lines)

**Files Modified:**
- `crates/player-ui/src/presentation/views/story_arc/mod.rs` - Added `Visual` variant to `StoryArcSubTab`
- `crates/player-ui/src/presentation/views/story_arc/content.rs` - Added tab link and render arm
- `crates/player-ui/src/presentation/components/story_arc/mod.rs` - Exported `visual_timeline` module

---

## Sprint 4 Complete!

All 6 tasks completed:
1. P2.4: Split Party Warning UX
2. P2.2: Location Preview Modal
3. P2.1: View-as-Character Mode
4. P2.5: Style Reference for Asset Generation
5. Session ID Refactor (prerequisite)
6. P2.3: Visual Timeline
