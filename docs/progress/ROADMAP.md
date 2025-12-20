# WrldBldr Roadmap

This document tracks implementation progress and remaining work. For detailed system specifications, see the [systems/](../systems/) directory.

**Last Updated**: 2025-12-18  
**Overall Progress**: Core gameplay complete; Queue System complete; Phase 18-21 complete

---

## Quick Status

| Tier | Description | Status |
|------|-------------|--------|
| Tier 1 | Critical Path (Core Gameplay) | **COMPLETE** |
| Tier 2 | Feature Completeness | **COMPLETE** |
| Tier 3 | Architecture & Quality | Partial |
| Tier 4 | Session & World Management | **COMPLETE** |
| Tier 5 | Future Features | Not Started |

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

### Tier 3: Architecture & Quality - IN PROGRESS

#### 3.1 Domain-Driven Design Patterns
- [x] Event bus architecture implemented (SQLite-backed pub/sub)
- [ ] Wire WorldAggregate into services
- [ ] Implement use case traits in services

#### 3.2 Error Handling
- [ ] Define domain/application/infrastructure errors
- [ ] Replace `anyhow::Result` with typed errors
- [ ] Map errors to HTTP responses

#### 3.3 Testing (Engine)
- [ ] Set up test infrastructure
- [ ] Domain entity unit tests
- [ ] Repository integration tests
- [ ] API endpoint tests

#### 3.4 Security (Engine)
- [ ] Fix CORS configuration
- [ ] Add authentication middleware
- [ ] Add authorization checks
- [ ] Input validation

#### 3.5 Testing (Player)
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

**NPC Schedule Improvements**
- [ ] Multi-slot schedules for NPCs (e.g., 9am-5pm shop, 6pm-10pm tavern)
- [ ] Daily timeline visualization for schedule planning

**Challenge System Improvements**
- [ ] Region-level challenge binding (schema exists, needs UI/service integration)
- [ ] Context-aware challenge suggestions in Director Mode

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

---

## Environment Setup

### Engine Development

```bash
cd Engine
nix-shell

export NEO4J_PASSWORD="your_password"
export OLLAMA_BASE_URL="http://localhost:11434/v1"

cargo run
```

### Player Development

```bash
cd Player
nix-shell

npm install
cargo run      # Desktop
dx serve --web # Web
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
| [systems/](../systems/) | Game system specifications |
| [architecture/](../architecture/) | Technical architecture docs |
