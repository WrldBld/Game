# LLM Phased Pipeline Fix Plan

Status tracking for phased LLM pipeline stabilization and related hotfixes.

## Context
We introduced a phased NPC dialogue pipeline (Phase 0-5) to improve determinism and test stability. Reviews flagged correctness gaps (tool parsing, missing upstream context in prompts, deflection guard absence) plus a few related runtime risks. This plan tracks the remediation sequence and progress.

## Goals
- Restore functional tool intent handling (Phase 3).
- Ensure upstream phase outputs are actually consumed in later prompts.
- Reapply safety/deflection guard for secret motivations in phased path.
- Fail fast on corrupted phase outputs and enforce trigger rules.
- Keep prompts/parsers aligned with deterministic expectations.

## Work Items

### Phased LLM Pipeline (Critical/High)
- [x] Fix Phase 3 <arguments> parsing regex and add parser test.
- [x] Inject Phase 0 digest + Phase 1/2/3 outputs into phase prompts (1-5) where required.
- [x] Add deflection enforcement for phased path after Phase 5 parse (match legacy guard).
- [x] Enforce Phase 2 single-YES rule in parser (coerce or fail fast).
- [x] Fail fast on phase_outputs_json deserialization (no silent reset).
- [x] Avoid UTF-8 slicing in context digest truncation.
- [x] Include IDs in Phase 1/2 prompt contracts and parse IDs to avoid name-only matching.
- [x] Parse tool_analysis should_act + reasoning in Phase 3.
- [x] Parse disposition change reason in Phase 4.
- [x] Make Phase 0 topic extraction deterministic (sorted topics).
- [x] Apply per-phase template temperatures from front matter.
- [x] Align Phase 1 rules with approval flow (at most one YES).
- [x] Normalize player dialogue in Phase 0 digest (punctuation/whitespace).
- [x] Drop tools when Phase 3 should_act is NO.
- [x] Require deflection behavior when Phase 4 should_deflect is YES.
- [x] Mark phased queue items failed on parse errors.
- [x] Align deflection override reasoning/topics with final dialogue.
- [x] Align Phase 3 tool prompt with ToolArgs variants.
- [x] Fail Phase 3 when should_act is YES but no tools parsed.
- [x] Reject hallucinated challenge/event tags when no active items.

### Related Hotfixes (Non-LLM)
- [x] Guard missing world_id in staging regenerate handler (avoid unwrap panic).
- [x] Fix EventBus dispatch locking to avoid deadlocks (snapshot callbacks).
- [x] Log invalid asset paths instead of silently dropping.
- [x] Log invalid approval game_time values when parsing.
- [x] Log template resolution errors before fallback.
- [x] Log invalid optional UUID parsing in queue helpers.
- [x] Reject invalid activation rules on visual state updates.
- [x] Fail fast on invalid asset/atmosphere inputs during state updates.
- [x] Clamp day values in time conversion to avoid negative totals.

## Notes
- Branch: feature/llm-phased-pipeline
- Reviews: reports/phased_llm_code_review_48273.md, reports/phased_llm_arch_review_48327.md
