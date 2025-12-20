# Player – Coding Guide (CLAUDE)

This is the **Player** side of the workspace: a Dioxus client with strict hexagonal boundaries enforced via crates.

## Environment (NixOS)

Run via the workspace shell:

- `nix-shell /home/otto/repos/WrldBldr/Game/shell.nix`

## Crates and boundaries

- `wrldbldr-player` (`crates/player`): **binary + composition root**
  - Creates concrete adapters (platform/api/ws) and passes them into the runner.

- `wrldbldr-player-runner` (`crates/player-runner`): **Dioxus launch + context wiring**
  - Owns `LaunchBuilder` setup and provides contexts.

- `wrldbldr-player-ui` (`crates/player-ui`): **presentation (UI)**
  - Dioxus views/components/state.
  - Calls `wrldbldr-player-app` services.
  - Must not construct adapters.

- `wrldbldr-player-app` (`crates/player-app`): **application services + DTOs**
  - Depends on ports from `wrldbldr-player-ports`.
  - Depends on shared foundations (`wrldbldr-domain`, `wrldbldr-protocol`) as needed.

- `wrldbldr-player-adapters` (`crates/player-adapters`): **infrastructure adapters**
  - HTTP client, websocket client, storage, platform implementations.
  - Implements traits from `wrldbldr-player-ports`.

- `wrldbldr-player-ports` (`crates/player-ports`): **ports + shared cross-layer types**
  - Sole owner of inbound/outbound port traits.

## Strict rules

- **Ports ownership:** do not add `application::ports` shims in `wrldbldr-player-app`.
- **No shim paths:** no cross-crate re-exports (`pub use ...wrldbldr_*`) and no crate aliasing (`use wrldbldr_* as ...`).
- **Construction happens at the edge:** UI consumes services; binary/runner wires dependencies.

## Where code goes

- Port traits: `crates/player-ports/src/inbound` and `crates/player-ports/src/outbound`
- Application DTOs/services: `crates/player-app/src/application/dto` and `crates/player-app/src/application/services`
- Adapters/platform/http/ws: `crates/player-adapters/src/infrastructure/**`
- UI state/components/views: `crates/player-ui/src/presentation/**`

## Running checks (required)

Inside Nix shell:

- `cargo xtask arch-check`
- `cargo check --workspace`

Optional WASM-only sanity:

- `cargo check -p wrldbldr-player --target wasm32-unknown-unknown`

## Common pitfalls

- UI importing adapters (`wrldbldr-player-adapters`) instead of ports/app.
- Introducing a second import path to the same type (re-export/alias shims).
- Adding platform-specific code outside adapters (prefer `player-adapters` platform module).
