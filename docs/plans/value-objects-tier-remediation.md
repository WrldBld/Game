**Created:** January 26, 2026
**Status:** In progress
**Owner:** OpenCode
**Scope:** Value object tier remediation (Tier-2 vs Tier-4)

---

## Findings (verbatim)

- `RollResult` stores derived `passed` but allows inconsistent states (staging_context.rs)
- `ActivationEvaluation` has `validate()` but public fields allow invalid states (activation_rules.rs)
- `ExpressionConfig` has implicit invariant (default must exist) not enforced (expression_config.rs)
- `ArchetypeChange` is a simple record and should be Tier‑4

---

## Plan

### Backend (domain-only)
- [x] **RollResult**: make fields private, compute `passed` in constructor or method; remove ability to set inconsistent state.
- [x] **ActivationEvaluation**: make fields private and enforce invariants in constructors (`resolved`, `needs_llm`, `with_llm_result`), remove public fields.
- [x] **ExpressionConfig**: enforce default expression invariant (validated composite). Constructors return `Result<Self, DomainError>` and validate `default_expression` exists.
- [x] **ArchetypeChange**: de‑encapsulate to Tier‑4 (public fields, remove trivial accessors if any).

### Validation
- [x] Add/adjust unit tests for RollResult and ActivationEvaluation invariants
- [x] Add unit tests for ExpressionConfig validation
- [ ] Ensure compile checks clean across engine/player/shared

---

## Progress

- [x] Backend refactor complete
- [x] Tests/validation complete
