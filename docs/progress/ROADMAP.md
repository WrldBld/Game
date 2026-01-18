# WrldBldr Roadmap

This document tracks implementation progress and remaining work. For detailed system specifications, see the [systems/](../systems/) directory.

**Last Updated**: 2026-01-06  
**Overall Progress**: Core gameplay partial; Simplified architecture complete; WebSocket implementation partial; Scene resolution and narrative effects partial; Lore & Visual State systems partial  
**Branch**: `new-arch`

---

## Quick Status

| Tier | Description | Status |
|------|-------------|--------|
| Tier 1 | Critical Path (Core Gameplay) | **PARTIAL** |
| Tier 2 | Feature Completeness | **PARTIAL** |
| Tier 3 | Architecture & Quality | **COMPLETE** (simplified 4-crate architecture) |
| Tier 4 | Session & World Management | **PARTIAL** (WebSocket-first) |
| Tier 5 | Future Features | Not Started |

**Architecture**: See [AGENTS.md](../../AGENTS.md) for current 4-crate structure.

---

## Priority Tiers

### Tier 1: Critical Path - **PARTIAL**
- LLM Integration (Engine)
- DM Approval Flow (Engine + Player)
- Player Action Sending (Player)

### Tier 2: Feature Completeness - **PARTIAL**
- Creator Mode API Integration (Player)
- Workflow Config Persistence (Player)
- Spectator View (Player)
- Phase 16: Director Decision Queue
- Phase 18: ComfyUI Enhancements
- Phase 19: Queue System Architecture
- Phase 20: Unified Generation Queue UI
- Phase 21: Player Character Creation
- Prompt Template System (configurable LLM prompts)

### Tier 3: Architecture & Quality - **COMPLETE**

Simplified from 11+ crates with 128+ traits to 4 crates with ~10 traits.

#### 3.1 Simplified Architecture (2026-01-03)
- [x] Migrate engine from 5 crates to single `engine` crate
- [x] Reduce port traits from 128+ to ~10
- [x] Implement entity modules with direct repo calls
- [x] Implement use cases orchestrating entities
- [x] Scene resolution with condition evaluation
- [x] Event effect executor for all narrative effects

#### 3.2 Code Quality
- [x] WebSocket-first migration complete (sessions removed)
- [x] LLM context wiring complete (mood, actantial, featured NPCs)
- [x] Extract UUID parsing helpers in websocket/mod.rs (2026-01-04)
- [x] Extract DM authorization helpers (2026-01-04)
- [x] Extract navigation data builder (2026-01-04)
- [x] Extract staging NPC filter to domain method (2026-01-04)
- [x] Wire ExecuteEffects into narrative approval handler (2026-01-04)
- [x] Implement flag storage system for scene conditions (2026-01-04)

#### 3.3 Testing & Security (Deferred to Post-MVP)
- [ ] Domain entity unit tests
- [ ] Repository integration tests  
- [ ] Authentication middleware
- [ ] Authorization checks

### Tier 4: Session & World Management - **PARTIAL**
- Phase 13: World Selection Flow
- Phase 14: Rule Systems & Challenges
- Phase 15: Routing & Navigation
- Phase 17: Story Arc
- Phase 23: PC Selection, Regions, Scenes

### Tier 5: Future Features - NOT STARTED

#### 5.1 Game Design Improvements
These improvements enhance existing systems without adding major new features:

**Navigation Enhancements**
- [ ] Travel time between regions/locations (triggers game time, random encounters)
- [ ] Party formation mechanics for coordinated exploration
- [ ] Party member location visibility on mini-map
- [ ] Region item placement (items visible on scene, pick up/drop)

**NPC Schedule Improvements**
- [ ] Multi-slot schedules for NPCs (e.g., 9am-5pm shop, 6pm-10pm tavern)
- [ ] Daily timeline visualization for schedule planning

**Challenge System Improvements**
- [x] Region-level challenge binding (completed 2025-12-24)
- [ ] Context-aware challenge suggestions in Director Mode

**Dialogue System Improvements**
- [x] Dialogue persistence as StoryEvents
- [x] SPOKE_TO relationship tracking
- [ ] Mood & Expression system (inline markers in dialogue)

See individual system documents for detailed user stories.

#### 5.2 Tactical Combat - DEFERRED
> Combat is explicitly out of scope for MVP. Focus is on narrative TTRPG gameplay.

#### 5.3 Audio System
- Audio manager (music, SFX, volume)
- Scene audio integration

#### 5.4 Save/Load System
- Save file format
- Save/Load endpoints
- Save/Load UI

