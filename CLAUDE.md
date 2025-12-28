# WrldBldr/Game – Coding Guide (CLAUDE)

This repo is a Rust workspace that enforces hexagonal architecture **by crate boundaries**.

## Environment (NixOS)

Run all commands inside the repo Nix shell:

- Enter shell: `nix-shell /home/otto/repos/WrldBldr/Game/shell.nix`
- Or one-shot: `nix-shell /home/otto/repos/WrldBldr/Game/shell.nix --run "cd /home/otto/repos/WrldBldr/Game && <cmd>"`

## Fast sanity checks (required)

- `cargo xtask arch-check` (must stay passing)
- `cargo check --workspace`

## Workspace layout (source of truth)

- Shared foundations
  - `crates/domain` (`wrldbldr-domain`): domain entities/value objects; **serde-free by default**.
  - `crates/protocol` (`wrldbldr-protocol`): wire DTOs for REST/WS; serialization-only.

- Engine (server)
  - `crates/engine-ports` (`wrldbldr-engine-ports`): **all engine ports** (inbound + outbound).
  - `crates/engine-app` (`wrldbldr-engine-app`): application services + DTOs.
  - `crates/engine-adapters` (`wrldbldr-engine-adapters`): Axum routes, websocket server, Neo4j repos, LLM clients, etc.
  - `crates/engine-runner` (`wrldbldr-engine-runner`): binary crate / composition root.

- Player (client)
  - `crates/player-ports` (`wrldbldr-player-ports`): **all player ports** (inbound + outbound) and shared cross-layer types.
  - `crates/player-app` (`wrldbldr-player-app`): application services + DTOs.
  - `crates/player-adapters` (`wrldbldr-player-adapters`): HTTP/WebSocket clients, platform adapters, storage, URL handling.
  - `crates/player-ui` (`wrldbldr-player-ui`): Dioxus UI; calls app services; no adapter construction.
  - `crates/player-runner` (`wrldbldr-player-runner`): composition-root crate (Dioxus launch + wiring). Produces the `wrldbldr-player` binary.

## Architecture constraints (strict)

### 1) Port ownership (two types)

**Infrastructure ports** (`*-ports` crates):
- Repository traits (`CharacterRepositoryPort`, `LocationRepositoryPort`)
- External service traits (`LlmPort`, `ComfyUIPort`, `BroadcastPort`)
- Connection/transport traits (`GameConnectionPort`)

**Use-case ports** (`*-app` crates, allowed):
- Service facade traits injected into use cases for dependency injection
- Use-case-specific abstractions that wrap infrastructure ports
- Example: `SceneServicePort`, `ChallengeResolutionPort`, `DmNotificationPort`

### 2) Protocol import rules

| Layer | Protocol Imports | Rationale |
|-------|------------------|-----------|
| domain | FORBIDDEN | Pure business logic |
| *-ports | FORBIDDEN (except API boundaries) | Infrastructure contracts |
| *-app/use_cases | FORBIDDEN | Business logic orchestration |
| *-app/services | Use app-layer DTOs, convert before calling port | Service layer isolation |
| *-app/handlers | ALLOWED | Boundary layer, converts protocol↔domain |
| *-adapters | ALLOWED | Implements wire-format conversion |
| player-ui | ALLOWED | Presentation boundary |

### 3) GameConnectionPort pattern

The `GameConnectionPort::request(payload: RequestPayload)` method is the correct design:
- Generic method handles all 118+ RequestPayload variants
- Individual methods exist only for operations needing special handling
- Services create app-layer DTOs, convert to RequestPayload before calling `request()`
- This is NOT a violation to fix

### 4) No shim import paths

Do not add "convenience" shims:

- No re-exports of `wrldbldr_*` from other crates/modules (`pub use`, `pub(crate) use`, `pub(super) use`).
- No crate aliasing (`use wrldbldr_* as foo;` or `extern crate wrldbldr_* as foo;`).

Goal: a single canonical import path for every type.

### 5) Composition roots own construction

- Binaries (and runner crates) may wire concrete adapters into services.
- UI must not construct infrastructure adapters.

## Feature parity work

- Primary tracking doc: `docs/progress/FEATURE_PARITY_GAP_REMOVAL.md`
- MVP acceptance criteria: `docs/progress/MVP.md`
- Current story status: `docs/progress/ACTIVE_DEVELOPMENT.md`

## Pointers

- Project-wide plans and specs are under `docs/`.
- Tooling/enforcement lives in `crates/xtask`.
