# US-INV-001: Fix Player Character Inventory System - Implementation Plan

**Created**: 2025-12-24  
**Status**: Complete (~95% - manual testing deferred)  
**Effort Estimate**: 3-4 days

## Session Progress (2025-12-24)

### Completed
- [x] Phase 1: Neo4jItemRepository with container support
- [x] Phase 2: PC inventory methods in PlayerCharacterRepositoryPort
- [x] Phase 3: Fixed WebSocket handlers (EquipItem, UnequipItem, DropItem)
- [x] Phase 4: Container system (CONTAINS edge between Items)
- [x] Phase 5: GiveItem through DM approval (Service Layer + DM Selection approach)
  - [x] Created `ItemService` trait and `ItemServiceImpl`
  - [x] Added `ItemService` to `CoreServices`
  - [x] Updated `ApprovalDecision` with `AcceptWithRecipients` and `item_recipients`
  - [x] Updated `DMApprovalQueueService` with `I: ItemService` generic parameter
  - [x] Implemented `execute_give_item_with_recipients()` method
  - [x] Fixed generic parameter propagation across all services
  - [x] Updated player-ui to handle new approval decision variants
- [x] Phase 6: HasItem scene condition now checks PC inventory

### Remaining
- [ ] Phase 7: Region item stubs (low priority)
- [ ] Phase 8: Documentation and testing

---

## Architecture Decision: Item Giving Implementation

### Selected Approach: Service Layer + DM Recipient Selection

**Service Layer (Proposal 1)**:
- Create `ItemService` trait and implementation following established patterns
- Inject into `OutcomeTriggerService` and `ToolExecutionService`
- Clean separation of concerns - ItemService handles all item operations
- Consistent with existing dependency injection architecture

**DM Recipient Selection (Option B)**:
- Enhance `ApprovalItem` with recipient selection data
- DM approval UI shows available PCs to receive items
- DM can select one or more recipients, or choose "don't give"
- Selection persisted through approval flow to tool execution

### Future Investigation: Effect Executor Unification (Proposal 3)

**Note for future refactoring (US-EFFECT-UNIFICATION):**

Currently, Tool execution (`ToolExecutionService`) and Trigger execution (`OutcomeTriggerService`) 
implement similar effects through different paths. A future refactor could unify these:

```rust
// Proposed unified interface
pub trait EffectExecutor {
    async fn execute_give_item(&self, context: EffectContext, params: GiveItemParams) -> Result<EffectResult>;
    async fn execute_reveal_info(&self, context: EffectContext, params: RevealInfoParams) -> Result<EffectResult>;
    // ... other effects
}

pub struct EffectContext {
    session_id: SessionId,
    world_id: WorldId,
    acting_character_id: Option<PlayerCharacterId>,
    target_character_ids: Vec<PlayerCharacterId>,
}
```

**Benefits of unification:**
- Eliminates duplicate effect implementation
- Consistent behavior across tools and triggers
- Easier to add new effect types
- Single point of maintenance

**Estimated effort:** 3-4 days (major refactor)
**Priority:** Low (current dual-path works, can optimize later)

---

---

## Problem Statement

The PC inventory system is fundamentally broken:

1. **No Item Repository** - `ItemRepositoryPort` exists but has no implementation
2. **Wrong Node Type** - WebSocket handlers use `CharacterId` but PCs are `:PlayerCharacter` nodes
3. **No Real Item Creation** - All `GiveItem` operations only log messages, don't modify database
4. **Scene Condition Broken** - `HasItem` condition cannot work without PC inventory

## Design Decisions

### 1. Item Giving Through DM Approval
- LLM can suggest items via `give_item` tool (already exists)
- DM approval UI must allow selecting recipient PC(s)
- DM can choose NOT to give an item at resolution time
- Existing `DMApprovalQueueService` pattern will be extended

### 2. Item Creation vs Transfer
- **Transfer**: Existing `ItemId` retained, creates `POSSESSES` edge
- **Creation**: New item created with new `ItemId`, then added to inventory
- LLM suggests items by name; system creates new items during execution

### 3. World ID in Triggers
- Add `world_id: WorldId` to `OutcomeTrigger::GiveItem` struct
- Propagate from challenge/event context

### 4. Container System
- Add `can_contain_items: bool` and `container_limit: Option<u32>` to Item
- Add `max_items: Option<u32>` to Region
- Implement `(Item)-[:CONTAINS]->(Item)` relationship in Phase 4

