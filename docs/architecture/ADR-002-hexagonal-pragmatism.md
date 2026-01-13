# ADR-002: Pragmatic Hexagonal Architecture

## Status

Accepted

## Date

2026-01-13

## Context

WrldBldr started with a complex hexagonal architecture featuring:
- 11+ crates with strict layering
- 128+ port traits for every interface
- Inbound and outbound ports for all boundaries
- Extensive abstraction even for internal code

This created significant overhead:
- Trait proliferation made navigation difficult
- Most traits would never be swapped in practice
- Boilerplate obscured business logic
- Onboarding new contributors was challenging

## Decision

Adopt a **pragmatic hexagonal architecture** with:
- **4 crates**: domain, protocol, engine, player
- **~10 port traits**: Only for real infrastructure boundaries
- **No inbound ports**: Handlers call services directly
- **Concrete types internally**: No abstraction for internal code paths

Port traits are reserved for infrastructure that might realistically be swapped:

| Boundary | Trait | Swap Scenario |
|----------|-------|---------------|
| Database | `CharacterRepo`, `LocationRepo`, etc. | Neo4j -> Postgres |
| LLM | `LlmPort` | Ollama -> Claude/OpenAI |
| Image Generation | `ImageGenPort` | ComfyUI -> other |
| Queues | `QueuePort` | SQLite -> Redis |
| Clock/Random | `ClockPort`, `RandomPort` | Testing |

## Consequences

### Positive

- Dramatically simpler codebase (4 crates vs 11+)
- Easier navigation and understanding
- Reduced boilerplate
- Faster development velocity
- Still testable via ~10 mock-able port traits

### Negative

- Can't swap internal implementations without refactoring
- Less "pure" hexagonal architecture
- Harder to extract modules into separate services later

### Neutral

- Entity modules call entity modules directly (no internal traits)
- Use cases orchestrate entities without abstraction layers
- API handlers call use cases as concrete types

## Alternatives Considered

### 1. Full Hexagonal (Original Approach)

128+ port traits, strict layering, complete abstraction.

**Rejected:** Too much overhead for the swap scenarios that would actually occur. The cost of abstraction exceeded the benefit.

### 2. No Ports (Pure Concrete)

No port traits at all, direct infrastructure calls everywhere.

**Rejected:** Would make testing difficult and lock us into specific implementations for infrastructure we might actually want to swap (database, LLM).

## References

- [AGENTS.md](../../AGENTS.md) - Current architecture documentation
- [hexagonal-architecture.md](hexagonal-architecture.md) - Superseded detailed doc
