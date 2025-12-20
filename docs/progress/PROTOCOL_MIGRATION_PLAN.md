# Protocol Crate Migration Plan

## Overview

Migrate all WebSocket wire-format types from Engine and Player into the shared `protocol` crate, eliminating ~1,500 lines of duplicate code and ensuring compile-time type safety across the entire stack.

## Decisions

| Decision | Choice |
|----------|--------|
| Type Naming | Use Engine names (`SceneData`, `CharacterData`, `RegionData`, etc.) |
| Player's `emotion` field | Add to protocol, create future task for Engine to populate it |
| Execution | Sequential (Stream A → B → C → D) |
| Commits | After each stream completes |
| Backwards Compatibility | Not required - clean break |
| Re-export shims | **Forbidden**: no `pub use` / module re-export layers in engine/player/app crates; import from the owning crate directly |

---

## Stream A: Protocol Crate Expansion

**Commit Message**: `feat(protocol): add WebSocket message types and shared DTOs`

### Tasks

| Task | Description |
|------|-------------|
| A.1 | Fix `ParticipantRole` in `types.rs` - change to `DungeonMaster`, `Player`, `Spectator` |
| A.2 | Add `ProposedToolInfo` struct to `types.rs` |
| A.3 | Add `ApprovalDecision` enum to `types.rs` |
| A.4 | Add `ChallengeSuggestionInfo` struct to `types.rs` |
| A.5 | Add `NarrativeEventSuggestionInfo` struct to `types.rs` |
| A.6 | Add scene types to `messages.rs`: `SceneData`, `CharacterData` (with `emotion` field), `CharacterPosition`, `InteractionData`, `DialogueChoice` |
| A.7 | Add navigation types to `messages.rs`: `RegionData`, `NavigationData`, `NavigationTarget`, `NavigationExit`, `NpcPresenceData` |
| A.8 | Add challenge types to `messages.rs`: `DiceInputType`, `AdHocOutcomes`, `OutcomeDetailData`, `ChallengeOutcomeDecisionData`, `OutcomeBranchData` |
| A.9 | Add session types to `messages.rs`: `ParticipantInfo`, `DirectorialContext`, `NpcMotivationData`, `SplitPartyLocation` |
| A.10 | Replace `ClientMessage` placeholder with full enum (~22 variants) |
| A.11 | Replace `ServerMessage` placeholder with full enum (~40 variants) |
| A.12 | Keep `lib.rs` minimal; expose types via canonical module paths (avoid broad `pub use` re-export shims) |
| A.13 | Verify protocol crate compiles |
| A.14 | Commit |

---

## Stream B: Engine Import Updates

**Commit Message**: `refactor(engine): use protocol crate for WebSocket types`

### Tasks

| Task | Description |
|------|-------------|
| B.1 | Update Engine code to import from `wrldbldr_protocol::...` directly (do not re-export protocol types from `domain/value_objects/mod.rs`) |
| B.2 | Update `domain/value_objects/approval.rs` - remove types now in protocol |
| B.3 | Update `application/dto/queue_items.rs` - import from protocol |
| B.4 | Refactor `infrastructure/websocket/messages.rs` - remove duplicates |
| B.5-B.19 | Update imports across ~15 files |
| B.20 | Verify Engine compiles |
| B.21 | Commit |

---

## Stream C: Player Import Updates

**Commit Message**: `refactor(player): use protocol crate for WebSocket types`

### Type Rename Mapping

| Old Player Name | New Protocol Name |
|-----------------|-------------------|
| `SceneSnapshot` | `SceneData` |
| `SceneCharacterState` | `CharacterData` |
| `SceneRegionInfo` | `RegionData` |
| `ProposedTool` | `ProposedToolInfo` |

### Tasks

| Task | Description |
|------|-------------|
| C.1 | Update `domain/value_objects/mod.rs` |
| C.2 | Major refactor of `application/dto/websocket_messages.rs` |
| C.3-C.18 | Update imports across ~20 files |
| C.19 | Verify Player compiles |
| C.20 | Commit |

---

## Stream D: Player ID Migration (String → UUID)

**Commit Message**: `refactor(player): migrate ID types from String to UUID`

### Tasks

| Task | Description |
|------|-------------|
| D.1 | Replace `domain/value_objects/ids.rs` - use protocol IDs |
| D.2-D.6 | Update domain entities to use UUID-based IDs |
| D.7 | Search and update remaining String ID usages |
| D.8 | Verify native build |
| D.9 | Verify WASM build |
| D.10 | Commit |

---

## Future Work: Character Emotion Support

**Context**: During protocol migration, the `emotion` field was added to `CharacterData` 
to match Player's `SceneCharacterState`. Engine currently sends `None`.

**Task**: Implement emotion tracking in Engine
- Add emotion state to character session tracking
- Derive emotion from dialogue context or NPC mood
- Send emotion in `CharacterData` when broadcasting scene updates

**Files to Update**:
- `engine/src/infrastructure/session/game_session.rs`
- `engine/src/infrastructure/websocket.rs` (where CharacterData is constructed)

**Priority**: Low (cosmetic feature for visual novel display)

---

## Verification Checklist

| Check | Command |
|-------|---------|
| Protocol compiles | `cargo check -p wrldbldr-protocol` |
| Engine compiles | `cargo check -p wrldbldr-engine` |
| Player compiles (native) | `cargo check -p wrldbldr-player` |
| Player compiles (WASM) | `cargo check -p wrldbldr-player --target wasm32-unknown-unknown` |
| Full workspace | `cargo check --workspace` |

---

## Summary

| Stream | Files | Commit |
|--------|-------|--------|
| A - Protocol Expansion | 3 | `feat(protocol): add WebSocket message types and shared DTOs` |
| B - Engine Updates | ~19 | `refactor(engine): use protocol crate for WebSocket types` |
| C - Player Updates | ~20 | `refactor(player): use protocol crate for WebSocket types` |
| D - Player ID Migration | ~10 | `refactor(player): migrate ID types from String to UUID` |
