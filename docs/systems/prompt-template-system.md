# Prompt Template System

## Overview

The Prompt Template System provides configurable LLM prompts for all AI-powered features in WrldBldr. It enables world-specific customization of NPC dialogue styles, staging decisions, challenge outcome descriptions, and worldbuilding suggestions without code changes.

Templates are resolved with priority: **World DB > Global DB > Environment Variable > Hard-coded Default**

---

## Game Design

This system enables:
- **Per-world customization**: Each world can have its own prompting style (grimdark, whimsical, etc.)
- **Runtime tuning**: DMs can adjust LLM behavior through the settings UI
- **Environment overrides**: Developers can test prompt changes without database modifications
- **Safe defaults**: Hard-coded fallbacks ensure the system works out-of-the-box

---

## User Stories

### Implemented

- [x] **US-PT-001**: As a DM, I can customize NPC dialogue response format so that NPCs speak in my world's style
  - *Implementation*: `PromptTemplateService` resolves `dialogue.response_format` per world
  - *Files*: `crates/engine/src/entities/prompt_template.rs`

- [x] **US-PT-002**: As a DM, I can customize staging decision prompts so that NPC presence logic matches my world
  - *Implementation*: `StagingService` resolves staging templates before LLM calls
  - *Files*: `crates/engine/src/entities/staging.rs`

- [x] **US-PT-003**: As a DM, I can customize challenge outcome descriptions so that successes/failures match my tone
  - *Implementation*: `OutcomeSuggestionService` resolves outcome templates
  - *Files*: `crates/engine/src/entities/challenge.rs`

- [x] **US-PT-004**: As a DM, I can customize worldbuilding suggestion prompts for character/location generation
  - *Implementation*: `SuggestionService` resolves 10 different suggestion templates
  - *Files*: `crates/engine/src/entities/suggestion.rs`

- [x] **US-PT-005**: As an admin, I can set global template overrides that apply to all worlds
  - *Implementation*: REST API for global template CRUD
  - *Files*: `crates/engine/src/api/http.rs`

- [x] **US-PT-006**: As a DM, I can set world-specific template overrides that only apply to my world
  - *Implementation*: REST API for per-world template CRUD
  - *Files*: `crates/engine/src/api/http.rs`

### Pending

- [ ] **US-PT-007**: As a DM, I can view and edit prompt templates through the settings UI

---

## Template Categories

| Category | Keys | Purpose |
|----------|------|---------|
| Dialogue | `dialogue.*` | NPC response format, challenge suggestions, narrative event suggestions |
| Staging | `staging.*` | NPC presence decisions, rule override instructions |
| Outcomes | `outcome.*` | Challenge result descriptions, branching outcome generation |
| Suggestions | `suggestion.*` | Worldbuilding content generation (names, descriptions, etc.) |

---

## Template Keys

### Dialogue System

| Key | Description |
|-----|-------------|
| `dialogue.response_format` | Response format instructions for NPC dialogue |
| `dialogue.challenge_suggestion_format` | Format for suggesting skill challenges |
| `dialogue.narrative_event_format` | Format for suggesting narrative events |

### Staging System

| Key | Description |
|-----|-------------|
| `staging.system_prompt` | System prompt for staging decisions |
| `staging.role_instructions` | Instructions explaining the LLM's staging role |
| `staging.response_format` | Expected JSON response format |

### Outcome System

| Key | Description |
|-----|-------------|
| `outcome.system_prompt` | System prompt for outcome descriptions |
| `outcome.branch_system_prompt` | System prompt for branching outcomes (uses `{branch_count}` placeholder) |

### Suggestion System

| Key | Description |
|-----|-------------|
| `suggestion.character_name` | Character name generation |
| `suggestion.character_description` | Physical description generation |
| `suggestion.character_wants` | Character motivation generation |
| `suggestion.character_fears` | Character fear generation |
| `suggestion.character_backstory` | Backstory generation |
| `suggestion.location_name` | Location name generation |
| `suggestion.location_description` | Location description generation |
| `suggestion.location_atmosphere` | Atmosphere/mood generation |
| `suggestion.location_features` | Notable features generation |
| `suggestion.location_secrets` | Hidden secrets generation |

---

## Resolution Priority

When resolving a template for a world:

