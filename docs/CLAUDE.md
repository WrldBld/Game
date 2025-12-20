# Documentation - Coding/Architecture Notes

This `docs/` directory contains planning docs, roadmaps, and technical specs for WrldBldr.

## Source Of Truth

- Coding + crate/layout rules: `../CLAUDE.md`
- Hexagonal enforcement refactor plan: `progress/HEXAGONAL_ENFORCEMENT_REFACTOR_MASTER_PLAN.md`
- Feature tracking: `progress/ACTIVE_DEVELOPMENT.md`, `progress/ROADMAP.md`, `progress/MVP.md`

## Architecture Reference (crate-based)

WrldBldr follows **hexagonal (ports & adapters)** architecture and enforces it primarily via a compile-time crate dependency DAG.

Quick mental model:

```
Domain  <--  Ports  <--  Application  <--  Adapters  <--  Runner (composition root)
                                  ^
                                  |
                               UI (player only)
```

Rules:
- Inner crates must not depend on outer crates.
- **Port traits only live in** `wrldbldr-engine-ports` / `wrldbldr-player-ports`.
- **No shim imports**: do not `pub use wrldbldr_*` from other crates and do not `use wrldbldr_* as alias`.
- Composition roots (`wrldbldr-engine-runner`, `wrldbldr-player-runner`) own adapter construction + wiring.

Enforcement:
- `cargo xtask arch-check` must stay passing (validates crate DAG + bans shim imports).

## Roadmap / Planning Conventions

When completing work:
- Update `progress/ROADMAP.md` and/or `progress/ACTIVE_DEVELOPMENT.md` status as appropriate.
- Use format `- [x] Task name (Date completed: YYYY-MM-DD)`.

## Doc Editing Note

Many older docs refer to legacy “Engine monolith” paths like `Engine/...` and `Player/...`. The real code lives under `../crates/*`.
