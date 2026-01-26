# ADR-011 Addendum: Protocol vs. Contracts Distinction

**Status:** Accepted
**Date:** January 21, 2026
**Supersedes:** Original ADR-011 section on shared crate imports
**Applies to:** All new code and architecture checks

---

## Problem

Original ADR-011 states: "Protocol/wire types should not leak into use case interfaces."

This rule has been applied too strictly, treating ALL types in `wrldbldr_shared` as "wire protocol", causing false positives such as `SettingsFieldMetadata` import in `settings_ops.rs`.

## Distinction

### Protocol Types (DISALLOWED in use cases)

These are **wire format representations** that should be converted at API boundary:

- `wrldbldr_shared::messages` - WebSocket messages
  - `ClientMessage`
  - `ServerMessage`
  - All message variants

- `wrldbldr_shared::requests` - Request DTOs
  - All request types sent from client

- `wrldbldr_shared::responses` - Response DTOs
  - All response types sent to client

- REST API request/response shapes

- Protocol enums that change frequently

**Rationale:** Protocol is volatile and represents serialization format. Business logic should not depend on wire format.

### Contract Types (ALLOWED in use cases)

These are **stable shared agreements** that both Engine and Player need to understand:

- `wrldbldr_shared::settings` - Schema metadata
  - `SettingsFieldMetadata`
  - `settings_metadata()` constructor

- `wrldbldr_shared::game_systems` - Game system traits and types
  - `GameSystem` trait
  - `CompendiumProvider` trait
  - Calculation engines

- `wrldbldr_shared::character_sheet` - Sheet schema definitions
  - `CharacterSheetSchema`
  - Field definitions and validation rules

- Other stable cross-runtime contracts

**Rationale:** These are application contracts, not transient wire protocol. They exist to be shared across runtime boundaries and represent stable domain understanding.

## Examples

### Allowed ✅

```rust
// In engine/src/use_cases/settings/settings_ops.rs:
use wrldbldr_shared::settings::settings_metadata;
use wrldbldr_shared::settings::SettingsFieldMetadata;

pub fn metadata(&self) -> Vec<SettingsFieldMetadata> {
    settings_metadata()  // Returns shared type directly
}
```

This is **ALLOWED** because `SettingsFieldMetadata` is in `wrldbldr_shared::settings` (contract module), not a wire protocol type.

### Disallowed ❌

```rust
// In engine/src/use_cases/some_use_case.rs:
use wrldbldr_shared::messages::ServerMessage;

pub fn execute(&self) -> Result<ServerMessage, Error> {
    // Returns wire protocol type directly from use case ❌
}
```

This is **DISALLOWED** because `ServerMessage` is a wire protocol type (WebSocket message). It should be constructed in API handler layer.

## Implementation Rules

For `cargo xtask arch-check`:

1. **Scan for imports from `wrldbldr_shared`**
2. **Classify imported module:**
   - If in `protocol_modules` allowlist → **VIOLATION**
   - If in `contracts_modules` allowlist → **ALLOWED**
   - If not in either → **WARNING**, manual review needed

3. **Module Allowlists:**

```rust
const PROTOCOL_MODULES: &[&str] = &[
    "messages",
    "requests",
    "responses",
];

const CONTRACTS_MODULES: &[&str] = &[
    "settings",
    "game_systems",
    "character_sheet",
];
```

## Testing

Run `cargo xtask arch-check` and verify:
- `settings_ops.rs` no longer flagged as violation
- Other legitimate contract imports are allowed
- Protocol imports (messages, requests, responses) are still caught

## References

- [ADR-011: Protocol Conversion Boundaries](ADR-011-protocol-conversion-boundaries.md)
- [Architecture Validation Results](../plans/architecture-validation-results.md)
