# Hexagonal Architecture Enforcement Refactor (Master Plan)

**Purpose**: This is the master plan for the "hexagonal enforcement" refactor. It defines the target crate layout, dependency rules, DTO/ID ownership, enforcement tooling, and the progress checklist.

**Goal**: Maximize **compile-time** enforcement of architecture boundaries by splitting the current large Engine and Player crates into smaller crates representing layers (domain, ports, application, adapters, UI).

**Status**: IN PROGRESS

**Last Updated**: 2025-12-20 (shim enforcement expanded: re-exports + crate aliases)

### Recent Work Notes
- Restored several engine files that were accidentally collapsed into single-line blobs during earlier scripted refactors.
- Cleaned up remaining `use ...; use ...` manglings and re-applied D6 imports (typed IDs from `wrldbldr_domain`, wire DTOs/enums from `wrldbldr_protocol`).
- Verified `nix-shell --run "RUSTFLAGS='-Awarnings' cargo check -p wrldbldr-engine-runner"` succeeds.
- Removed remaining serde derives from `crates/engine-app/src/domain/entities/observation.rs` to keep engine-domain serde-free (per D5).
- Updated D7 to reflect chrono-backed canonical GameTime + standardized TimeOfDay mapping prior to implementing B6–B8.
- Player split wiring: `wrldbldr-player-ui` now imports wire DTOs from `wrldbldr-protocol` (not `wrldbldr-player-app`), and `wrldbldr-player-adapters` no longer depends on UI routing types (deep links parse to an adapters-owned `DeepLink`).
- Fixed `RawApiPort for ApiAdapter` recursion by disambiguating calls via `ApiPort::...`.
- Player ports dedupe: removed the player-app “ports shim” directory and switched `wrldbldr-player-app` to import port traits from `wrldbldr-player-ports`.
- Protocol move: `AppEvent` now lives in `wrldbldr-protocol` (`wrldbldr_protocol::AppEvent`) and engine code imports it directly (per D6).
- Engine ports export unblocked: `WorldExporterPort` (and related structs) is exported from `wrldbldr-engine-ports`; world export/persistence now uses `wrldbldr_protocol::RuleSystemConfig` instead of engine-app DTOs.
- Verified `nix-shell --run "cargo check --workspace"` and `cargo xtask arch-check` succeed.
- Shim enforcement tightened: removed remaining cross-crate re-export shims and crate-alias shims (`use wrldbldr_* as ...`).
- `cargo xtask arch-check` now also detects:
  - `pub* use ::?wrldbldr_*` re-export shims
  - `use ::?wrldbldr_* as <alias>;` crate-alias shims
  - `extern crate ::?wrldbldr_* as <alias>;` crate-alias shims
  and reports the first offending line with its line number.
- Engine ports consolidation: removed the engine-app “ports shim” directory and converted remaining services to import port traits directly from `wrldbldr_engine_ports`.

---

## Decisions (Locked)

### D8. Player composition root
- Approved: `wrldbldr-player-runner` owns the Dioxus launch/composition root (wiring + `LaunchBuilder.launch(...)`).
- `wrldbldr-player-ui` stays presentation-only:
  - It must not construct infrastructure adapters.
  - It must not depend on `wrldbldr-player-adapters`.
  - It may create runtime-local state/contexts (Dioxus signals) inside components.

### D1. Base layer ownership
- **Domain is the base** (core meaning of the system).
- **Protocol is an external boundary format** (REST + WebSocket DTOs).

### D2. ID ownership
- **`wrldbldr-domain` owns typed IDs** (`WorldId`, `CharacterId`, etc.).
- **`wrldbldr-protocol` DTOs use raw `uuid::Uuid`** fields.
- Conversion is explicit at boundaries.

### D3. Protocol scope
- `wrldbldr-protocol` contains **all public API DTOs**:
  - WebSocket messages and message payload structs
  - REST request/response DTOs

Constraints:
- Protocol remains *serialization-only* (serde/uuid/chrono; no axum/sqlx/dioxus/etc).
- Protocol does **not** depend on domain.

### D4. Ports strategy
- Ports are split per app:
  - `wrldbldr-engine-ports`
  - `wrldbldr-player-ports`

### D5. Domain serialization
- Default: `wrldbldr-domain` is **serde-free**.
- Allow later, explicit exception via:
  - `wrldbldr-domain-serde` crate (preferred), or
  - feature-gated serde support (acceptable but less strict).

### D6. No shim imports (general policy; strict Option A)
- **Do not re-export or alias** `wrldbldr-*` crates from “convenience” modules in other crates (Engine/Player app/adapters/bins).
- Each file imports from the owning crate directly:
  - IDs from `wrldbldr_domain::...`
  - Wire DTOs from `wrldbldr_protocol::...`
  - Ports from `wrldbldr_engine_ports::...` / `wrldbldr_player_ports::...`

