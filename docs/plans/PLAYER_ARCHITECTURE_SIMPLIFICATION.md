# Player Architecture Simplification (Draft)

Status: Draft
Owner: Team
Last updated: 2026-01-14

## Goal

Bring the Player side in line with the Engine’s simplified hexagonal rule:

- **Hexagonal architecture is for infrastructure boundaries only.**
- Internal code calls internal code directly as **concrete types**.
- Keep multi-platform support (Desktop + WASM) via thin adapter/transport wrappers.

This is not about removing portability — it’s about removing _unnecessary_ ports, factories, and “ports-of-ports” that exist mainly to enforce crate boundaries.

## Current State (What’s “too abstract”)

### 1) Ports crate is doing more than boundary ports

The player-ports crate defines large object-safe port traits that are used pervasively via `Arc<dyn ...>`.

- `GameConnectionPort` is object-safe and used everywhere in services.
- It is split into sub-ports (navigation/session/etc) largely to work around mocking/tooling constraints.

Net effect:

- lots of dynamic dispatch and cloning of trait objects
- “mockability” drives design more than runtime boundaries

### 2) Platform is a “god interface” and duplicated DI container logic

- `PlatformPort` bundles time/sleep/random/storage/logging/document/config + connection factory.
- player-adapters also contains a `Platform` DI container + a second layer of provider traits + dyn wrappers.

This creates:

- two conceptual “platform APIs”
- extra abstraction layers that don’t add swappability (desktop vs wasm already decided at compile-time)

### 3) HTTP boundary uses a two-tier port (`ApiPort` + `RawApiPort`) mainly for object safety

`ApiPort` is generic (not object-safe), so `RawApiPort` exists to store behind `Arc<dyn ...>`, then `Api` wraps and re-implements typed `ApiPort`.

This is an object-safety workaround caused by cross-crate layering, not a fundamental boundary.

### 4) Composition root (runner) exists mostly to bridge traits

player-runner builds:

- `Arc<dyn PlatformPort>`
- `Arc<dyn GameConnectionPort>`
- `Arc<dyn RawApiPort>`

and then passes them into Dioxus context.

## Target State (What we want)

### Principle: “Ports only for swap-able infra”

On Player, the realistic swap boundaries are small:

- Engine transport (WebSocket + reconnect + request/response correlation)
- HTTP client (if still separate)
- Storage (web localStorage vs desktop file)
- Clock/Random (mainly for testability)

Everything else should be concrete types within the Player crate.

### Shape: `player` crate looks like `engine` crate

Long-term ideal is to converge toward a single `crates/player/` (to match the documented 4-crate model), structured similarly to the Engine:

```
crates/player/
  src/
    app.rs
    use_cases/
    features/
    ui/                # Dioxus routes/components
    infrastructure/
      engine_transport/
        core.rs
        desktop.rs
        wasm.rs
      http/
      storage/
      clock.rs
      random.rs
```

#### “App” object

Analogous to Engine’s `App`, the Player has an `App` struct that holds concrete sub-systems:

- `engine: EngineClient` (concrete)
- `api: EngineApi` (concrete)
- `storage: Storage` (concrete)

UI receives `Arc<App>` via context.

### Ports that remain

Keep ports only if they are genuinely swappable _and_ needed for tests:

- `EngineTransportPort` (or keep it concrete and use `cfg` + fakes in tests)
- `StoragePort`
- `ClockPort`, `RandomPort` (optional)

Avoid `PlatformPort` as a grab-bag interface.

## Websocket-specific direction (“platform is only transport”)

We already started this:

- Core/shared logic extracted into reusable modules (`core.rs`, `shared.rs`), while desktop/wasm own the platform socket.

Next step:

- Define a minimal transport interface internally (not necessarily a public trait):
  - connect/open
  - send text frame
  - deliver inbound text frame to core
- Move buffering/state transitions into the core so both transports behave identically.

## Migration Roadmap (staged, low-risk)

### Phase 0: Audit + invariants (no behavior change)

- Identify which traits are actually needed as swap boundaries.
- Identify which are crate-boundary artifacts.

### Phase 1: Replace “PlatformPort” with concrete `Platform`

- Prefer `Arc<Platform>` in Dioxus context.
- Remove `PlatformPort` and the duplicated provider traits.
- Keep platform implementations as `cfg(wasm32)` / `cfg(not(wasm32))` modules.

Outcome: UI no longer forces a god trait + dyn dispatch for everything.

### Phase 2: Collapse GameConnection usage in services

- Replace `Arc<dyn GameConnectionPort>` flowing through every service.
- Introduce a concrete `EngineClient` facade with methods used by services.
- Keep a small test fake instead of mockall-driven trait splits.

Outcome: fewer trait objects, fewer layers, simpler call graph.

### Phase 3: Replace `ApiPort`/`RawApiPort` with object-safe, endpoint-typed API

- Make `EngineApi` expose concrete endpoint methods (no generic trait needed):
  - `list_worlds()`, `get_world(world_id)`, etc.
- Underneath, use a single platform HTTP adapter.

Outcome: no object-safety scaffolding.

### Phase 4: Crate consolidation

- Merge `player-ports` into `player-app` (or into a new unified `player` crate).
- Merge `player-ui` into the unified player crate.
- Keep `player-runner` thin or replace with `player/src/main.rs` if desired.

Outcome: player mirrors engine’s simplified architecture.

## Open Questions

- Do we want to enforce “4 crates only” strictly now (domain/protocol/engine/player), or do it incrementally?
- Do we keep some thin `ports` module for tests, or use concrete types + fakes?
- How much of the current `Platform` API is truly needed vs legacy?
