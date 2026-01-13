# Code Review Checklist

Quick reference for code reviewers. Check applicable items before approving.

## Architecture

- [ ] No `use_cases` importing from `api` layer
- [ ] No `domain` importing from `engine`
- [ ] Protocol types only in API layer (not in use cases or entities)
- [ ] New port traits added to `infrastructure/ports.rs` (not scattered)
- [ ] Entities call entities directly (no unnecessary abstraction)

## Error Handling

- [ ] Errors have context (entity type, ID, operation name)
  ```rust
  // Good: RepoError::not_found("Character", id)
  // Bad:  RepoError::NotFound
  ```
- [ ] No silent `.unwrap()` on fallible operations
- [ ] Errors logged with tracing before propagation (where appropriate)
- [ ] User-facing errors don't expose internal details

## Database (Neo4j)

- [ ] Cypher queries use parameters (no string interpolation)
  ```rust
  // Good: query("MATCH (c:Character {id: $id})").param("id", id)
  // Bad:  query(&format!("MATCH (c:Character {{id: '{}'}})", id))
  ```
- [ ] SAFETY comments for any `format!()` in queries (with justification)
- [ ] Indexes exist for frequently queried properties
- [ ] Relationships stored as edges (not JSON blobs)

## Testing

- [ ] New LLM calls have VCR cassettes (`cargo test` in record mode)
- [ ] Happy path and error cases covered
- [ ] No flaky timing dependencies
- [ ] Mock expectations are specific (not `.any()`)

## Memory & Performance

- [ ] Ephemeral state uses `TtlCache` (not unbounded HashMap)
- [ ] No unbounded `Vec` growth in loops
- [ ] Large allocations documented or bounded
- [ ] Async functions don't block the runtime

## Typed IDs

- [ ] Domain IDs used throughout (`CharacterId`, not raw `Uuid`)
- [ ] Parsing from strings handles errors
  ```rust
  // Good: CharacterId::try_from(uuid_str)?
  // Bad:  CharacterId::from(uuid_str.parse().unwrap())
  ```

## Dioxus (Player UI)

- [ ] Hooks called unconditionally at top of components
- [ ] No nested signal reads that could cause RefCell panics
- [ ] Event handlers don't capture Signal references across await points

## Documentation

- [ ] Public APIs have doc comments (what, not how)
- [ ] Complex logic has inline comments explaining why
- [ ] ADR created for significant architectural decisions

## Security

- [ ] No secrets in code or committed files
- [ ] User input validated at system boundaries
- [ ] SQL/Cypher injection not possible

---

## Quick Sanity Checks

```bash
# Build passes
cargo check --workspace

# Tests pass
cargo test --workspace

# Clippy clean
cargo clippy --workspace -- -D warnings

# E2E tests (if LLM code changed)
E2E_LLM_MODE=playback cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=1
```

---

## Common Issues

### Pattern: Missing Error Context

```rust
// Before: loses context
repo.get(id).await.map_err(|_| SomeError::NotFound)?;

// After: preserves context
repo.get(id).await.map_err(|e| SomeError::repo(e))?;
```

### Pattern: Unbounded Collection

```rust
// Before: can grow forever
let mut cache: HashMap<K, V> = HashMap::new();

// After: bounded with TTL
let cache = TtlCache::new(Duration::from_secs(3600));
```

### Pattern: String ID Handling

```rust
// Before: panics on invalid ID
let id = CharacterId::from(id_str.parse().unwrap());

// After: proper error handling
let id = id_str.parse::<Uuid>()
    .map(CharacterId::from)
    .map_err(|_| Error::InvalidId(id_str.to_string()))?;
```
