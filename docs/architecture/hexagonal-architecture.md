# Hexagonal Architecture

> **Status**: SUPERSEDED
>
> This document has been replaced by the simplified architecture.
> See `AGENTS.md` in the project root for the current architecture.

## Summary

WrldBldr has moved from a complex hexagonal architecture (11+ crates, 128+ port traits) to a simplified structure:

- **4 crates**: domain, protocol, engine, player
- **~10 port traits**: Only for real infrastructure boundaries
- **No inbound ports**: Handlers call services directly
- **No internal traits**: Concrete types throughout

See [AGENTS.md](../../AGENTS.md) for full details.