Rationale (why we avoid shims generally):
- **Single source of truth**: prevents ambiguous “where does this type live?” and stops drift during refactors.
- **Sharper boundaries**: shims hide dependencies; direct imports make layer boundaries visible in code review.
- **Better enforcement**: `cargo xtask arch-check` + grep-based checks work best when imports point at true owners.
- **Less churn later**: when a type moves crates, only direct call sites change; shims create “sticky” legacy paths.

Allowed exception (rare, requires explicit justification):
- A crate may re-export *its own* internal modules for ergonomics (e.g. `wrldbldr_engine_ports::outbound::*`), but other crates must not create additional alias layers like `engine-app::application::ports::*` or crate-alias shims like `use wrldbldr_protocol as messages;`.

### D7. Game time ownership + payload policy
- Canonical `GameTime` lives in `wrldbldr-domain` (serde-free).
  - Current canonical internal representation is `chrono`-backed for now to preserve existing behavior:
    - `current: chrono::DateTime<Utc>`
    - `is_paused: bool`

- `wrldbldr-protocol` contains the serialized wire representation.
  - `wrldbldr_protocol::GameTime` is a boundary struct with:
    - `{ day: u32, hour: u8, minute: u8, is_paused: bool }`
  - Protocol must remain *serialization-only* (serde/uuid/chrono; no UI/axum/sqlx/etc).
  - Protocol must not provide UI-facing formatting or display helpers.

- WebSocket/HTTP must send **structured `GameTime` only**.
  - No `display` strings
  - No standalone `time_of_day` strings/enums

- Player UI derives time-of-day + formatting from structured `GameTime`.
  - `TimeOfDay` is UI-local.
  - The time-of-day mapping is standardized:
    - `5..=11 => Morning`, `12..=17 => Afternoon`, `18..=21 => Evening`, `else => Night`
  - Current display uses ordinal-style output (future customizable calendar/settings planned).

---

## Target Workspace Layout

### Core crates
- `crates/domain` → `wrldbldr-domain`
- `crates/protocol` → `wrldbldr-protocol` (expanded)

### Port crates
- `crates/engine-ports` → `wrldbldr-engine-ports`
- `crates/player-ports` → `wrldbldr-player-ports`

### Engine crates
- `crates/engine-app` → `wrldbldr-engine-app` (use-cases / application services)
- `crates/engine-adapters` → `wrldbldr-engine-adapters` (http/ws/db/clients/queues)
- `crates/engine-runner` → `wrldbldr-engine-runner` (bin / composition root only)

### Player crates
- `crates/player-app` → `wrldbldr-player-app` (application services)
- `crates/player-adapters` → `wrldbldr-player-adapters` (http/ws/platform)
- `crates/player-ui` → `wrldbldr-player-ui` (Dioxus presentation + routes + state)
- `crates/player-runner` → `wrldbldr-player-runner` (composition root; produces `wrldbldr-player` binary)

---

## Dependency Rules (Compile-Time Enforced)

### Allowed crate dependencies (DAG)

Core:
- `domain` → *(no internal deps)*
- `protocol` → *(no internal deps)*

Ports:
- `engine-ports` → `domain`, `protocol`
- `player-ports` → `domain`, `protocol`

Engine:
- `engine-app` → `domain`, `protocol`, `engine-ports`
- `engine-adapters` → `engine-app`, `engine-ports`, `protocol`
- `engine-runner` (bin) → `engine-adapters`

Player:
- `player-app` → `domain`, `protocol`, `player-ports`
- `player-adapters` → `player-app`, `player-ports`, `protocol`
- `player-ui` → `player-app`, `player-ports`, `protocol`
- `player-runner` (composition root; produces `wrldbldr-player` bin) → `player-ui`, `player-adapters`

### Forbidden dependencies (examples)
- `domain` must not depend on `protocol`, `axum`, `sqlx`, `dioxus`, etc.
- `engine-app` must not depend on adapter crates or adapter libraries (axum/sqlx/neo4rs/reqwest/etc).

---

## DTO / Mapping Conventions

### Protocol DTO style
- Protocol DTOs use raw `uuid::Uuid`.
- Protocol enums/structs are serde-friendly.

### Boundary mapping
- Mapping occurs at the outer edge of the app layer:
  - Engine request handler (http/ws) maps protocol DTOs to domain IDs and domain types.
  - App services operate primarily on domain types / domain IDs.

