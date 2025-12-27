# WrldBldr Roadmap

This document tracks implementation progress and remaining work. For detailed system specifications, see the [systems/](../systems/) directory.

**Last Updated**: 2025-12-26  
**Overall Progress**: Core gameplay complete; WebSocket-first migration complete; LLM context wiring complete; Code quality audit done

---

## Quick Status

| Tier | Description | Status |
|------|-------------|--------|
| Tier 1 | Critical Path (Core Gameplay) | **COMPLETE** |
| Tier 2 | Feature Completeness | **COMPLETE** |
| Tier 3 | Architecture & Quality | **IN PROGRESS** |
| Tier 4 | Session & World Management | **COMPLETE** (WebSocket-first) |
| Tier 5 | Future Features | Not Started |

See [CONSOLIDATED_IMPLEMENTATION_PLAN.md](./CONSOLIDATED_IMPLEMENTATION_PLAN.md) for prioritized remaining work.

---

## Priority Tiers

### Tier 1: Critical Path - **COMPLETE**
- LLM Integration (Engine)
- DM Approval Flow (Engine + Player)
- Player Action Sending (Player)

### Tier 2: Feature Completeness - **COMPLETE**
- Creator Mode API Integration (Player)
- Workflow Config Persistence (Player)
- Spectator View (Player)
- Phase 16: Director Decision Queue
- Phase 18: ComfyUI Enhancements
- Phase 19: Queue System Architecture
- Phase 20: Unified Generation Queue UI
- Phase 21: Player Character Creation
- Prompt Template System (configurable LLM prompts)

### Tier 3: Architecture & Quality - IN PROGRESS

See [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) for detailed audit findings.

#### 3.1 Domain-Driven Design Patterns
- [x] Event bus architecture implemented (SQLite-backed pub/sub)
- [ ] Wire WorldAggregate into services
- [ ] Implement use case traits in services

#### 3.2 Code Quality (NEW - 2025-12-26)
- [x] WebSocket-first migration complete (sessions removed)
- [x] LLM context wiring complete (mood, actantial, featured NPCs)
- [ ] Fix player-app REST service calls to deleted endpoints (CRITICAL)
- [ ] Delete dead code modules (~680 lines)
- [ ] Create shared row converters module
- [ ] Move DTOs from ports to app/domain layer

#### 3.3 Error Handling
- [ ] Define domain/application/infrastructure errors
- [ ] Replace `anyhow::Result` with typed errors
- [ ] Map errors to HTTP responses

#### 3.4 Testing (Engine)
- [ ] Set up test infrastructure
- [ ] Domain entity unit tests
- [ ] Repository integration tests
- [ ] API endpoint tests

#### 3.5 Security (Engine)
- [x] Fix CORS configuration (env-based)
- [ ] Add authentication middleware
- [ ] Add authorization checks
- [ ] Input validation

#### 3.6 Testing (Player)
- [ ] Set up test infrastructure
- [ ] State management tests
- [ ] Component tests

### Tier 4: Session & World Management - **COMPLETE**
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
- [ ] Dialogue persistence as StoryEvents
- [ ] SPOKE_TO relationship tracking
- [ ] Mood & Expression system (inline markers in dialogue)

See individual system documents for detailed user stories.

#### 5.2 Tactical Combat
- Combat service (turn order, movement, attack resolution)
- Combat WebSocket messages
- Grid renderer
- Combat UI

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
cargo run -p wrldbldr-engine-runner   # Engine only
dx serve --hot-reload                  # Player UI (web)
```

### Verification Commands

```bash
# Must pass before committing
cargo check --workspace
cargo xtask arch-check
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
| 2025-12-26 | LLM context wiring complete: mood, actantial context, featured NPC names |
| 2025-12-26 | WebSocket migration Phases 1-5 complete: ~10,100 lines removed |
| 2025-12-26 | Code quality audit complete: 6 categories, 53-80h estimated remediation |
| 2025-12-26 | Created CONSOLIDATED_IMPLEMENTATION_PLAN.md |
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
| [CONSOLIDATED_IMPLEMENTATION_PLAN.md](./CONSOLIDATED_IMPLEMENTATION_PLAN.md) | Prioritized remaining work |
| [CODE_QUALITY_REMEDIATION_PLAN.md](./CODE_QUALITY_REMEDIATION_PLAN.md) | Technical debt audit |
| [ACTIVE_DEVELOPMENT.md](./ACTIVE_DEVELOPMENT.md) | Current sprint tracking |
| [systems/](../systems/) | Game system specifications |
| [architecture/](../architecture/) | Technical architecture docs |
