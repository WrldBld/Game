**Created:** January 26, 2026
**Status:** Complete
**Owner:** OpenCode
**Scope:** Post-remediation review fixes

---

## Findings (verbatim)

**Code Review**
- World export/import backward compatibility regression (format_version still 1)
- Time advance toast auto‑dismiss does not re‑arm
- New location defaults override world settings
- EndConversation tests still expect success on repo failure

**Architecture Review**
- Primitive IDs in domain value objects (world_state.rs)
- Boolean trigger state in NarrativeEvent hydration
- Raw UUID parameters in engine
- Error stringification in use cases

---

## Plan

### Backend
- [x] Bump world export format_version to 2 and fail fast on v1 (no legacy support)
- [x] Convert domain world_state IDs to typed IDs (CharacterId, PlayerCharacterId, ApprovalId)
- [x] Replace NarrativeEvent hydration bool with TriggerStatus or explicit enum
- [x] Replace raw UUID parameters in engine with typed IDs where applicable
- [x] Replace error stringification with typed errors in use cases
- [x] Update EndConversation tests to fail-fast expectations (if not already)

### Frontend
- [x] Fix time toast auto-dismiss re-arm (effect dependency)
- [x] Ensure location create defaults send None when using world defaults (0 → None)
- [x] Prompt template editor fetch effect dependency (if not already)

### Validation
- [x] Code review + architecture review
- [x] cargo check (engine/player/shared)