### Persistence
- Adapters use persistence-specific records:
  - `wrldbldr-engine-adapters::persistence::records::*`
- Persistence structs can derive serde if needed for DB interactions, but they stay in adapters.

---

## Enforcement Tooling

### T1. Compile-time enforcement (primary)
- Architecture boundaries are enforced by crate dependencies.

### T2. `xtask` architecture checks (secondary)
Add `crates/xtask` providing:
- `cargo xtask arch-check`: validate allowed dependency DAG using `cargo metadata`.

### T3. `cargo-deny` bans (optional follow-up)
Add bans to prevent heavy deps appearing in `domain` and `protocol`.

### T4. Module-level enforcement (optional follow-up)
If needed later:
- add a module import checker (via `cargo-modules` or custom scan) within adapter crates.

---

## Execution Plan + Progress Checklist

This refactor is executed as a single coordinated change-set on the refactor branch.

### Phase A — Scaffolding (safe, mechanical)
- [ ] A1. Add new crates (domain/protocol/ports/app/adapters/ui/xtask)
- [ ] A2. Update workspace root `Cargo.toml` members and workspace deps
- [ ] A3. Update `.cargo/config.toml` aliases for `xtask`

### Phase B — Move shared types
- [x] B1. Move typed IDs into `wrldbldr-domain`
- [x] B2. Update `wrldbldr-protocol` to use raw `Uuid`
- [x] B3. Fix queue DTOs + boundary UUID conversions
- [x] B4. Remove serde derives from domain types containing IDs
- [x] B5. Remove all domain/protocol re-export shims (strict Option A) (Engine: no `crate::domain::value_objects::*Id` / no protocol re-exports)
- [x] B6. Canonicalize `GameTime` in `wrldbldr-domain` (serde-free)
- [x] B7. Refactor WS/HTTP to send structured `GameTime` only
- [x] B8. Update Player UI to derive `TimeOfDay` + format display

### Phase C — Ports extraction
- [x] C1. Move Engine ports → `wrldbldr-engine-ports` *(done: engine-ports owns inbound+outbound; engine-app ports shim removed; services import ports directly)*
- [x] C2. Move Player ports → `wrldbldr-player-ports` *(done: single source of port traits; `player-app` ports module removed)*
- [x] C3. Create `wrldbldr-core-ports` only if we find truly shared ports (removed for now; can be reintroduced later)

### Phase D — Engine split
- [x] D1. Move application services → `wrldbldr-engine-app`
- [x] D2. Move infrastructure/adapters → `wrldbldr-engine-adapters`
- [x] D3. Reduce `wrldbldr-engine-runner` to composition root

### Phase E — Player split
- [ ] E1. Move application services → `wrldbldr-player-app`
- [x] E2. Move infrastructure/adapters → `wrldbldr-player-adapters`
- [x] E3. Move presentation/routes/state → `wrldbldr-player-ui`
- [x] E4. Reduce `wrldbldr-player` to composition root

### Phase F — Enforcement + build validation
- [x] F1. Add `xtask arch-check` validating the crate dependency DAG
- [x] F2. Run `cargo check --workspace` (via `nix-shell`)
- [x] F3. Run `cargo check -p wrldbldr-engine-runner` / `wrldbldr-player` / `wrldbldr-protocol`
- [x] F4. Add a CI/local check to detect cross-crate shims in non-owner crates (`pub* use ::?wrldbldr_*`, `use wrldbldr_* as ...`, `extern crate wrldbldr_* as ...`), reporting file:line

Recent progress notes:
- B6 done: canonical `wrldbldr_domain::GameTime` + `TimeOfDay` (serde-free) is the engine source of truth.
- B7 done: engine HTTP (`/api/sessions/{id}/game-time`, `/api/sessions/{id}/game-time/advance`) and derived scene now return structured `wrldbldr_protocol::GameTime`; WS `GameTimeUpdated` and `StagingApprovalRequired` also use structured `GameTime`.
- Engine follow-up: `StagingContext` now stores `time_of_day` as `String` to keep domain `TimeOfDay` serde-free.
- Phase E (in progress): Bold copy of legacy player code into `wrldbldr-player-ui` / `wrldbldr-player-app` / `wrldbldr-player-adapters` / `wrldbldr-player-ports`; now rewiring imports so the new split crates compile, with composition rooted in `wrldbldr-player-runner`.

---

## Running the build (NixOS)

From `Game/`:

```bash
nix-shell --run "cargo check --workspace"
nix-shell --run "cargo check -p wrldbldr-engine-runner"
nix-shell --run "cargo check -p wrldbldr-player-runner --bin wrldbldr-player"
nix-shell --run "cargo check -p wrldbldr-protocol"
```
