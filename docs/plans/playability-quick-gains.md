# Playability Quick Gains Plan

## Overview

This plan targets the smallest changes that materially improve the DM/Player loop and move the project toward a playable demo. It focuses on time feedback, conversation lifecycle control, staging settings, and prompt template overrides.

**Scope:** UI wiring, small orchestration updates, and configuration surfaces.

**Note:** US-TIME-010 (Set exact game time) and US-TIME-012 (Time mode toggle) are already implemented in the backend/protocol. UI wiring remains.

---

## Ordered Delivery

1. **US-DLG-017** — Player End Conversation (P0)
2. **US-TIME-013** — Dialogue Approval Emits Time Suggestion (P0)
3. **US-TIME-011** — Player Time-Advance Toast (P1)
4. **US-STG-016** — World Staging Auto-Approve Timeout UI (P1)
5. **US-STG-017** — Location Staging Defaults (TTL + LLM Presence) (P1)
6. **US-DLG-010** — Dialogue Response Template Overrides UI (P2)

---

## User Stories

### US-DLG-017 — Player End Conversation (P0)

**Story:** As a player, I can end a conversation so that I can return to exploration without a hanging dialogue state.

**Acceptance Criteria**
- Player action sends an end-conversation message to the server.
- Server marks the conversation inactive and broadcasts the end to relevant clients.
- Player UI clears dialogue choices and returns to non-dialogue view within one message cycle.
- DM UI no longer shows active approval flow for the ended conversation.
- Tests cover the end-conversation use case and message handling.

**Dependencies:** Dialogue state management; WebSocket messages for end events.

**Likely Files**
- `crates/engine/src/use_cases/conversation/end.rs`
- `crates/engine/src/api/websocket/mod.rs`
- `crates/shared/src/messages.rs`
- `crates/shared/src/requests/conversation.rs`
- `crates/player/src/ui/presentation/components/action_panel.rs`
- `crates/player/src/ui/presentation/handlers/session_message_handler.rs`

---

### US-TIME-013 — Dialogue Approval Emits Time Suggestion (P0)

**Story:** As a system, dialogue approvals emit time suggestions so that conversations can advance time with DM oversight.

**Acceptance Criteria**
- On dialogue approval, the system emits a `TimeSuggestion` when `TimeMode::Suggested`.
- Suggestions include a reason tied to the dialogue action.
- No suggestion is emitted when `TimeMode::Manual`.
- Tests verify suggested mode vs manual mode behavior.

**Dependencies:** Time suggestion use case; approval flow; time config.

**Likely Files**
- `crates/engine/src/use_cases/queues/mod.rs`
- `crates/engine/src/use_cases/time/mod.rs`
- `crates/engine/src/api/websocket/mod.rs`
- `crates/domain/src/game_time.rs`

---

### US-TIME-011 — Player Time-Advance Toast (P1)

**Story:** As a player, I see a time-advance toast so that I notice time progression.

**Acceptance Criteria**
- Player receives `GameTimeAdvanced` and shows a toast with reason + time delta.
- Toast auto-dismisses and does not block input.
- Multiple advances queue or replace gracefully.
- Uses world time format settings.

**Dependencies:** `GameTimeAdvanced` payload handling in player state.

**Likely Files**
- `crates/player/src/ui/presentation/views/pc_view.rs`
- `crates/player/src/ui/presentation/components/navigation_panel.rs`
- `crates/player/src/ui/presentation/state/game_state.rs`
- `crates/player/src/ui/presentation/handlers/session_message_handler.rs`

---

### US-STG-016 — World Staging Auto-Approve Timeout UI (P1)

**Story:** As a DM, I can configure auto-approve timeout per world so that staging doesn’t block play.

**Acceptance Criteria**
- World settings UI shows `staging_timeout_seconds` and `auto_approve_on_timeout`.
- Saving persists values and reload shows updated values.
- Tooltips explain staging timeout behavior.

**Dependencies:** World settings UI.

**Likely Files**
- `crates/player/src/ui/presentation/components/creator/world_form.rs`
- `crates/player/src/application/services/world_service.rs`

---

### US-STG-017 — Location Staging Defaults (TTL + LLM Presence) (P1)

**Story:** As a DM, I can set location staging defaults (TTL + LLM presence toggle) so staging behavior matches the location.

**Acceptance Criteria**
- Location settings expose `presence_cache_ttl_hours` and `use_llm_presence`.
- Values persist and are reflected on reload.
- Staging approval uses location defaults when present.

**Dependencies:** Location settings UI and DTOs.

**Likely Files**
- `crates/player/src/ui/presentation/components/creator/location_form.rs`
- `crates/player/src/application/services/location_service.rs`
- `crates/engine/src/infrastructure/neo4j/location_repo.rs`
- `crates/domain/src/aggregates/location.rs`

---

### US-DLG-010 — Dialogue Response Template Overrides UI (P2)

**Story:** As a DM, I can customize the LLM response format through a template overrides UI so dialogue matches my world tone.

