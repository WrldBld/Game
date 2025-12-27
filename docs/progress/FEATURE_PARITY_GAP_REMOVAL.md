# Feature Parity Gap Removal

**Status:** COMPLETE (2025-12-24)
**Purpose:** Regained functionality and UX that existed before while preserving the hexagonal/ports-based crate structure.

---

## Summary

All feature parity gaps identified have been resolved. This document is retained for historical reference.

| Gap ID | Description | Status |
|--------|-------------|--------|
| GAP-UI-INV-001 | Inventory equip/unequip toggle | RESOLVED |
| GAP-UI-INV-002 | Inventory drop item | RESOLVED |
| GAP-UI-NAV-001 | Mini-map background image | RESOLVED |
| GAP-DM-CHAL-001 | Ad-hoc challenge creation | RESOLVED |
| GAP-ENG-AST-001 | Asset generation persistence | RESOLVED |
| GAP-ENG-CHAL-002 | Challenge outcome triggers | RESOLVED |
| GAP-ENG-SCN-001 | Scene broadcast on endpoints | RESOLVED |
| GAP-UI-STATE-001 | State updates in message handler | RESOLVED |
| GAP-DIR-UX-001 | View-as-character mode | RESOLVED (Sprint 4) |

---

## Resolution Details

### GAP-UI-INV-001/002: Inventory Actions (2025-12-24)
- Added `EquipItem`, `UnequipItem`, `DropItem` client messages to protocol
- Added `ItemEquipped`, `ItemUnequipped`, `ItemDropped`, `InventoryUpdated` server messages
- Added `get_inventory_item()` to `CharacterRepositoryPort` and implemented in Neo4j adapter
- Wired UI callbacks in `pc_view.rs`

### GAP-UI-NAV-001: Mini-map Image (2025-12-24)
- Added `map_asset: Option<String>` to `RegionData` in protocol
- Updated all `RegionData` constructions in WebSocket handlers
- Wired `map_image` prop in `pc_view.rs`

### GAP-DM-CHAL-001: Ad-hoc Challenge Creation (2025-12-24)
- Added `create_adhoc_challenge()` to `GameConnectionPort` trait
- Implemented in WebSocket client adapter
- Wired DM view callback to port method

### GAP-ENG-AST-001: Asset Generation (2025-12-24)
- Implemented ComfyUI `/history/{prompt_id}` polling
- Download generated images and save to `data/generated_assets/`
- Create Asset records via `AssetRepositoryPort`

### GAP-ENG-CHAL-002: Challenge Outcome Triggers (2025-12-24)
- Added `original_triggers` to `ChallengeOutcomeApprovalItem`
- Convert DTOs to domain `OutcomeTrigger` on approval
- Execute triggers via `OutcomeTriggerService`

### GAP-ENG-SCN-001: Scene Broadcast (2025-12-24)
- Broadcast `ServerMessage::SceneUpdate` after PC scene resolution
- Added to `create_player_character` and `update_player_character_location`

### GAP-UI-STATE-001: State Updates (2025-12-24)
- Added `observations_refresh_counter` to GameState
- Update `selected_pc_id` signal on `PcSelected` message

### GAP-DIR-UX-001: View-as-Character (Sprint 4)
- Implemented as part of Sprint 4 UX Polish
- Location preview modal and view-as-character mode complete

---

## Verification

All gaps verified with:
- `cargo xtask arch-check` - PASS
- `cargo check --workspace` - PASS
