# Neo4j Safe Fragment Policy

## Overview

Cypher queries in WrldBldr occasionally need to interpolate relationship types or structural query fragments because the Neo4j driver does not support parameterization of these elements. This is a limitation of Cypher itself - you cannot parameterize relationship types, labels, or property keys.

## Risk

String interpolation in queries can lead to Cypher injection vulnerabilities if user-controlled data is interpolated. For example:

```rust
// DANGEROUS - User input directly interpolated
let rel_type = user_input_relationship_type;  // Could be malicious!
let q = query(&format!("MATCH (a)-[:{rel_type}]->(b)"));
```

This could allow injection of arbitrary Cypher code.

## Mitigation

All Cypher string interpolation is centralized in `safe_fragments` modules within repository files. These modules:

1. Take enum types as input (e.g., `WantTargetRef`, `ActantialRole`)
2. Return only pre-defined static strings (e.g., "MATCH (target:Item {id: $target_id})")
3. Are fully tested to ensure all enum variants have safe fragment mappings
4. Use `&'static str` to guarantee compile-time constants

This approach ensures that **only enum variants** (which are compile-time known types) can be used to construct query fragments, preventing any dynamic or user-controlled data from being interpolated.

## Usage

### In `character_repo.rs`

The `safe_fragments` module provides two functions:

#### `want_target_match(&WantTargetRef) -> &'static str`

Returns the complete MATCH clause fragment for different target types:

```rust
use safe_fragments::want_target_match;

let target_match = safe_fragments::want_target_match(&target);

let q = query(&format!(
    "MATCH (w:Want {{id: $want_id}})
    {target_match}
    MERGE (w)-[:TARGETS]->(target)"
))
.param("want_id", want_id.to_string())
.param("target_id", target_id);
```

**Possible return values:**

| Input WantTargetRef | Output Fragment |
|-------------------|----------------|
| `WantTargetRef::Character(_)` | `MATCH (target) WHERE target.id = $target_id AND (target:Character OR target:PlayerCharacter)` |
| `WantTargetRef::Item(_)` | `MATCH (target:Item {id: $target_id})` |
| `WantTargetRef::Goal(_)` | `MATCH (target:Goal {id: $target_id})` |

#### `actantial_role_relationship(&ActantialRole) -> &'static str`

Returns the relationship type string for actantial views:

```rust
use safe_fragments::actantial_role_relationship;

let relationship_type = safe_fragments::actantial_role_relationship(&role);

let q = query(&format!(
    "MATCH (c:Character {{id: $id}})-[r:{relationship_type}]->(target)"
))
.param("id", character_id.to_string());
```

**Possible return values:**

| Input ActantialRole | Output Fragment |
|---------------------|-----------------|
| `ActantialRole::Helper` | `VIEWS_AS_HELPER` |
| `ActantialRole::Opponent` | `VIEWS_AS_OPPONENT` |
| `ActantialRole::Sender` | `VIEWS_AS_SENDER` |
| `ActantialRole::Receiver` | `VIEWS_AS_RECEIVER` |
| `ActantialRole::Unknown` | `VIEWS_AS_HELPER` |

## DOs and DON'Ts

### ✅ DO

```rust
// DO: Use enum types as input
let target = WantTargetRef::Character(char_id);
let fragment = safe_fragments::want_target_match(&target);

// DO: Use the fragment in format!
let q = query(&format!("MATCH (a)-[r:{fragment}]->(b)"));

// DO: Test all enum variants are covered
#[test]
fn test_all_variants() {
    // Test WantTargetRef variants
    assert!(!want_target_match(&WantTargetRef::Character(id)).is_empty());
    assert!(!want_target_match(&WantTargetRef::Item(id)).is_empty());
    assert!(!want_target_match(&WantTargetRef::Goal(id)).is_empty());

    // Test ActantialRole variants
    assert!(!actantial_role_relationship(&ActantialRole::Helper).is_empty());
    // ... etc
}
```

### ❌ DON'T

```rust
// DON'T: Use user input directly
let rel_type = user_input;  // DANGER!
let q = query(&format!("MATCH (a)-[:{rel_type}]->(b)"));

// DON'T: Construct dynamic fragments
let fragment = format!("TARGET_{}", user_suffix);  // DANGER!

// DON'T: Add new enum variant without updating safe_fragments
// If you add a new variant to WantTargetRef, you MUST update want_target_match
```

## Enforcement

1. **Compile-time safety:** Functions return `&'static str`, guaranteeing the returned strings are compile-time constants
2. **Comprehensive tests:** All enum variants are tested to ensure safe fragments exist
3. **Code review:** All `format!` calls in query construction are reviewed to ensure they only use safe_fragments output
4. **Architecture check:** `cargo xtask arch-check` validates that safe fragments are used correctly

## Adding New Relationship Types

If you need to add a new relationship type or query fragment:

1. **Add the enum variant** (if applicable)
2. **Update the safe_fragments function** to handle the new variant
3. **Add tests** for the new variant
4. **Document the new fragment** in this policy

Example:

```rust
// 1. Add to safe_fragments module
pub fn new_relationship_type(rel: &NewRelationshipType) -> &'static str {
    match rel {
        NewRelationshipType::SomeType => "SOME_RELATIONSHIP",
        NewRelationshipType::OtherType => "OTHER_RELATIONSHIP",
    }
}

// 2. Add tests
#[test]
fn test_new_relationship_types() {
    assert_eq!(
        new_relationship_type(&NewRelationshipType::SomeType),
        "SOME_RELATIONSHIP"
    );
}

// 3. Use in queries
let rel_type = safe_fragments::new_relationship_type(&rel);
let q = query(&format!("MATCH (a)-[:{rel_type}]->(b)"));
```

## References

- [ADR-011: Protocol Conversion Boundaries](ADR-011-protocol-conversion-boundaries.md)
- [Safe Fragments Implementation](../../crates/engine/src/infrastructure/neo4j/character_repo.rs#safe-fragments)
- [Neo4j Cypher Manual: Parameterized Queries](https://neo4j.com/docs/cypher-manual/current/syntax/parameters/)
