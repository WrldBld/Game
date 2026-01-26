**Created:** January 26, 2026
**Status:** Complete
**Owner:** OpenCode
**Scope:** Remediate architecture audit findings

---

## Findings (verbatim)

1) **Typed ID bypass in AssetRepo port**
   - `crates/engine/src/infrastructure/ports/repos.rs` uses raw `Uuid` in AssetRepo APIs.

2) **Protocol conversion coupling in use cases**
   - `crates/engine/src/use_cases/staging/types.rs` contains protocol formatting and conversion logic; risk of use-case layer absorbing wire/presentation concerns.

3) **Neo4j query duplication**
   - Repeated `MATCH (c:Character {id: $id})` across character/lore/staging repos.

---

## Plan (backend only)

### 1) AssetRepo typed IDs
- [x] Introduce typed owner ID wrapper (AssetOwnerId) and update AssetRepo signatures
- [x] Update neo4j asset_repo implementation + all call sites
- [x] Add/update tests for asset use cases

### 2) Protocol conversion relocation
- [x] Remove protocol formatting from use_cases/staging/types.rs
- [x] Move conversion to API layer (ws_staging/ws_movement)
- [x] Adjust tests if needed

### 3) Neo4j query helper extraction
- [x] Add helper fragments for common character matches
- [x] Update character/lore/staging repos to use helpers

---

## Progress

- [x] Backend remediation complete
- [x] Code review + architecture review
- [x] Checks/tests clean
