# Tech Debt Remediation Plan

**Created**: 2024-12-30
**Updated**: 2024-12-31
**Status**: In Progress

---

## Executive Summary

This plan addresses validated tech debt categories:
1. ~~**ISP Treatment**: Split `GameConnectionPort`~~ - **ALREADY COMPLETE** (validated 2024-12-31)
2. **File Size Reduction**: Split 8 files exceeding 1,400 lines
3. **Duplicate Code Elimination**: ~~Remove 3 confirmed duplicate structs~~ **COMPLETE** and consolidate parse utilities
4. **Forward Compatibility**: Add `#[serde(other)]` to 2 enums (not 4 - see validation notes)

**Estimated Remaining Effort**: 2-3 developer days
**Risk Level**: Low (refactoring, no feature changes)

---

## Completed Work

### Phase 1: GameConnectionPort ISP Treatment - COMPLETE

**Validated 2024-12-31**: The ISP split was already implemented. The existing structure in `player-ports/src/outbound/game_connection/` includes:

| Existing Trait | Methods | Status |
|----------------|---------|--------|
| `ConnectionLifecyclePort` | 5 | Complete |
| `SessionCommandPort` | 1 | Complete |
| `PlayerActionPort` | 7 | Complete |
| `DmControlPort` | 12 | Complete |
| `NavigationPort` | 2 | Complete |
| `GameRequestPort` | 3 | Complete |

**Note**: Callback methods (`on_state_change`, `on_message`) remain on the main trait due to mockall limitations with `Fn` objects. This is documented and intentional.

**Optional future work**: Migrate services to depend on narrower sub-traits (low priority, improves testability).

---

### Phase 3: Duplicate Code Elimination - PARTIALLY COMPLETE

#### 3.1 `DmInfo` Duplicate - COMPLETE (2024-12-31)

**Changes made**:
- Removed local `DmInfo` struct from `engine-adapters/src/infrastructure/world_connection_manager.rs`
- Updated import to use `DmInfo` directly from `wrldbldr_engine_ports::outbound`
- Simplified `get_dm_info()` impl to return directly without conversion
- Updated `mod.rs` to re-export from ports

#### 3.2 `CreateSkillRequest` + `UpdateSkillRequest` Duplicates - COMPLETE (2024-12-31)

**Changes made**:
- Removed local `CreateSkillRequest` and `UpdateSkillRequest` structs from `engine-app/src/application/services/skill_service.rs`
- Updated imports to use types directly from `wrldbldr_engine_ports::outbound`
- Removed conversion code in `impl SkillServicePort`
- Updated `services/mod.rs` to re-export from ports
- Added `SkillCategory` import to test module

#### 3.3 `GenerationRequest` Duplicate - COMPLETE (2024-12-31)

**Changes made**:
- Removed local `GenerationRequest` struct from `engine-app/src/application/services/generation_service.rs`
- Updated import to use `GenerationRequest` directly from `wrldbldr_engine_ports::outbound`
- Removed conversion code in `impl GenerationServicePort`

---

## Remaining Work

### Phase 2: Large File Splitting

**Note**: File size thresholds are guidelines, not strict requirements. Focus on logical organization.

#### 2.1 `character_repository.rs` (2,073 lines)

**Target Structure**:
```
engine-adapters/src/infrastructure/persistence/character_repository/
├── mod.rs                    # Re-exports, Neo4jCharacterRepository struct
├── crud.rs                   # CharacterCrudPort impl (~200 lines)
├── want.rs                   # CharacterWantPort impl (~350 lines)
├── actantial.rs              # CharacterActantialPort impl (~300 lines)
├── inventory.rs              # CharacterInventoryPort impl (~250 lines)
├── location.rs               # CharacterLocationPort impl (~400 lines)
├── disposition.rs            # CharacterDispositionPort impl (~250 lines)
├── converters.rs             # row_to_character, stored types (~300 lines)
└── tests.rs                  # #[cfg(test)] module (if any)
```

#### 2.2 `narrative_event_repository.rs` (2,005 lines)

**Target Structure**:
```
engine-adapters/src/infrastructure/persistence/narrative_event_repository/
├── mod.rs                    # Re-exports, struct definition
├── crud.rs                   # NarrativeEventCrudPort impl
├── query.rs                  # NarrativeEventQueryPort impl
├── tie.rs                    # NarrativeEventTiePort impl
├── npc.rs                    # NarrativeEventNpcPort impl
├── stored_types.rs           # StoredEventEffect, StoredOutcomeTrigger, etc.
├── conversions.rs            # From impls for stored <-> domain
└── row_mapping.rs            # row_to_narrative_event() helper
```

#### 2.3 `story_event_repository.rs` (1,814 lines)

**Target Structure**:
```
engine-adapters/src/infrastructure/persistence/story_event_repository/
├── mod.rs                    # Re-exports, struct definition
├── crud.rs                   # StoryEventCrudPort impl
├── query.rs                  # StoryEventQueryPort impl
├── edge.rs                   # StoryEventEdgePort impl
├── dialogue.rs               # StoryEventDialoguePort impl
├── stored_types.rs           # StoredStoryEventType, etc.
├── conversions.rs            # From impls
└── row_mapping.rs            # row_to_story_event() helper
```

#### 2.4 `sheet_template.rs` (1,786 lines)

