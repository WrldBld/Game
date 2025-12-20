# Engine – Coding Guide (CLAUDE)

This is the **Engine** side of the workspace: an Axum + Neo4j backend with strict hexagonal boundaries enforced via crates.

## Environment (NixOS)

Run via the workspace shell:

- `nix-shell /home/otto/repos/WrldBldr/Game/shell.nix`

## Crates and boundaries

- `wrldbldr-engine` (`crates/engine`): **binary + composition root**
  - Builds the app state, wires adapters into services, starts the server.

- `wrldbldr-engine-adapters` (`crates/engine-adapters`): **infrastructure adapters**
  - Axum routes, websocket server, Neo4j repositories, LLM client adapters, export adapters.
  - Implements port traits defined in `wrldbldr-engine-ports`.

- `wrldbldr-engine-app` (`crates/engine-app`): **application services + DTOs**
  - Orchestrates domain logic and ports; no direct dependency on adapters.

- `wrldbldr-engine-ports` (`crates/engine-ports`): **ports + shared cross-layer types**
  - Sole owner of inbound/outbound port traits.

- Shared foundations used by engine crates:
  - `wrldbldr-domain` (`crates/domain`) – entities/value objects; serde-free by default.
  - `wrldbldr-protocol` (`crates/protocol`) – REST/WS DTOs; serialization-only.

## Strict rules

- **Ports ownership:** do not add `application::ports` shims in `wrldbldr-engine-app`.
- **No shim paths:** no cross-crate re-exports (`pub use ...wrldbldr_*`) and no crate aliasing (`use wrldbldr_* as ...`).
- **HTTP/websocket handlers call services:** routes should delegate to `wrldbldr-engine-app` services.

## Where code goes

- Port traits: `crates/engine-ports/src/inbound` and `crates/engine-ports/src/outbound`
- Application services/DTOs: `crates/engine-app/src/application/services` and `crates/engine-app/src/application/dto`
- Axum + websocket + persistence: `crates/engine-adapters/src/infrastructure/**`

## Running checks (required)

Inside Nix shell:

- `cargo xtask arch-check`
- `cargo check --workspace`
