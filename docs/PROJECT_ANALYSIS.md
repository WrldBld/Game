# WrldBldr Project Analysis

**Date:** 2025-12-20  
**Based on:** Documentation review prioritizing latest files (ACTIVE_DEVELOPMENT.md, STAGING_IMPLEMENTATION_PLAN.md, system docs)

---

## Executive Summary

**WrldBldr** is a TTRPG (Tabletop Role-Playing Game) management system with an AI-powered game master assistant. It enables DMs to create rich narrative worlds where players interact with NPCs through a visual novel interface, with LLM-generated dialogue that requires DM approval before players see it.

### Core Architecture
- **Engine** (Rust/Axum): Backend with Neo4j graph database, Ollama LLM integration, ComfyUI for image generation
- **Player** (Rust/Dioxus): Frontend with visual novel interface
- **Hexagonal Architecture**: Strict crate-based dependency enforcement
- **Pure Graph Model**: Neo4j nodes/edges for all relational data (with documented exceptions for JSON blobs)

### Current Status
- **Phase A & B**: Complete (Core Player Experience, Player Knowledge & Agency)
- **Phase C**: In Progress (DM Tools & Advanced Features)
- **Staging System**: Complete (replaces PresenceService)
- **Core Gameplay Loop**: Functional

---

## What This Project Is About

### Vision
A TTRPG platform that combines:
1. **Rich Narrative Context**: Graph-based world model with character motivations (Actantial Model), relationships, and narrative events
2. **AI-Assisted Storytelling**: LLM generates contextual NPC responses informed by deep world knowledge
3. **DM Control**: All AI-generated content requires DM approval (accept/modify/reject/takeover)
4. **Emergent Storytelling**: Event triggers, character wants, and relationship dynamics create narrative opportunities
5. **Visual Novel Presentation**: JRPG-style exploration with backdrops, sprites, and dialogue

### Key Innovations
- **Pure Neo4j Graph Model**: All entity relationships as edges (no JSON blobs for relational data)
- **Per-Character Actantial Model**: Each character has their own view of helpers/opponents/desires
- **Dual Trigger System**: Both Engine and LLM can suggest narrative events
- **Staging System**: DM-approved NPC presence with rule-based + LLM-enhanced suggestions
- **Context-Aware LLM**: Configurable token budgets with automatic summarization

### Game Loop
```
Player Action → Engine Context Building → LLM Response → DM Approval → State Update
     ↑                                                                    │
     └──────────────────────────────────────────────────────────────────┘
```

---

## Design Gaps & Issues

### 1. **Observation System Data Model Inconsistency** ⚠️ HIGH PRIORITY

**Issue**: Documentation conflicts about how observations are stored.

- **neo4j-schema.md** (line 479): "Observation is currently persisted as properties on `Character`"
- **observation-system.md** (line 94-104): Describes `OBSERVED_NPC` edge with properties

**Impact**: Unclear which is the source of truth. The edge-based model is more aligned with the "pure graph" principle.

**Recommendation**: 
- Audit actual implementation in `observation_repository.rs`
- Standardize on edge-based model (aligns with graph-first design)
- Update documentation to match implementation

---

### 2. **Missing Repository Implementations** ⚠️ MEDIUM PRIORITY

**Issue**: Ports defined but not implemented.

- **ItemRepositoryPort**: Referenced in CODE_REVIEW_REMEDIATION_PLAN.md Phase 6.1, but Item nodes are used heavily via `POSSESSES` edges
- **GoalRepositoryPort**: Referenced in Phase 6.2, Goal nodes exist but no repository creates/updates them
- **Session nodes**: Referenced in queries (`OCCURRED_IN_SESSION`, `HAS_PC`) but no repository creates them

**Impact**: 
- Items can't be created/updated via repository pattern (violates architecture)
- Goals are "optional/legacy" per neo4j-schema.md note
- Session tracking incomplete

**Recommendation**:
- Implement ItemRepositoryPort (needed for inventory system)
- Implement GoalRepositoryPort if Actantial Model goals are used
- Create SessionRepositoryPort or document why sessions aren't persisted

---

### 3. **Narrative Event Triggers/Outcomes as JSON** ⚠️ MEDIUM PRIORITY

**Issue**: Despite "pure graph" principle, narrative event triggers/outcomes stored as JSON.

- **neo4j-schema.md** (line 459): "triggers/outcomes/effects are currently stored as JSON on the `NarrativeEvent` node (`triggers_json`, `outcomes_json`). Edges like `TRIGGERED_BY_*`, `EFFECT_*`, etc. are not created by the current persistence layer."

**Impact**: 
- Violates stated design principle ("no JSON blobs for relational data")
- Makes graph queries for "events triggered by location X" impossible
- Reduces query flexibility

**Recommendation**:
- Consider migrating to edge-based model for commonly queried triggers (location, NPC, challenge)
- Keep JSON for complex nested structures (custom conditions, complex effect chains)
- Document this as an acceptable exception if migration is too costly