### 5. Region Item Placement (Future)
- Design the `(Region)-[:CONTAINS_ITEM]->(Item)` edge
- Stub methods in repository
- Track via separate user story US-REGION-ITEMS

---

## Implementation Phases

### Phase 1: Item Repository Foundation
**Goal**: Enable standalone Item CRUD operations

**Files**:
- [ ] NEW: `crates/engine-adapters/src/infrastructure/persistence/item_repository.rs`
- [ ] MOD: `crates/engine-adapters/src/infrastructure/persistence/mod.rs`
- [ ] MOD: `crates/domain/src/entities/item.rs` (add container fields)
- [ ] MOD: `crates/domain/src/entities/region.rs` (add max_items field)

**Tasks**:
1. Create `Neo4jItemRepository` implementing `ItemRepositoryPort`
2. Add `items()` method to `Neo4jRepository`
3. Add container fields to Item entity
4. Add max_items field to Region entity

**Validation**: `cargo check --workspace`

---

### Phase 2: PC Inventory Repository Methods
**Goal**: Enable PC-specific inventory operations

**Files**:
- [ ] MOD: `crates/engine-ports/src/outbound/repository_port.rs`
- [ ] MOD: `crates/engine-adapters/src/infrastructure/persistence/player_character_repository.rs`

**Tasks**:
1. Add inventory methods to `PlayerCharacterRepositoryPort`:
   - `add_inventory_item(pc_id, item_id, quantity, is_equipped, acquisition_method)`
   - `get_inventory(pc_id)`
   - `get_inventory_item(pc_id, item_id)`
   - `update_inventory_item(pc_id, item_id, quantity, is_equipped)`
   - `remove_inventory_item(pc_id, item_id)`
2. Implement in `Neo4jPlayerCharacterRepository`

**Validation**: `cargo check --workspace`

---

### Phase 3: Fix WebSocket Handlers
**Goal**: Use correct ID types and repositories

**Files**:
- [ ] MOD: `crates/engine-adapters/src/infrastructure/websocket.rs` (~lines 2916-3111)

**Tasks**:
1. Fix `EquipItem` handler: `CharacterId` → `PlayerCharacterId`, use `player_characters()` repo
2. Fix `UnequipItem` handler: Same changes
3. Fix `DropItem` handler: Same changes, keep destroy behavior

**Validation**: `cargo check --workspace` && `cargo xtask arch-check`

---

### Phase 4: Container System Implementation
**Goal**: Allow items to contain other items

**Files**:
- [ ] MOD: `crates/engine-ports/src/outbound/repository_port.rs` (ItemRepositoryPort)
- [ ] MOD: `crates/engine-adapters/src/infrastructure/persistence/item_repository.rs`
- [ ] MOD: `docs/architecture/neo4j-schema.md`

**Tasks**:
1. Add container methods to `ItemRepositoryPort`:
   - `add_item_to_container(container_id, item_id, quantity)`
   - `get_container_contents(container_id)`
   - `remove_item_from_container(container_id, item_id, quantity)`
   - `get_container_capacity(container_id) -> (current, max)`
2. Implement container Neo4j queries using `(Item)-[:CONTAINS]->(Item)` edge
3. Enforce `container_limit` constraint
4. Update schema documentation

**Validation**: `cargo check --workspace`

---

### Phase 5: GiveItem Through DM Approval
**Goal**: Route item giving through DM approval with recipient selection

**Files**:
- [ ] MOD: `crates/domain/src/entities/challenge.rs` (OutcomeTrigger)
- [ ] MOD: `crates/engine-app/src/application/dto/challenge.rs` (OutcomeTriggerRequestDto)
- [ ] MOD: `crates/engine-app/src/application/dto/queue_items.rs` (ApprovalItem)
- [ ] MOD: `crates/engine-app/src/application/services/dm_approval_queue_service.rs`
- [ ] MOD: `crates/engine-app/src/application/services/tool_execution_service.rs`
- [ ] MOD: `crates/protocol/src/messages.rs` (new message types)
- [ ] MOD: `crates/player-ui/src/presentation/components/dm_panel/npc_approval.rs` (UI)

**Tasks**:
1. Update `OutcomeTrigger::GiveItem` struct:
   - Add `world_id: WorldId`
   - Change `item_name` to be used for creation
   - Add `item_type: Option<String>`
   - Add `is_unique: bool`
