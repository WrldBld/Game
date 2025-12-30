# Plans - Claude Code Instructions

This directory contains project planning documents, roadmaps, and design specs for WrldBldr.

## Key Files

- `ROADMAP.md` - Main project roadmap with phases and implementation status
- `Hexagonal_refactor.md` - **CRITICAL** Architecture refactoring plan for hexagonal/DDD compliance
- Phase-specific plans in subdirectories

## Architecture Reference

WrldBldr follows **hexagonal architecture** (ports and adapters) with **domain-driven design**.

### Quick Reference

```
Domain ← Application ← Infrastructure
                    ← Presentation

INNER layers NEVER depend on OUTER layers
```

### Layer Rules (STRICTLY ENFORCED)

| Layer | Can Import | Cannot Import |
|-------|------------|---------------|
| Domain | (nothing) | application, infrastructure, presentation |
| Application | domain | infrastructure, presentation |
| Infrastructure | domain, application | presentation |
| Presentation | domain, application | infrastructure (use services!) |

### Before Writing Code

1. **Check `Hexagonal_refactor.md`** for current architecture violations and planned fixes
2. **Determine the correct layer** for your code using the file placement rules in Engine/Player CLAUDE.md
3. **Use port traits** instead of concrete infrastructure types in application services
4. **Route through services** - presentation calls services, services call repositories via ports

## Conventions

### ROADMAP.md Updates

When completing work on Engine or Player:

1. **Always update ROADMAP.md** with completion status
2. Use format: `- [x] Task name (Date completed: YYYY-MM-DD)`
3. Move completed phases to appropriate status section
4. Add notes about deferred items or known issues

### Phase Planning

- Each major feature should have a phase number (e.g., Phase 17: Story Arc)
- Break phases into sub-phases: A, B, C for backend; D, E, F, G for frontend
- Document dependencies between phases
- **Consider architecture impact** - which layer(s) does this feature touch?

### Architecture Documentation

When planning new features:

1. Identify which **domain entities** are involved
2. Determine if new **ports** are needed
3. Plan the **application service** that orchestrates the feature
4. Design the **infrastructure adapters** (API routes, repositories)
5. Plan the **presentation components** that consume the services

### Version Control

- Plans are tracked in git alongside code
- Commit plan updates with related code changes
- Use descriptive commit messages for plan changes

## Project Structure Reference

```
WrldBldr/
├── Engine/              # Rust backend (Axum + Neo4j)
│   └── CLAUDE.md        # Engine-specific architecture rules
├── Player/              # Rust frontend (Dioxus)
│   └── CLAUDE.md        # Player-specific architecture rules
└── plans/               # This directory
    ├── CLAUDE.md        # This file
    ├── ROADMAP.md       # Project roadmap
    └── Hexagonal_refactor.md  # Architecture refactoring plan
```

## Related Documentation

- **Engine architecture**: `Engine/CLAUDE.md`
- **Player architecture**: `Player/CLAUDE.md`
- **Refactoring plan**: `plans/Hexagonal_refactor.md`
- **Engine API routes**: `Engine/src/infrastructure/http/`
- **Player routes**: `Player/src/routes.rs`
- **Domain entities**: `Engine/src/domain/entities/`

## Architecture Violation Checklist

Before merging any code, verify:

### Engine
- [ ] No `use crate::infrastructure::` in `application/services/*.rs`
- [ ] No `use crate::infrastructure::` in `domain/**/*.rs`
- [ ] HTTP routes call services, not repositories directly
- [ ] Services use trait bounds, not concrete types

### Player
- [ ] No `use crate::infrastructure::` in `application/services/*.rs`
- [ ] No `use crate::infrastructure::` in `presentation/**/*.rs`
- [ ] No `use crate::presentation::` in `application/**/*.rs`
- [ ] Components call services, not HttpClient directly
- [ ] State types are domain/application types, not infrastructure DTOs
