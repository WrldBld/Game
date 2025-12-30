# ADR-001: UUID Generation in Domain Layer

## Status

Accepted

## Date

2024-12-30

## Context

The domain layer's `ids.rs` uses `Uuid::new_v4()` in the `define_id!` macro to generate unique identifiers for all 25+ entity ID types (e.g., `WorldId`, `CharacterId`, `ItemId`).

This technically introduces system entropy access (I/O) into the domain layer, which should be pure according to strict hexagonal architecture principles. The domain layer is expected to contain only pure business logic with no side effects.

## Decision

Accept `Uuid::new_v4()` in the domain layer as a pragmatic trade-off.

## Rationale

1. **No observable side effects** - UUID generation reads system entropy but doesn't modify system state or produce observable side effects beyond the returned value.

2. **Testability preserved** - All ID types provide a `from_uuid(uuid: Uuid)` method that allows tests to inject known UUID values for deterministic testing:
   ```rust
   let known_id = CharacterId::from_uuid(Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap());
   ```

3. **High refactoring cost, low value** - Refactoring 25+ ID types to use injected UUID factories would require:
   - Adding factory traits/parameters throughout the codebase
   - Modifying every entity constructor
   - Significant churn for minimal architectural benefit

4. **Industry standard** - Most hexagonal architecture implementations in Rust accept this pragmatic trade-off, as UUID generation is considered "effectively pure" due to its lack of observable side effects.

## Consequences

### Positive

- Simple, ergonomic ID creation with `XxxId::new()`
- No additional factory infrastructure needed
- Familiar pattern for developers

### Negative

- Domain layer is not 100% pure (contains entropy access)
- Purists may object to this trade-off

### Guidelines

- Tests requiring deterministic IDs should use `XxxId::from_uuid(known_uuid)`
- New ID types should follow the same pattern using the `define_id!` macro
- Do not add other I/O operations to the domain layer using this as precedent

## Alternatives Considered

### 1. UUID Factory Injection

Inject a UUID factory trait into all entity constructors:

```rust
pub trait UuidFactory: Send + Sync {
    fn new_uuid(&self) -> Uuid;
}

impl Character {
    pub fn new(name: String, uuid_factory: &dyn UuidFactory) -> Self {
        Self {
            id: CharacterId::from_uuid(uuid_factory.new_uuid()),
            // ...
        }
    }
}
```

**Rejected:** Too invasive, adds complexity throughout codebase for minimal benefit.

### 2. Move ID Generation to Adapters

Only allow `XxxId::from_uuid()` in domain, generate UUIDs in adapter layer.

**Rejected:** Would require changing all entity constructors and call sites, breaking ergonomics.

## References

- [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)
- [Domain-Driven Design](https://www.domainlanguage.com/ddd/)
- Rust `uuid` crate documentation
