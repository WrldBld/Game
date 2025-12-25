# US-REGION-ITEMS: Region Item Placement System

**Created**: 2025-12-24  
**Status**: Not Started  
**Priority**: Low  
**Depends On**: US-INV-001 (completed)

## Overview

Enable items to be placed in regions (world locations) rather than only in PC inventories.
This supports:
- Items dropped by PCs appearing in the region
- Pre-placed loot and treasures
- Environmental items for puzzles and interactions
- Shop inventories tied to merchant locations

## Background

Currently, when a PC drops an item, it is destroyed. This user story will implement
the `(Region)-[:CONTAINS_ITEM]->(Item)` relationship to allow items to exist in
the game world outside of inventories.

## Requirements

### Functional Requirements

1. **Item Placement**
   - Items can be placed in a region
   - Region capacity (max_items) is enforced
   - Items are visible to PCs in the same region

2. **Item Pickup**
   - PCs can pick up items from regions
   - Picked up items are removed from region and added to PC inventory
   - DM can approve/deny pickup attempts

3. **Item Drop**
   - When PC drops item, it appears in their current region
   - If region is at capacity, drop fails with message

4. **Visibility**
   - Region items are included in scene context for LLM
   - Items can have visibility conditions (hidden, revealed, etc.)

### Non-Functional Requirements

- Neo4j queries for region items must be efficient
- Item state changes should be atomic (no half-dropped items)

## Technical Design

### Neo4j Schema

```cypher
// Edge: Region contains items
(r:Region)-[:CONTAINS_ITEM {
    placed_at: datetime,
    placed_by: String,        // PC ID or "system"
    visibility: String,       // "visible", "hidden", "revealed"
    quantity: Integer
}]->(i:Item)
```

### Repository Methods

Already stubbed in `RegionRepositoryPort`:
- `add_item_to_region(region_id, item_id)` -> Result<()>
- `get_region_items(region_id)` -> Result<Vec<Item>>
- `remove_item_from_region(region_id, item_id)` -> Result<()>

Additional methods needed:
- `get_region_item_count(region_id)` -> Result<u32>
- `get_visible_region_items(region_id, observer_pc_id)` -> Result<Vec<Item>>

### Service Layer

New `RegionItemService`:
- `place_item(region_id, item_id, placed_by)` -> Result<()>
- `pickup_item(region_id, item_id, pc_id)` -> Result<()>
- `drop_item(pc_id, item_id)` -> Result<()>
- `list_items(region_id, visibility_filter)` -> Result<Vec<Item>>

### Integration Points

| System | Integration |
|--------|-------------|
| DropItem WebSocket | Call RegionItemService.drop_item instead of destroying |
| Scene Resolution | Include region items in scene context |
| LLM Prompts | Add region items to NPC response context |
| DM Panel | Show region items in location view |

## Implementation Phases

### Phase 1: Repository Implementation
- Implement Neo4j queries for CONTAINS_ITEM edge
- Add edge properties (placed_at, placed_by, visibility)
- Enforce max_items capacity

### Phase 2: Service Layer
- Create RegionItemService
- Wire into CoreServices or LocationServices

### Phase 3: Drop Item Integration
- Update DropItem handler to place item in region
- Handle capacity full scenario

### Phase 4: Pickup Integration
- Add PickupItem WebSocket message
- Create DM approval flow for contested pickups

### Phase 5: LLM Context
- Add region items to scene context builder
- Update NPC response prompts

### Phase 6: UI Updates
- Show region items in location panel
- Add pickup interaction for players

## Effort Estimate

- Phase 1: 4 hours (repository)
- Phase 2: 2 hours (service)
- Phase 3: 2 hours (drop integration)
- Phase 4: 4 hours (pickup + approval)
- Phase 5: 2 hours (LLM context)
- Phase 6: 4 hours (UI)

**Total**: ~18 hours (2-3 days)

## Open Questions

1. Should region items be visible to all PCs or only those in the region?
2. How to handle contested pickup (multiple PCs reaching for same item)?
3. Should hidden items require a search/perception check?
4. How to handle shop inventories vs dropped items?

## References

- US-INV-001: Fix Player Character Inventory System (prerequisite)
- Neo4j Schema: `docs/architecture/neo4j-schema.md`
- Region Entity: `crates/domain/src/entities/region.rs`
