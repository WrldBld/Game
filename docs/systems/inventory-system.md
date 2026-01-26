# Inventory System

## Overview

## Canonical vs Implementation

This document is canonical for how the system *should* behave in gameplay.
Implementation notes are included to track current status and may lag behind the spec.

**Legend**
- **Canonical**: Desired gameplay rule or behavior (source of truth)
- **Implemented**: Verified in code and wired end-to-end
- **Planned**: Designed but not fully implemented yet


The Inventory System manages **items owned by player characters and NPCs**, including equipment, containers, and region item placement. Items can be equipped, dropped, picked up, and organized in containers.

---

## Game Design

The Inventory System provides classic RPG item management:

1. **PC Inventory**: Player characters own items with quantity tracking
2. **Equipment Slots**: Items can be equipped for stat bonuses and visual representation
3. **Container System**: Items can contain other items (bags, chests)
4. **Region Items**: Items can exist in regions and be picked up by players
5. **Acquisition Tracking**: Each item records how it was obtained

### Acquisition Methods

| Method | Description |
|--------|-------------|
| `Gifted` | Given by another character |
| `Purchased` | Bought from a merchant |
| `Found` | Discovered in the world |
| `Crafted` | Created by the character |
| `Inherited` | Starting equipment or legacy |
| `Stolen` | Taken without permission |
| `Rewarded` | Earned as quest/challenge reward |

---

## User Stories

### Implemented

- [x] **US-INV-001**: As a player, I can see my character's inventory
- [x] **US-INV-002**: As a player, I can equip items from my inventory
- [x] **US-INV-003**: As a player, I can unequip equipped items
- [x] **US-INV-004**: As a player, I can drop items from my inventory
- [x] **US-INV-005**: As a player, I can pick up items from the current region
- [x] **US-INV-006**: As a DM, I can give items to player characters
- [x] **US-INV-007**: As a DM, I can place items in regions

### Planned

- [ ] **US-INV-008**: As a player, I can transfer items between characters
- [ ] **US-INV-009**: As a player, I can organize items in containers
- [ ] **US-INV-010**: As a DM, I can set container capacity limits

---

## Data Model

### Neo4j Nodes

```cypher
(:Item {
    id: "uuid",
    world_id: "uuid",
    name: "Sword of the Fallen",
    description: "A blade that once belonged...",
    item_type: "Weapon",
    is_unique: true,
    properties: "{...}",           // JSON item properties
    can_contain_items: false,      // Is this item a container?
    container_limit: null          // Max items if container (null = unlimited)
})
```

### Neo4j Edges

```cypher
// World contains items (item templates/definitions)
(world:World)-[:CONTAINS_ITEM]->(item:Item)

// PC inventory
(playerCharacter:PlayerCharacter)-[:POSSESSES {
    quantity: 1,
    equipped: false,
    acquired_at: datetime(),
    acquisition_method: "Gifted"
}]->(item:Item)

// NPC inventory (legacy)
(character:Character)-[:POSSESSES {
    quantity: 1,
    equipped: true,
    acquired_at: datetime(),
    acquisition_method: "Inherited"
}]->(item:Item)

// Container system - items can contain other items
(containerItem:Item)-[:CONTAINS {
    quantity: 1,
    added_at: datetime()
}]->(item:Item)

// Region item placement
(region:Region)-[:CONTAINS_ITEM {
    quantity: 1,
    added_at: datetime()
}]->(item:Item)
```

---

## API

### WebSocket Messages

#### Client -> Server

| Message | Fields | Purpose |
|---------|--------|---------|
| `EquipItem` | `pc_id`, `item_id` | Equip an item |
| `UnequipItem` | `pc_id`, `item_id` | Unequip an item |
| `DropItem` | `pc_id`, `item_id`, `quantity` | Drop/destroy item |
| `PickupItem` | `pc_id`, `item_id` | Pick up from region |

#### Server -> Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `ItemEquipped` | `pc_id`, `item_id`, `item_name` | Item was equipped |
| `ItemUnequipped` | `pc_id`, `item_id`, `item_name` | Item was unequipped |
| `ItemDropped` | `pc_id`, `item_id`, `item_name`, `quantity` | Item was dropped |
| `ItemPickedUp` | `pc_id`, `item_id`, `item_name` | Item was picked up |
| `InventoryUpdated` | `pc_id` | Inventory changed signal |

### Request/Response (via `Request` message)

| RequestPayload | Purpose |
|----------------|---------|
| `PlaceItemInRegion` | DM places item in region |
| `CreateAndPlaceItem` | Create a new item and place it in a region |

Requests for inventory CRUD (create/update/delete items, give item to PC, get PC inventory) are not wired in the engine request handlers yet.

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| Item Entity | ✅ | - | `entities/item.rs` |
| ItemRepository | ✅ | - | Neo4j persistence |
| PlayerCharacterInventoryPort | ✅ | - | Port trait |
| Inventory Use Cases | ✅ | - | Equip/unequip/drop/pickup |
| Protocol Messages | ✅ | ✅ | WebSocket messages |
| WebSocket Request Handlers | ⏳ | - | Inventory CRUD requests not wired |
| Inventory UI Component | - | ✅ | Shows PC inventory |
| Region Items Display | - | ✅ | Shows pickable items in `SceneChanged` |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/entities/item.rs` | Item entity |
| Infrastructure | `crates/engine/src/infrastructure/ports.rs` | Repository traits |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/item_repo.rs` | Neo4j item persistence |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/player_character_repo.rs` | Inventory edges |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Protocol | `crates/protocol/src/messages.rs` | Inventory messages |
| Presentation | `crates/player/src/ui/presentation/components/inventory_panel.rs` | Inventory UI |

---

## Related Systems

- **Depends on**: [Character System](./character-system.md) (PC ownership)
- **Used by**: [Challenge System](./challenge-system.md) (item requirements), [Navigation System](./navigation-system.md) (region items)

---

## Revision History

| Date | Change |
|------|--------|
| 2025-12-31 | Initial documentation |