2. Update `ApprovalItem` to include recipient selection
3. Add `ItemGiftProposal` to protocol with recipient options
4. Update DM approval UI with:
   - Recipient PC selector (can select multiple or none)
   - Item name/description display
   - "Don't give item" option
5. Update `execute_give_item()` to:
   - Create Item node if needed
   - Add to selected PC inventories
   - Handle DM's "don't give" choice

**Validation**: `cargo check --workspace` && `cargo xtask arch-check`

---

### Phase 6: Wire Scene Condition
**Goal**: Make HasItem scene condition work

**Files**:
- [ ] MOD: `crates/engine-app/src/application/services/scene_resolution_service.rs`

**Tasks**:
1. Replace warning log with actual PC inventory check
2. Use `pc_repository.get_inventory_item(pc_id, item_id)`

**Validation**: `cargo check --workspace` && `cargo xtask arch-check`

---

### Phase 7: Region Item Placement (Stubs Only)
**Goal**: Design future region item system

**Files**:
- [ ] MOD: `crates/engine-ports/src/outbound/repository_port.rs` (RegionRepositoryPort)
- [ ] MOD: `docs/architecture/neo4j-schema.md`
- [ ] NEW: `docs/progress/US-REGION-ITEMS.md` (future user story)

**Tasks**:
1. Add stub methods to `RegionRepositoryPort`:
   - `add_item_to_region()` - returns error "Not implemented"
   - `get_region_items()` - returns error "Not implemented"
   - `remove_item_from_region()` - returns error "Not implemented"
2. Document intended edge structure
3. Create US-REGION-ITEMS user story

**Validation**: `cargo check --workspace`

---

### Phase 8: Documentation & Testing
**Goal**: Update docs and verify everything works

**Files**:
- [ ] MOD: `docs/progress/ACTIVE_DEVELOPMENT.md`
- [ ] MOD: `docs/architecture/neo4j-schema.md`

**Tasks**:
1. Update ACTIVE_DEVELOPMENT.md - mark US-INV-001 as Done
2. Update schema docs with new relationships
3. Manual testing checklist:
   - [ ] Create item in world
   - [ ] DM approval gives item to PC
   - [ ] Equip item from inventory
   - [ ] Unequip item
   - [ ] Drop item (destroys it)
   - [ ] HasItem scene condition works
   - [ ] Container item holds other items
   - [ ] Container capacity enforced

**Validation**: `cargo check --workspace` && `cargo xtask arch-check`

---

## UI Mockups

### DM Approval Card - Item Gift Proposal

```
+----------------------------------------------------------+
| NPC Dialogue Approval                              [x]   |
+----------------------------------------------------------+
| Merchant says:                                           |
| "Take this key, you'll need it for the tower."           |
|                                                          |
| Proposed Actions:                                        |
| +------------------------------------------------------+ |
| | [x] give_item: Tower Key                             | |
| |     "An ornate brass key with tower engravings"      | |
| |                                                      | |
| |     Give to: [v Select recipient(s)              ]   | |
| |               [ ] Alaric (Warrior)                   | |
| |               [x] Elena (Mage)                       | |
| |               [ ] Finn (Rogue)                       | |
| |               --------------------------------       | |
| |               [ ] Don't give this item               | |
| +------------------------------------------------------+ |
|                                                          |
| [Approve] [Approve with Edits] [Reject] [Take Over]      |
+----------------------------------------------------------+
```

### Challenge Outcome - GiveItem Trigger

```
+----------------------------------------------------------+
| Challenge Outcome Approval                               |
+----------------------------------------------------------+
| Challenge: Persuade the Guard                            |
| Roll: 15 + 3 = 18 (Success!)                            |
|                                                          |
| Outcome: The guard is impressed by your eloquence        |
| and hands over the patrol schedule.                      |
|                                                          |
| Outcome Triggers:                                        |
| +------------------------------------------------------+ |
| | [x] Give Item: Patrol Schedule                       | |
| |     Recipients: [v Elena (rolled)]                   | |
| |                                                      | |
| | [x] Reveal Info: Guard rotation times                | |
| +------------------------------------------------------+ |
|                                                          |
| [Accept] [Edit Outcome] [Request Suggestions]            |
+----------------------------------------------------------+
```

---

## Key Integration Points