**Target Structure**:
```
domain/src/entities/sheet_template/
├── mod.rs                    # Core types (SheetTemplate, SheetSection, etc.)
├── field_types.rs            # FieldType enum and related types
├── defaults/
│   ├── mod.rs                # default_sheet_template_for_variant()
│   ├── dnd5e.rs              # D&D 5e template
│   ├── pathfinder.rs         # Pathfinder templates
│   ├── coc.rs                # Call of Cthulhu template
│   ├── fate.rs               # FATE templates
│   ├── pbta.rs               # PbtA template
│   ├── swn.rs                # Stars Without Number template
│   ├── savage_worlds.rs      # Savage Worlds template
│   └── cypher.rs             # Cypher System template
└── tests.rs
```

#### 2.5 `story_event_service.rs` (1,474 lines)

**Target Structure**:
```
engine-app/src/application/services/story_event/
├── mod.rs                    # StoryEventService struct, trait definition
├── recording.rs              # Event recording methods
├── query.rs                  # Query/list methods
├── admin.rs                  # Admin operations (soft delete, etc.)
├── dialogue.rs               # Dialogue-related methods
└── isp_impls.rs              # ISP trait implementations (delegations)
```

#### 2.6 `session_message_handler.rs` (1,400 lines)

**Target Structure**:
```
player-ui/src/presentation/handlers/session_message/
├── mod.rs                    # Main handle_server_message dispatcher
├── gameplay.rs               # Gameplay events (scene, movement, etc.)
├── challenge.rs              # Challenge/outcome events (~300 lines - largest)
├── generation.rs             # Asset generation events
├── approval.rs               # DM approval events
├── staging.rs                # Staging system events
└── inventory.rs              # Inventory update events
```

---

### Phase 3 Continued: Parse Utility Consolidation

**Current State**:
- `engine-adapters/src/infrastructure/websocket/context.rs:211-258` (9 parse functions, returns `ServerMessage`)
- `engine-app/src/application/handlers/common.rs:27-136` (17 parse functions, returns `ResponseResult`)
- 8 private `parse_*_id` functions scattered in websocket handlers (misc.rs, movement.rs, inventory.rs)
- `parse_pc_id` already exists in `websocket/handlers/common.rs`

**Action**:
1. **Keep both Result-returning families** - different error types serve different transport layers
2. **Consolidate Option-returning helpers** to `websocket/handlers/common.rs`:
   - Add: `parse_npc_id` (from misc.rs)
   - Add: `parse_region_id` (from movement.rs)
   - Add: `parse_location_id` (from movement.rs)
   - Add: `parse_item_id` (from inventory.rs)
3. **Remove private duplicates** from misc.rs, movement.rs, inventory.rs
4. **Bonus**: Consolidate duplicate `extract_context` in movement.rs (matches `extract_context_opt` in common.rs)

---

### Phase 4: Forward Compatibility Fixes - UPDATED

**Validation Result**: Only 2 of 4 enums can have `#[serde(other)]` added.

`#[serde(other)]` only works on **unit variants**. Enums with data variants cannot use this attribute.

| File | Enum | Has Data Variants | Can Fix? |
|------|------|-------------------|----------|
| `protocol/src/dto.rs:195` | `PromptMappingTypeDto` | No | **YES** |
| `protocol/src/dto.rs:383` | `InputTypeDto` | Yes (`Select(Vec<String>)`) | **NO** |
| `engine-dto/src/persistence.rs:559` | `SectionLayoutDto` | Yes (`Grid { columns: u8 }`) | **NO** |
| `engine-dto/src/persistence.rs:618` | `ItemListTypeDto` | No | **YES** |

**Action**:
- Add `#[serde(other)] Unknown` to `PromptMappingTypeDto`
- Add `#[serde(other)] Unknown` to `ItemListTypeDto`
- Document that `InputTypeDto` and `SectionLayoutDto` cannot have forward-compatible catch-all due to data variants

---

## Updated Implementation Schedule

| Phase | Task | Effort | Status |
|-------|------|--------|--------|
| ~~1.x~~ | ~~GameConnectionPort ISP~~ | ~~8h~~ | **COMPLETE** (pre-existing) |
| 2.1 | Split `character_repository.rs` | 2h | Pending |
| 2.2 | Split `narrative_event_repository.rs` | 2h | Pending |
| 2.3 | Split `story_event_repository.rs` | 2h | Pending |
| 2.4 | Split `sheet_template.rs` | 2h | Pending |
| 2.5 | Split `story_event_service.rs` | 2h | Pending |
| 2.6 | Split `session_message_handler.rs` | 2h | Pending |
| ~~3.1~~ | ~~Remove `DmInfo` duplicate~~ | ~~15m~~ | **COMPLETE** |
| ~~3.2~~ | ~~Remove `CreateSkillRequest` duplicate~~ | ~~15m~~ | **COMPLETE** |
| ~~3.3~~ | ~~Remove `GenerationRequest` duplicate~~ | ~~15m~~ | **COMPLETE** |
| 3.4 | Consolidate parse utilities | 1h | Pending |
| 4.1 | Add `#[serde(other)]` to 2 enums | 15m | Pending |

**Remaining Effort**: ~13 hours (1.5-2 developer days)

---

## Validation Checklist

After each phase:
- [x] `cargo check --workspace` passes
- [x] `cargo test -p wrldbldr-engine-app --lib` passes (70 tests)
- [x] `cargo xtask arch-check` passes
- [x] No new clippy warnings

---

## Success Metrics

| Metric | Before | After | Status |
|--------|--------|-------|--------|
| Files >1,000 lines | 8 | 0 | Pending |
| God traits (15+ methods) | 1 | 0 | **COMPLETE** |
| True duplicate structs | 3 | 0 | **COMPLETE** |
| Duplicate parse functions | 8 | 0 | Pending |
| Enums missing `#[serde(other)]` | 2* | 0 | Pending |

*Reduced from 4 to 2 after validation (2 enums have data variants)