1. **World DB Override**: Check `world_prompt_templates` table for world-specific value
2. **Global DB Override**: Check `prompt_templates` table for global value
3. **Environment Variable**: Check `WRLDBLDR_PROMPT_{KEY}` (e.g., `WRLDBLDR_PROMPT_DIALOGUE_RESPONSE_FORMAT`)
4. **Default**: Use hard-coded default from `domain/value_objects/prompt_templates.rs`

---

## API

### REST Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | `/api/prompt-templates` | List all templates with metadata | ✅ |
| GET | `/api/prompt-templates/global` | List global overrides | ✅ |
| PUT | `/api/prompt-templates/global/{key}` | Set global override | ✅ |
| DELETE | `/api/prompt-templates/global/{key}` | Delete global override | ✅ |
| GET | `/api/prompt-templates/world/{world_id}` | List world overrides | ✅ |
| PUT | `/api/prompt-templates/world/{world_id}/{key}` | Set world override | ✅ |
| DELETE | `/api/prompt-templates/world/{world_id}/{key}` | Delete world override | ✅ |
| GET | `/api/prompt-templates/resolve/{key}` | Resolve template (global only) | ✅ |
| GET | `/api/prompt-templates/resolve/{key}?world_id={id}` | Resolve template for world | ✅ |

---

## Data Model

### SQLite Tables

```sql
-- Global overrides
CREATE TABLE prompt_templates (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Per-world overrides
CREATE TABLE world_prompt_templates (
    world_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (world_id, key)
);
```

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| Domain Types | ✅ | - | `prompt_templates.rs` |
| Port Trait | ✅ | - | `PromptTemplateRepositoryPort` |
| SQLite Repository | ✅ | - | `prompt_template_repository.rs` |
| Service | ✅ | - | `PromptTemplateService` |
| HTTP Routes | ✅ | - | REST API |
| Dialogue Integration | ✅ | - | `LLMService`, `PromptBuilder` |
| Staging Integration | ✅ | - | `StagingService` |
| Outcome Integration | ✅ | - | `OutcomeSuggestionService` |
| Suggestion Integration | ✅ | - | `SuggestionService` |
| Settings UI | - | ⏳ | Pending |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/value_objects/prompt_templates.rs` | Template keys, defaults, metadata |
| Infrastructure | `crates/engine/src/infrastructure/ports.rs` | Repository port trait |
| Entity | `crates/engine/src/entities/prompt_template.rs` | Resolution logic with caching |
| Infrastructure | `crates/engine/src/infrastructure/sqlite/prompt_template_repo.rs` | SQLite implementation |
| API | `crates/engine/src/api/http.rs` | REST endpoints |

### Integrated Services

| Service | File | Templates Used |
|---------|------|----------------|
| LLM Entity | `crates/engine/src/entities/llm.rs` | `dialogue.*` |
| PromptBuilder | `crates/engine/src/use_cases/conversation/prompt_builder.rs` | `dialogue.*` |
| Staging Entity | `crates/engine/src/entities/staging.rs` | `staging.*` |
| Challenge Entity | `crates/engine/src/entities/challenge.rs` | `outcome.*` |
| Suggestion Entity | `crates/engine/src/entities/suggestion.rs` | `suggestion.*` |

---

## Placeholder Substitution

Suggestion templates support placeholders that are replaced at runtime:

| Placeholder | Source |
|-------------|--------|
| `{entity_type}` | `SuggestionContext.entity_type` |
| `{entity_name}` | `SuggestionContext.entity_name` |
| `{world_setting}` | `SuggestionContext.world_setting` |
| `{hints}` | `SuggestionContext.hints` |
| `{additional_context}` | `SuggestionContext.additional_context` |
| `{branch_count}` | Passed to `OutcomeSuggestionService.generate_branches()` |

---

## Environment Variable Override

Any template can be overridden via environment variable:

```bash
# Format: WRLDBLDR_PROMPT_{KEY_WITH_UNDERSCORES}
export WRLDBLDR_PROMPT_DIALOGUE_RESPONSE_FORMAT="Custom format instructions..."
export WRLDBLDR_PROMPT_STAGING_SYSTEM_PROMPT="You are a helpful assistant..."
```

Environment variables take precedence over defaults but are overridden by database values.

---

## Related Systems

- **Depends on**: None (foundational service)
- **Used by**: [Dialogue](./dialogue-system.md), [Staging](./staging-system.md), [Challenge](./challenge-system.md), [Asset](./asset-system.md)

---

## Revision History

| Date | Change |
|------|--------|
| 2024-12-24 | Initial implementation complete |
