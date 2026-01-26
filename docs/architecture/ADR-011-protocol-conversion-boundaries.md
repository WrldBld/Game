# ADR-011: Protocol Conversion Boundaries

## Status: Accepted

## Context

Code reviews have repeatedly flagged two patterns as potential architecture violations:

1. **`to_protocol()` methods on use case types** - Methods that convert use case result types to `wrldbldr_shared` wire format types
2. **`wrldbldr_shared` types used in use cases** - Shared types appearing in use case signatures

These appear to violate layer separation (use cases should not know about wire format), but deeper analysis reveals both patterns are architecturally correct and intentional.

## Decision

### Pattern 1: `to_protocol()` Methods on Use Case Types

**This is CORRECT.** Conversion helper methods may live on use case types.

**Why:**
- What matters is WHEN conversion happens, not WHERE the code lives
- These methods are CALLED from the API layer (handlers), so conversion happens at the correct boundary
- Co-locating conversion with the type improves encapsulation - the type controls its own serialization
- Moving methods to API layer would require exposing all internal fields via accessors

**Correct flow:**
```
[Use Case] returns domain type → [API Handler] calls .to_protocol() → [Wire format sent]
                                  ↑ conversion happens HERE (correct)
```

**Example (correct):**
```rust
// use_cases/staging/types.rs
impl StagedNpc {
    /// Convert to wire format for WebSocket response
    pub fn to_protocol(&self) -> wrldbldr_shared::StagedNpcInfo {
        StagedNpcInfo {
            id: self.id.to_string(),
            name: self.name.clone(),
            // ... fields
        }
    }
}

// api/websocket/ws_staging.rs - CALLER is in API layer
let response = staged_npc.to_protocol();  // Conversion at correct boundary
```

### Pattern 2: Shared Re-exports of Domain Types

**This is CORRECT.** The `shared` crate re-exports domain types for cross-crate availability.

**Why:**
- `shared` contains TWO categories of types:
  1. **Wire format types** - DTOs, WebSocket messages (need conversion at boundaries)
  2. **Shared vocabulary types** - Domain types re-exported for engine AND player to use
- Types like `CharacterSheetValues`, `SheetValue`, `GameTime` are domain types defined in `wrldbldr_domain`
- `shared` re-exports them so the Player crate can use them without depending on engine
- This is NOT a layer violation - it's the documented architecture for shared vocabulary

**Example (correct):**
```rust
// domain/src/types/character_sheet.rs - CANONICAL definition
pub struct CharacterSheetValues {
    pub values: BTreeMap<String, SheetValue>,
    pub last_updated: Option<DateTime<Utc>>,
}

// shared/src/lib.rs - RE-EXPORT for cross-crate use
pub use wrldbldr_domain::types::{CharacterSheetValues, SheetValue};

// engine/src/use_cases/... - Uses the type (this is fine!)
pub async fn execute(&self, sheet: CharacterSheetValues) -> Result<...>
```

### Pattern 3: `from_protocol()` Conversion Helpers

**This is CORRECT.** Input conversion helpers on use case input types.

**Why:**
- The helper is called from API handlers, so conversion happens at the correct boundary
- The use case internally works with domain types
- The helper is just a convenient factory method

**Example (correct):**
```rust
// use_cases/session/directorial.rs
impl DirectorialUpdateInput {
    /// Convert from wire format - called by API handler
    pub fn from_protocol(
        world_id: WorldId,
        proto: wrldbldr_shared::DirectorialContext,  // Wire type IN
    ) -> Self {
        Self {
            world_id,
            context: ports::DirectorialContext { ... },  // Domain type used internally
        }
    }
}

// api/websocket/ws_dm.rs - CALLER is in API layer
let input = DirectorialUpdateInput::from_protocol(world_id, wire_context);
let result = use_case.execute(input).await?;
```

## Anti-Pattern: Architecture Theater

The following changes would be "architecture theater" - they look like improvements but add complexity without benefit:

### DON'T: Move `to_protocol()` to API Layer
```rust
// WRONG - breaks encapsulation, no actual benefit
// api/converters.rs
pub fn staged_npc_to_protocol(npc: &StagedNpc) -> StagedNpcInfo {
    StagedNpcInfo {
        id: npc.id(),      // Now need public accessor
        name: npc.name(),  // Now need public accessor
        // Every field needs exposure
    }
}
```

**Problems:**
- API layer needs to know internal structure of use case types
- Requires adding public accessors for all fields
- If type changes, converter must change (coupling)
- Conversion still happens at same boundary - no architectural improvement

### DON'T: Duplicate Domain Types
```rust
// WRONG - pointless duplication
// Wanting to "not use shared types" by creating duplicates

// domain/src/types/sheet.rs
pub struct CharacterSheetValues { ... }  // Already exists!

// use_cases/types.rs
pub struct UseCaseSheetValues { ... }  // Duplicate!

impl From<CharacterSheetValues> for UseCaseSheetValues { ... }  // Boilerplate!
```

**Problems:**
- Two identical types for same concept
- Conversion boilerplate that just copies fields
- Maintenance burden (changes in two places)
- Contradicts the shared vocabulary pattern

## Consequences

### Positive
- Clear guidance on what IS and IS NOT a layer violation
- Prevents unnecessary refactoring that adds complexity
- Maintains encapsulation (types control their own conversion)
- Preserves single source of truth for shared vocabulary types

### Negative
- Reviewers must understand the distinction between:
  - Where conversion CODE lives (can be on the type)
  - Where conversion HAPPENS (must be at API boundary)
- `wrldbldr_shared` imports in use cases require case-by-case analysis

## How to Review

When reviewing code that uses `wrldbldr_shared` types in use cases:

1. **Is it a wire format type?** (DTOs, WebSocket messages)
   - Check: Is there a corresponding domain type?
   - Check: Is conversion happening at API boundary?
   - If yes to both: CORRECT

2. **Is it a re-exported domain type?** (CharacterSheetValues, GameTime, etc.)
   - Check: Is the canonical definition in `wrldbldr_domain`?
   - Check: Does `shared` just `pub use` it?
   - If yes to both: CORRECT (not a layer violation)

3. **Is it a use case RETURNING a shared type directly?**
   - This IS a violation - use cases should return domain types
   - The API layer should call `.to_protocol()` on the result

## Related

- [ADR-008: Tiered Encapsulation](ADR-008-tiered-encapsulation.md) - When to use public vs private fields
- [ADR-009: Repository Layer Elimination](ADR-009-repository-layer-elimination.md) - Avoiding unnecessary abstraction layers

## Updates

**January 21, 2026:** Added distinction between protocol types and contract types. See [ADR-011 Addendum: Protocol vs. Contracts Distinction](ADR-011-protocol-contracts-distinction.md) for details.