---

## Completed Phases Summary

| Phase | Description | Completion Date |
|-------|-------------|-----------------|
| 13 | World Selection Flow | 2025-12-11 |
| 14 | Rule Systems & Challenges | 2025-12-12 |
| 15 | Routing & Navigation | 2025-12-12 |
| 16 | Director Decision Queue | 2025-12-15 |
| 17 | Story Arc | 2025-12-15 |
| 18 | ComfyUI Enhancements | 2025-12-15 |
| 19 | Queue System Architecture | 2025-12-15 |
| 20 | Unified Generation Queue UI | 2025-12-15 |
| 21 | Player Character Creation | 2025-12-15 |
| 23 | PC Selection, Regions, Scenes | 2025-12-18 |
| - | Prompt Template System | 2025-12-20 |
| - | Phase B User Stories (Inventory, Known NPCs, Mini-map) | 2025-12-18 |
| - | Sprint 4 UX Polish (Split Party, Location Preview, View-as-Character, Style Reference, Visual Timeline) | 2025-12-25 |
| - | Simplified Architecture (4 crates, ~10 port traits) | 2026-01-03 |
| - | Scene Resolution Service (condition evaluation) | 2026-01-03 |
| - | Event Effect Executor (all narrative effects) | 2026-01-03 |
| - | Code Quality: Helper extraction, Flag Storage | 2026-01-04 |
| - | WebSocket Implementation: PC data, region items, challenge flow, directorial context | Partial |
| - | Lore System (entities, repo, handlers, UI) | Partial |
| - | Visual State System (LocationState, RegionState, activation rules) | Partial |
| - | Game Time Enhancements (TimeUseCases, TimeControl UI) | Partial |
| - | Phase 1B WebSocket CRUD (Challenge/NarrativeEvent/EventChain) | 2026-01-06 |
| - | Phase 1C WebSocket CRUD (Goal/Want/Actantial) | 2026-01-06 |

---

## Identified Gaps (For Future Work)

### Systems Needing Implementation

| Gap | Description | Priority | Status |
|-----|-------------|----------|--------|
| Flag Storage | Persistent game flags for FlagSet conditions/effects | Medium | **COMPLETE** (2026-01-04) |
| XP/Level Tracking | Track experience and level (no character advancement) | Low | Not Started |
| Combat System | Tactical combat (DEFERRED - out of scope for MVP) | None | Deferred |
| WebSocket CRUD Coverage | Scene/Act/Interaction/Skill handlers are now wired alongside prior request groups. | High | **COMPLETE** (2026-01-07) |
| HTTP Settings Endpoints | /api/settings + per-world settings + metadata | High | **COMPLETE** (2026-01-07) |
| Rule System Presets | Presets endpoint used by player | Medium | **COMPLETE** (2026-01-07) |

See `docs/plans/STRICT_REVIEW_REMEDIATION_PLAN.md` for the active remediation plan.

### Code Quality Items

| Item | Description | Priority | Status |
|------|-------------|----------|--------|
| UUID Parsing Helpers | Extract repeated pattern in websocket/mod.rs | Low | **COMPLETE** (2026-01-04) |
| DM Auth Macro | Extract authorization checks | Low | **COMPLETE** (2026-01-04) |
| Navigation Data Builder | Deduplicate between move handlers | Low | **COMPLETE** (2026-01-04) |
| Staging NPC Filter | Extract visibility filter to domain method | Low | **COMPLETE** (2026-01-04) |
| ExecuteEffects Wiring | Wire effect executor into approval handlers | Medium | **COMPLETE** (2026-01-04) |

### Documentation Updates Needed

Some system documentation files still reference legacy paths or claim APIs that are not wired yet. See `docs/plans/SYSTEMS_REVIEW_AND_FIXES.md` for the active cleanup checklist.

---

For detailed specifications of each system, see:
- [navigation-system.md](../systems/navigation-system.md) - Regions, movement, game time
- [npc-system.md](../systems/npc-system.md) - NPC presence, DM events
- [character-system.md](../systems/character-system.md) - NPCs, PCs, archetypes, relationships
- [observation-system.md](../systems/observation-system.md) - Player knowledge tracking
- [challenge-system.md](../systems/challenge-system.md) - Skill checks, dice, outcomes
- [narrative-system.md](../systems/narrative-system.md) - Events, triggers, effects
- [dialogue-system.md](../systems/dialogue-system.md) - LLM integration, DM approval
- [scene-system.md](../systems/scene-system.md) - Visual novel, backdrops, sprites
- [asset-system.md](../systems/asset-system.md) - ComfyUI, image generation
- [staging-system.md](../systems/staging-system.md) - NPC presence staging, DM approval
- [prompt-template-system.md](../systems/prompt-template-system.md) - Configurable LLM prompts
- [lore-system.md](../systems/lore-system.md) - World knowledge, discoverable chunks
- [visual-state-system.md](../systems/visual-state-system.md) - LocationState, RegionState, activation rules
- [game-time-system.md](../systems/game-time-system.md) - Game time tracking and advancement

