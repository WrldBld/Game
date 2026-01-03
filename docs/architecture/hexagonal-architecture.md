# Hexagonal Architecture

> **Status**: SUPERSEDED
>
> This document has been replaced by the simplified architecture.
> See `docs/plans/SIMPLIFIED_ARCHITECTURE.md` for the current architecture.

## Summary

WrldBldr has moved from a complex hexagonal architecture (11+ crates, 128+ port traits) to a simplified structure:

- **4 crates**: domain, protocol, engine, player
- **~10 port traits**: Only for real infrastructure boundaries
- **No inbound ports**: Handlers call services directly
- **No internal traits**: Concrete types throughout

See [SIMPLIFIED_ARCHITECTURE.md](../plans/SIMPLIFIED_ARCHITECTURE.md) for full details.
