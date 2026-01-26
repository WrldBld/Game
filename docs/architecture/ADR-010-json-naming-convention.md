# ADR-010: Standardize JSON Naming Convention to snake_case

## Status

**ACCEPTED** - Pending implementation

## Context

The codebase has inconsistent JSON serialization naming:
- ~100 types use `#[serde(rename_all = "camelCase")]`
- ~100 types use `#[serde(rename_all = "snake_case")]`

This causes confusion and bugs, especially with nested types where enum variant names get renamed but inner struct fields don't.

Example of the problem:
```rust
#[serde(rename_all = "camelCase")]
enum TriggerType {
    DialogueTopic { topic_keywords: Vec<String> },  // variant renamed, field NOT renamed
}
```
Serializes to: `{"dialogueTopic": {"topic_keywords": [...]}}`

## Decision

**Standardize on snake_case for all JSON serialization.**

### Rationale

1. **Rust native** - No serde attributes needed for most types
2. **Consistent** - Both variant names and field names use same convention
3. **Less boilerplate** - Remove 200+ `rename_all` attributes
4. **Simpler mental model** - JSON matches Rust field names
5. **API boundary handling** - If JavaScript frontend needs camelCase, handle via DTOs at API layer

### Migration Strategy

1. **Phase 1**: Remove all `rename_all = "camelCase"` attributes
2. **Phase 2**: Remove `rename_all = "snake_case"` attributes (they're now default)
3. **Phase 3**: Update all JSON test fixtures and Neo4j stored data
4. **Phase 4**: Update any client code expecting camelCase

### Impact

- ~200 files need attribute changes
- Test fixture JSON files need updates
- Neo4j stored JSON blobs may need migration
- Client TypeScript types need regeneration

## Consequences

### Positive
- Single consistent convention
- Simpler code (less serde attributes)
- JSON matches Rust source exactly
- Easier debugging (field names match)

### Negative
- Large refactor effort
- JavaScript clients need to adapt or use transformation layer
- Existing stored data needs migration

## Implementation Tracking

See `docs/plans/JSON_NAMING_REFACTOR.md` for implementation progress.