---

## Environment Setup

### Development (Unified Game Directory)

```bash
cd Game
nix-shell shell.nix

export NEO4J_PASSWORD="your_password"
export OLLAMA_BASE_URL="http://localhost:11434/v1"

# Run both Engine and Player UI
task dev

# Or run individually:
cargo run -p wrldbldr-engine   # Engine server
dx serve --hot-reload          # Player UI (web)
```

### Verification Commands

```bash
# Must pass before committing
cargo check --workspace
cargo clippy --workspace
```

---

## Definition of Done

A task is complete when:

1. **Code**: Implementation compiles without warnings
2. **Tests**: Related tests pass (when test infrastructure exists)
3. **Integration**: Works with connected components
4. **Documentation**: Code comments for non-obvious logic
5. **Review**: No obvious bugs or security issues

---

## Recent Changelog

| Date | Changes |
|------|---------|
| 2026-01-05 | Feature: Lore System - domain entities, Neo4j repo, WebSocket handlers, LoreJournal/LoreForm UI |
| 2026-01-05 | Feature: Visual State System - LocationState/RegionState, activation rules, ResolveVisualState use case |
| 2026-01-05 | Feature: Game Time enhancements - TimeUseCases, TimeControl UI component |
| 2026-01-05 | Bug fixes: Neo4j JSON arrays, set_active atomicity, default state fallback, PartialEq, short-circuit |
| 2026-01-04 | Code quality: UUID helpers, DM auth helpers, navigation builder, staging filter, flag storage |
| 2026-01-04 | Feature: ExecuteEffects wired into narrative event approval, flag storage for scene conditions |
| 2025-12-26 | LLM context wiring complete: mood, actantial context, featured NPC names |
| 2025-12-26 | WebSocket migration Phases 1-5 complete: ~10,100 lines removed |
| 2025-12-26 | Code quality audit complete: 6 categories, 53-80h estimated remediation |
| 2025-12-26 | Created architecture remediation plan (superseded by strict review plan) |
| 2025-12-25 | Sprint 4 UX Polish complete (Split Party Warning, Location Preview, View-as-Character, Style Reference, Visual Timeline) |
| 2025-12-25 | Session ID refactor: removed session_id from story events (world-scoped only) |
| 2025-12-25 | US-STG-013 (hidden NPCs) and US-OBS-006 (unrevealed interactions) complete |
| 2025-12-24 | Documentation alignment: system docs updated to match ACTIVE_DEVELOPMENT.md |
| 2025-12-20 | Prompt Template System complete (configurable LLM prompts) |
| 2025-12-18 | Phase B complete (Inventory, Known NPCs, Mini-map, Events) |
| 2025-12-18 | Phase 23 (Regions, Navigation, Game Time, Observations) complete |
| 2025-12-18 | Documentation reorganization: created systems/, architecture/, progress/ |
| 2025-12-15 | Phases 16-21 complete (Queue System, ComfyUI, Character Creation) |
| 2025-12-15 | Event bus architecture implemented |
| 2025-12-12 | Phases 14-15, 17 complete (Rule Systems, Routing, Story Arc) |
| 2025-12-11 | Phases 13 complete (World Selection) |
| 2025-12-11 | Tiers 1-2 complete (Core Gameplay, Feature Completeness) |

---

## Related Documentation

| Document | Purpose |
|----------|---------|
| [_index.md](../_index.md) | Documentation overview |
| [MVP.md](./MVP.md) | Vision and acceptance criteria |
| [STRICT_REVIEW_REMEDIATION_PLAN.md](../plans/STRICT_REVIEW_REMEDIATION_PLAN.md) | Active remediation plan (strict layering) |
| [IMPLEMENTATION_GAPS_PLAN.md](../plans/IMPLEMENTATION_GAPS_PLAN.md) | Wiring gaps and remaining UX validation |
| [ACTIVE_DEVELOPMENT.md](./ACTIVE_DEVELOPMENT.md) | Current sprint tracking |
| [systems/](../systems/) | Game system specifications |
| [architecture/](../architecture/) | Technical architecture docs |
