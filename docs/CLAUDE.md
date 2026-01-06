# Documentation - Coding/Architecture Notes

This `docs/` directory contains planning docs, roadmaps, and technical specs for WrldBldr.

## Source Of Truth

- Agent Guidelines + Architecture Rules: `../AGENTS.md`
- Feature tracking: `progress/ACTIVE_DEVELOPMENT.md`, `progress/ROADMAP.md`, `progress/MVP.md`

## Architecture Reference (crate-based)

WrldBldr follows a **simplified hexagonal architecture**:

- Hexagonal boundaries are for _infrastructure that might realistically be swapped_ (DB/LLM/queues/clock/random).
- Internal engine code calls internal engine code directly as concrete types.

Crates (current):

```
crates/
   domain/       # Pure business types (entities, value objects, typed IDs)
   protocol/     # Wire format for Engine <-> Player communication
   engine/       # All server-side code (including infra implementations)
   player-*/     # Client-side ports/app/adapters/ui/runner
```

Rules:

- **No shim imports**: do not `pub use wrldbldr_*` from other workspace crates and do not `use wrldbldr_* as alias`.
- Keep `cargo xtask arch-check` passing (crate DAG + shim checks).

## Roadmap / Planning Conventions

When completing work:

- Update `progress/ROADMAP.md` and/or `progress/ACTIVE_DEVELOPMENT.md` status as appropriate.
- Use format `- [x] Task name (Date completed: YYYY-MM-DD)`.

## Doc Editing Note

Many older docs refer to legacy “Engine monolith” paths like `Engine/...` and `Player/...`. The real code lives under `../crates/*`.