**Acceptance Criteria**
- UI lists `dialogue.response_format` with resolved value.
- DM can save a world override; resolve endpoint returns the override.
- Dialogue generation uses the resolved template.

**Dependencies:** Prompt Template System APIs.

**Likely Files**
- `crates/player/src/ui/presentation/components/settings/*`
- `crates/player/src/application/services/*`
- `crates/engine/src/use_cases/queues/mod.rs`
- `crates/domain/src/value_objects/prompt_templates.rs`

---

## UI Mockups

### Player Time-Advance Toast

```
                    ┌────────────────────────────────────────┐
                    │  ⏰ Time Advanced                      │
                    │                                        │
                    │  Resting at camp                       │
                    │  +8 hours                              │
                    │                                        │
                    │  Current time: Day 3, 14:00            │
                    │                                        │
                    │  [Dismiss]                             │
                    └────────────────────────────────────────┘

   ┌─────────────────────────────────────────────────────────────────────────┐
   │                                                                         │
   │                            [Backdrop Scene]                            │
   │                                                                         │
   │                       [Character Sprites]                              │
   │                                                                         │
   │   ┌─────────────────────────────────────────────────────────────────┐ │
   │   │ 🗣️ "Alright, everyone get some rest. We move at dawn."          │ │
   │   │ [▌]                                                              │ │
   │   └─────────────────────────────────────────────────────────────────┘ │
   └─────────────────────────────────────────────────────────────────────────┘

┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐
│ Talk       │ │ Examine    │ │ Travel     │ │ Character  │
│ [NPC]      │ │ [Camp]     │ │ [Exit]     │ │ [Sheet]    │
└────────────┘ └────────────┘ └────────────┘ └────────────┘
```

### DM Time Control Additions (Set Time + Mode Toggle)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Session: The Lost Mine                     Players: 3/4    🕐 Day 2, 08:15 │
│                                                            [🔄 Auto ▼]    │
├─────────────────────────────────────────────────────────────────────────────┤
│  [⚙️ Settings]  [📋 NPCs]  [📍 Locations]  [🎲 Challenges]  [+ New Item]   │
└─────────────────────────────────────────────────────────────────────────────┘

                             ┌─────────────┐
                             │ 🔄 Auto     │
                             │ ⏹️  Manual   │
                             └─────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│  Set Game Time                                                      [X]     │
├─────────────────────────────────────────────────────────────────────────────┤
│  Current: Day 2, 08:15                                                     │
│  Day:   [2]   Hour: [08]   Minute: [15]                                   │
│  Quick: [Dawn 06:00] [Noon 12:00] [Dusk 18:00] [Midnight 00:00]            │
│  Reason: [After resting at the inn...]                                    │
│  [Set Time] [Cancel]                                                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Player End Conversation Button

```
┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────────────┐
│ Continue   │ │ Talk       │ │ Examine    │ │ End Conversation   │
│ [Dialogue] │ │ [Other]    │ │ [Marcus]   │ │ [× Marcus]         │
└────────────┘ └────────────┘ └────────────┘ └────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│  End Conversation?                                                          │
│  End conversation with Marcus?                                             │
│  [Yes, End It]  [Keep Talking]                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### World Settings — Staging Auto-Approve

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Settings                                                           [X]     │
├─────────────────────────────────────────────────────────────────────────────┤
│  [General]  [Staging]  [Prompts]  [Notifications]                           │
│  Auto-Approve Staging  [✓] Enabled                                          │
│  Auto-Approve Timeout  [30] seconds  [5] [15] [30] [60]                     │
│  [Save Changes] [Reset to Defaults]                                         │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Location Settings — Presence Defaults

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Location: Rusty Anchor Tavern                                     [X]     │
├─────────────────────────────────────────────────────────────────────────────┤
│  [General]  [Presence]  [Events]                                           │
│  Presence Cache TTL [15] minutes  [5] [10] [15] [30]                        │
│  Use LLM Presence Detection [✓] Enabled                                     │
│  [Save Changes] [Reset to Defaults]                                         │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Prompt Template Editor (Dialogue Response Format)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Prompt Templates                                                 [+ New]   │
├─────────────────────────────────────────────────────────────────────────────┤
│  [🔍 Search...]  [Category: Dialogue ▼]                                     │
│  dialogue.response_format                                  [Edit]          │
│  Response format instructions for NPC dialogue                             │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│  dialogue.response_format                                           [← Back]│
├─────────────────────────────────────────────────────────────────────────────┤
│  Template Content                                                        │
│  You are an RPG storyteller...                                           │
│  [Save] [Save & Activate]                                                │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Risks

- Dialogue end flow must reconcile with pending approvals and queued LLM responses.
- Time suggestions must avoid double-counting if existing handlers already advance time.
- Prompt template UI depends on stable template resolution endpoints.

---

## Validation Checklist

- End-to-end: Player → End Conversation → Server → UI reset.
- Dialogue approval → TimeSuggestion → DM decision → GameTimeAdvanced → Player toast.
- World settings and location settings persist and reload correctly.
- Prompt template override changes are reflected in LLM request payloads.