---

### 4. **Dialogue Tracking Incomplete** ⚠️ MEDIUM PRIORITY

**Issue**: Dialogue exchanges not consistently persisted.

- **dialogue-system.md** (lines 63-75): US-DLG-010/011/012 are pending
- **staging-system.md**: Requires dialogue history for LLM context in presence decisions
- **ACTIVE_DEVELOPMENT.md**: Part A of Staging System (Dialogue Tracking) marked complete, but US-DLG-010/011/012 still pending

**Impact**: 
- Staging System LLM can't access recent dialogue context (despite Part A being "complete")
- Inconsistent: some dialogues may be tracked, others not

**Recommendation**:
- Verify if Part A completion means dialogue tracking is working or just the repository methods exist
- Complete US-DLG-010/011/012 if not done
- Ensure `record_dialogue_exchange` is called after every approved dialogue

---

### 5. **Challenge-Region Binding Schema vs Implementation Gap** ⚠️ LOW PRIORITY

**Issue**: Schema supports region-level challenges, but not implemented.

- **neo4j-schema.md** (line 443): "there are no `AVAILABLE_AT_LOCATION` / `AVAILABLE_AT_REGION` edges in the current persistence layer—availability is modeled with a single `AVAILABLE_AT` edge to `Location`"
- **ACTIVE_DEVELOPMENT.md** (US-CHAL-010): "Region-level Challenge Binding" - schema referenced but not implemented
- **challenge-system.md**: Not reviewed, but likely has similar notes

**Impact**: 
- Challenges can only be bound to locations, not regions
- Limits design flexibility (e.g., "convince the bartender" challenge only at bar counter region)

**Recommendation**:
- Implement `AVAILABLE_AT_REGION` edge if region-level binding is needed
- Or document that challenges are location-level only (simpler model)

---

### 6. **Hidden NPCs Feature Incomplete** ⚠️ MEDIUM PRIORITY

**Issue**: US-STG-013 (Hidden NPCs) is planned but not implemented.

- **staging-system.md** (lines 81-94): Design exists, implementation checklist provided
- **ACTIVE_DEVELOPMENT.md** (lines 36-77): In progress, Part H of Staging System pending

**Impact**: 
- Can't stage NPCs as "present but hidden" for mystery scenarios
- Unrevealed interactions not supported

**Recommendation**:
- Complete Part H of Staging System implementation
- This is a natural extension of the staging system and supports narrative design

---

### 7. **Scene Entry Conditions Not Evaluated** ⚠️ LOW PRIORITY

**Issue**: SceneCondition enum exists but evaluation missing.

- **ACTIVE_DEVELOPMENT.md** (US-SCN-009): "SceneCondition enum exists, evaluation missing"
- **scene-system.md**: Not reviewed, but likely references conditions

**Impact**: 
- Scenes can't be gated by conditions (e.g., "only show after challenge X completed")
- Limits narrative control

**Recommendation**:
- Implement `evaluate_conditions()` helper
- Call from scene resolution service
- Low effort (0.5 days estimated)

---

### 8. **PlayerCharacter Position Tracking Redundancy** ⚠️ LOW PRIORITY

**Issue**: Position tracked both as properties and edges.

- **neo4j-schema.md** (line 355): "the current persistence layer tracks location via `AT_LOCATION`/`STARTED_AT` edges and also stores `current_location_id`/`current_region_id` as properties on `PlayerCharacter` for convenience. There are no `CURRENTLY_AT` / `CURRENTLY_IN_REGION` / `STARTED_IN_REGION` edges today."

**Impact**: 
- Data duplication (properties + edges)
- Potential inconsistency if one updates but not the other
- Violates "edges for relationships" principle

**Recommendation**:
- Standardize on edges OR properties (not both)
- If edges: create `CURRENTLY_AT` / `CURRENTLY_IN_REGION` edges
- If properties: remove edge tracking (but this violates graph-first design)
- Prefer edges for consistency with design principles

---

### 9. **System Relationships Not Fully Documented** ⚠️ LOW PRIORITY

**Issue**: Some systems should be more tightly integrated.

**Missing Connections**:

1. **Observation ↔ Staging**: 
   - Observations should be created when NPCs appear in staging
   - Currently: Observations created "when NPCs appear in scenes" but staging determines presence
   - **Gap**: No explicit connection between staging approval and observation creation

2. **Narrative Events ↔ Staging**:
   - Active narrative events should influence LLM staging suggestions (they do per staging-system.md)
   - But: No clear flow for "event triggers staging change" or "staging change triggers event"
   - **Gap**: One-way influence (events → staging), but not bidirectional

3. **Challenge ↔ Observation**:
   - Challenge outcomes can create observations (deduced type)
   - But: No clear integration for "observation unlocks challenge" or "challenge reveals observation"
   - **Gap**: One-way flow (challenge → observation)