| System | File | Integration |
|--------|------|-------------|
| Item Creation | `item_repository.rs` | New file, creates Item nodes |
| PC Inventory | `player_character_repository.rs` | POSSESSES edge operations |
| WebSocket | `websocket.rs:2916-3111` | Fix ID types |
| DM Approval | `dm_approval_queue_service.rs` | Recipient selection |
| Tool Execution | `tool_execution_service.rs` | Create & transfer items |
| Scene Conditions | `scene_resolution_service.rs` | HasItem check |
| Protocol | `messages.rs` | ItemGiftProposal message |
| UI | `npc_approval.rs` | Recipient selector |

---

## Progress Tracking

### Phase 1: Item Repository Foundation
- [ ] Task 1.1: Create Neo4jItemRepository
- [ ] Task 1.2: Add items() to Neo4jRepository
- [ ] Task 1.3: Add container fields to Item
- [ ] Task 1.4: Add max_items to Region
- [ ] Validation: cargo check passes

### Phase 2: PC Inventory Repository Methods
- [ ] Task 2.1: Add methods to PlayerCharacterRepositoryPort
- [ ] Task 2.2: Implement in Neo4jPlayerCharacterRepository
- [ ] Validation: cargo check passes

### Phase 3: Fix WebSocket Handlers
- [ ] Task 3.1: Fix EquipItem handler
- [ ] Task 3.2: Fix UnequipItem handler
- [ ] Task 3.3: Fix DropItem handler
- [ ] Validation: cargo check && arch-check pass

### Phase 4: Container System Implementation
- [ ] Task 4.1: Add container methods to ItemRepositoryPort
- [ ] Task 4.2: Implement container queries
- [ ] Task 4.3: Update schema docs
- [ ] Validation: cargo check passes

### Phase 5: GiveItem Through DM Approval
- [x] Task 5.1: Created ItemService trait and ItemServiceImpl
- [x] Task 5.2: Updated ApprovalDecision with AcceptWithRecipients and item_recipients
- [x] Task 5.3: Added ItemService generic to DMApprovalQueueService
- [x] Task 5.4: Implemented execute_give_item_with_recipients() method
- [x] Task 5.5: Fixed generic parameter propagation (QueueServices, GameServices, queue_workers)
- [x] Task 5.6: Updated player-ui approval_state and content.rs
- [x] Validation: cargo check && arch-check pass

### Phase 6: Wire Scene Condition
- [ ] Task 6.1: Update HasItem evaluation
- [ ] Validation: cargo check && arch-check pass

### Phase 7: Region Item Stubs
- [x] Task 7.1: Add stub methods to RegionRepositoryPort (add_item_to_region, get_region_items, remove_item_from_region)
- [x] Task 7.2: Document edge structure in neo4j-schema.md
- [x] Task 7.3: Created US-REGION-ITEMS.md user story
- [x] Validation: cargo check passes

### Phase 8: Documentation & Testing
- [x] Task 8.1: Updated ACTIVE_DEVELOPMENT.md with US-INV-001 completion
- [x] Task 8.2: Updated neo4j-schema.md with new Item fields and relationships
- [ ] Task 8.3: Manual testing complete (deferred - requires running system)
- [x] Validation: All checks pass (cargo check && cargo xtask arch-check)

---

## Dependencies Between Phases

```
Phase 1 (Item Repo) ─────────────────────────────────────────┐
       │                                                      │
       v                                                      │
Phase 2 (PC Inventory) ──────────────────────────────────────┤
       │                                                      │
       v                                                      │
Phase 3 (WebSocket Fix) ─────────────────────────────────────┤
       │                                                      │
       ├──────────────────────┐                               │
       v                      v                               │
Phase 4 (Containers)    Phase 5 (DM Approval)                 │
       │                      │                               │
       └──────────┬───────────┘                               │
                  v                                           │
            Phase 6 (Scene Condition) ────────────────────────┤
                  │                                           │
                  v                                           │
            Phase 7 (Region Stubs) ───────────────────────────┤
                  │                                           │
                  v                                           │
            Phase 8 (Docs & Testing) <────────────────────────┘
```

Phase 4 and Phase 5 can be done in parallel if desired.

---

## Risk Mitigation

1. **Large scope** - Split into clear phases with independent validation
2. **UI changes** - Keep existing approval patterns, extend don't replace
3. **Breaking changes** - No production data, safe to change schemas
4. **Session context** - Thread world_id through trigger execution path

---

## Notes

- Container limit of 0 means unlimited
- Region max_items of None means unlimited
- Items are destroyed on drop (for now) - see US-REGION-ITEMS
- LLM suggests items by name; system creates new Item nodes
- DM can always override LLM suggestions
