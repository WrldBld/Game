# Feature Parity Gap Removal

**Purpose:** regain functionality and UX that existed before (or is promised by docs) while preserving the new hexagonal/ports-based crate structure.

**Non-goals (for this phase):**
- Designing separate desktop vs mobile UI layouts
- Adding new gameplay beyond documented scope
- Relaxing architecture rules to “make it work”

## Constraints (must stay true)

- `cargo xtask arch-check` must pass.
- Ports ownership is strict:
  - `wrldbldr-engine-ports` and `wrldbldr-player-ports` own inbound/outbound port traits.
  - App crates do not define their own ports modules.
- No shim paths:
  - No cross-crate re-exports (`pub use` / `pub(crate) use` / `pub(super) use`).
  - No crate aliasing (`use wrldbldr_* as ...`).

## How to use this document

Each gap below should be addressed by:
1. Confirming current behavior in code.
2. Picking the minimal architecture-respecting implementation path.
3. Adding/adjusting DTOs in the owning layer (typically `*-app`), not in UI/adapters.
4. Verifying with `cargo check --workspace` and `cargo xtask arch-check`.

## Source of truth for “expected features”

- `docs/progress/MVP.md`
- `docs/progress/ACTIVE_DEVELOPMENT.md`
- System specs under `docs/systems/*.md`

If a system doc checkbox conflicts with `ACTIVE_DEVELOPMENT.md` “Completed” claims, treat it as a documentation bug and record it in the **Doc Drift** section.

---

## Highest Priority Gaps (blocking core loop / major regressions)

### GAP-UI-INV-001: Inventory equip/unequip toggle not wired

- **Doc basis:** `docs/progress/ACTIVE_DEVELOPMENT.md` claims US-CHAR-009 is completed; inventory actions are part of the story.
- **Current state:** `InventoryPanel` renders but callbacks are not provided.
- **Evidence:** `crates/player-ui/src/presentation/views/pc_view.rs:627` uses `on_toggle_equip: None`.
- **Likely fix location:**
  - UI: `crates/player-ui/src/presentation/views/pc_view.rs`
  - Ports: extend `wrldbldr-player-ports` `GameConnectionPort` or reuse existing `PlayerAction` message if supported.
  - Engine: ensure there is a websocket message / command to equip/unequip.
- **Acceptance test:** toggling equip updates inventory display (equipped tab/filter, item state) after server confirms.

### GAP-UI-INV-002: Inventory “drop item” not wired

- **Doc basis:** same as GAP-UI-INV-001.
- **Current state:** callback not provided.
- **Evidence:** `crates/player-ui/src/presentation/views/pc_view.rs:628` uses `on_drop_item: None`.
- **Approach:**
  - Prefer a single “player action” command path if the protocol already has “drop”.
  - Otherwise add a port method on `GameConnectionPort` and implement in adapters.
- **Acceptance test:** dropped item disappears from inventory and optionally appears as a scene/world event.

### GAP-UI-NAV-001: Mini-map background image not wired

- **Doc basis:** navigation system mini-map should show location map (docs mention map image overlay).
- **Current state:** the mini-map supports `map_image`, but nothing supplies it.
- **Evidence:** `crates/player-ui/src/presentation/views/pc_view.rs:659` sets `map_image: None`.
- **Likely fix location:**
  - Add map image URL/path to the location/region DTO the UI already consumes.
  - Ensure the engine provides it in `SceneChanged` / relevant snapshot.
- **Acceptance test:** mini-map shows provided image on supporting locations.

### GAP-DM-CHAL-001: Ad-hoc challenge creation modal not wired

- **Doc basis:** DM tools are part of Phase C; however the modal is present and implies functionality.
- **Current state:** UI modal logs a warning and does not call the backend.
- **Evidence:** `crates/player-ui/src/presentation/views/dm_view.rs:110`.
- **Approach:**
  - Add a method to `wrldbldr-player-ports::outbound::GameConnectionPort` (or reuse an existing command) for ad-hoc challenge creation.
  - Implement in `wrldbldr-player-adapters` websocket client.
  - Ensure protocol message exists in `wrldbldr-protocol`.
- **Acceptance test:** DM can create ad-hoc challenge, player receives roll prompt / challenge event.

---

## Medium Priority Gaps (engine correctness / approvals / persistence)

### GAP-ENG-AST-001: Asset generation queue does not create asset records

- **Doc basis:** asset system expects generated assets to persist.
- **Current state:** queue marks request complete after sleep; does not download/store images.
- **Evidence:** `crates/engine-app/src/application/services/asset_generation_queue_service.rs:142`.
- **Approach:**
  - Implement downloading results from ComfyUI history.
  - Create assets in `AssetRepositoryPort` and associate with entity.
  - Ensure errors fail the queue item with a useful message.
- **Acceptance test:** generated images become visible in asset gallery and survive restart.

### GAP-ENG-CHAL-002: Challenge outcome triggers not executed

- **Doc basis:** challenge system includes unlocks and trigger effects.
- **Current state:** service broadcasts resolution but does not parse/execute triggers.
- **Evidence:** `crates/engine-app/src/application/services/challenge_outcome_approval_service.rs:349`.
- **Approach:**
  - Convert `ProposedToolInfo` outcome trigger representation back into domain `OutcomeTrigger`.
  - Execute triggers via existing tool execution paths.
- **Acceptance test:** choosing an outcome with unlock triggers changes world state.

### GAP-ENG-SCN-001: Scene update not broadcast on character endpoints

- **Doc basis:** player should see updated scene/region when server changes.
- **Current state:** route TODO indicates missing broadcast.
- **Evidence:** `crates/engine-adapters/src/infrastructure/http/player_character_routes.rs:258` and `:379`.
- **Acceptance test:** when server changes player’s scene, connected client receives the update.

---

## UX / Polish Gaps (non-blocking)

### GAP-UI-STATE-001: Observation/map/selected PC state updates are TODO

- **Evidence:**
  - `crates/player-ui/src/presentation/handlers/session_message_handler.rs:638`
  - `crates/player-ui/src/presentation/handlers/session_message_handler.rs:664`
- **Risk:** UI can drift out-of-sync after server messages.
- **Acceptance test:** switching PCs and receiving observation updates updates relevant panels.

### GAP-DIR-UX-001: “View-as-character” and location preview placeholders

- **Evidence:**
  - `crates/player-ui/src/presentation/views/director/content.rs:323`
  - `crates/player-ui/src/presentation/views/director/content.rs:364`
- **Approach:** scope to minimal “open modal” functionality first.

---

## Doc Drift / Inconsistencies (fix docs or align checklists)

These are not necessarily code gaps, but they will cause confusion when tracking parity.

- `docs/progress/ACTIVE_DEVELOPMENT.md` marks US-NAV-008/009/010 and US-CHAR-009 as completed in its “Completed” section, but `docs/systems/navigation-system.md` and `docs/systems/character-system.md` still list them as Pending.
  - Likely action: update system docs to reflect completion *or* mark completion section as outdated.

---

## Suggested execution order

1. Wire inventory equip/drop (small UI+protocol changes, high user value).
2. Wire mini-map image (mostly DTO/message plumbing).
3. Wire ad-hoc challenge creation (ports + adapter + protocol).
4. Fix engine side trigger execution and asset queue persistence.

## Verification checklist (run inside Nix shell)

- `cargo xtask arch-check`
- `cargo check --workspace`
- Optional: `cargo check -p wrldbldr-player --target wasm32-unknown-unknown`