4. **Dialogue ↔ Observation**:
   - Dialogue can reveal NPC information (heard_about type)
   - But: No explicit "dialogue creates observation" flow documented
   - **Gap**: Should be automatic when DM shares NPC location info

**Recommendation**:
- Document these relationships in system docs
- Consider adding explicit integration points (e.g., "when staging approved, create direct observations for all present NPCs")
- Add cross-system diagrams showing data flow

---

### 10. **Architecture Violations Not Tracked** ⚠️ LOW PRIORITY

**Issue**: Some violations may exist but aren't documented.

- **hexagonal-architecture.md** (line 156): "None tracked in this document"
- **CODE_REVIEW_REMEDIATION_PLAN.md** (Phase 1.4): Documents one violation in `services.rs` (type aliases importing infrastructure)

**Impact**: 
- Unclear if other violations exist
- No process for tracking exceptions

**Recommendation**:
- Run `cargo xtask arch-check` regularly
- Document any approved violations in HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md
- Consider adding violation detection to CI

---

## Systems That Should Be More Related

### 1. **Staging + Observation Integration**

**Current State**: Separate systems, minimal integration.

**Should Be**: 
- When staging is approved and NPCs appear, automatically create `Direct` observations for all present NPCs
- Staging system should query observation history when generating LLM context (it does per staging-system.md, but connection not explicit)

**Benefit**: Ensures player knowledge matches what they've seen, supports "fog of war" gameplay

---

### 2. **Dialogue + Observation Integration**

**Current State**: Dialogue can share NPC location info, but not consistently tracked.

**Should Be**:
- When DM shares NPC location via `ShareNpcLocation`, create `HeardAbout` observation
- When dialogue reveals NPC information, automatically create observation
- Dialogue history should be queryable by observation system

**Benefit**: Consistent knowledge tracking, supports investigation gameplay

---

### 3. **Narrative Events + Staging Bidirectional**

**Current State**: Events influence staging (LLM considers active events), but staging doesn't trigger events.

**Should Be**:
- Staging changes could trigger "NPC Arrived" or "NPC Left" events
- Events could modify staging (e.g., "Festival event makes all NPCs present at market")

**Benefit**: More dynamic world, events have visible consequences

---

### 4. **Challenge + Observation Integration**

**Current State**: Challenge outcomes can create observations, but observations don't unlock challenges.

**Should Be**:
- Observations could be prerequisites for challenges (e.g., "must have observed NPC X before challenge Y")
- Challenge completion could reveal new observations automatically

**Benefit**: Investigation gameplay, knowledge-gated challenges

---

## Positive Design Patterns

### 1. **Staging System Design**
- Excellent separation of concerns (rule-based vs LLM)
- DM approval workflow maintains narrative control
- Pre-staging UI for proactive DM management
- TTL caching reduces repetitive approvals

### 2. **Hexagonal Architecture Enforcement**
- Crate-based dependency DAG prevents violations
- `cargo xtask arch-check` enforces rules
- Clear port/adapter pattern

### 3. **Graph-First Data Model**
- Most relationships as edges (excellent for queries)
- JSON blobs only for non-relational data (documented exceptions)

### 4. **Context Budget System**
- Configurable token limits per category
- Automatic summarization when over budget
- Prevents LLM context bloat

### 5. **Dual Decision Modes**
- Rule-based (deterministic) + LLM-enhanced (contextual) for staging
- Engine + LLM can suggest narrative events
- DM chooses, maintains control

---

## Recommendations Summary

### High Priority
1. **Resolve Observation System data model inconsistency** (edge vs properties)
2. **Complete Hidden NPCs feature** (US-STG-013) - natural extension of staging

### Medium Priority
3. **Implement missing repositories** (Item, Goal, Session)
4. **Complete dialogue tracking** (US-DLG-010/011/012)
5. **Consider migrating narrative triggers to edges** (or document exception)

### Low Priority
6. **Standardize PlayerCharacter position tracking** (edges vs properties)
7. **Implement scene entry conditions** (US-SCN-009)
8. **Document system relationships** more explicitly
9. **Add explicit integration points** between systems (staging → observation, dialogue → observation)

### Future Enhancements
10. **Region-level challenge binding** (if needed)
11. **Multi-slot NPC schedules** (US-NPC-010)
12. **Travel time between locations** (US-NAV-011)

---

## Conclusion

WrldBldr is a well-architected TTRPG platform with strong design principles (hexagonal architecture, graph-first data model, DM-controlled AI). The core gameplay loop is functional, and the Staging System is a sophisticated addition.

**Main Gaps**: 
- Some incomplete features (hidden NPCs, dialogue tracking)
- Data model inconsistencies (observation storage, narrative triggers as JSON)
- Missing repository implementations
- Systems could be more tightly integrated (staging ↔ observation, dialogue ↔ observation)

**Strengths**:
- Clear architecture with enforcement tools
- Comprehensive documentation
- Thoughtful design (staging system, context budgets, dual decision modes)

The project is in good shape overall, with most gaps being incremental improvements rather than fundamental design flaws.

