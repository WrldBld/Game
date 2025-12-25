# Implementation Plan: Wire PromptTemplateService into LLM Services (Option B)

**Status:** Complete  
**Created:** 2024-12-24  
**Last Updated:** 2024-12-24

## Overview

Refactor all LLM prompt generation to use the configurable `PromptTemplateService` with async service injection. Templates are resolved with priority: World DB → Global DB → Env → Default.

---

## Phase 1: Dialogue System (`LLMService` + `prompt_builder.rs`)

### Current State
- `prompt_builder.rs` contains pure functions (`build_system_prompt_with_notes`, etc.)
- Uses `RESPONSE_FORMAT_INSTRUCTIONS` const and inline format strings
- Called from `LLMService::generate_npc_response()` in `llm/mod.rs`

### Target State
- Convert `prompt_builder` to a struct `PromptBuilder` that holds `Arc<PromptTemplateService>`
- Methods become async and resolve templates internally
- `LLMService` owns a `PromptBuilder` instance

### Changes Required

#### 1.1 Create `PromptBuilder` struct in `prompt_builder.rs`

```rust
pub struct PromptBuilder {
    prompt_template_service: Arc<PromptTemplateService>,
}

impl PromptBuilder {
    pub fn new(prompt_template_service: Arc<PromptTemplateService>) -> Self {
        Self { prompt_template_service }
    }

    pub async fn build_system_prompt_with_notes(
        &self,
        world_id: Option<WorldId>,
        context: &SceneContext,
        character: &CharacterContext,
        notes: &DirectorialNotes,
        // ... other params stay same
    ) -> String {
        // Resolve templates
        let response_format = self.resolve(world_id, prompt_keys::DIALOGUE_RESPONSE_FORMAT).await;
        let challenge_format = self.resolve(world_id, prompt_keys::DIALOGUE_CHALLENGE_SUGGESTION_FORMAT).await;
        let narrative_format = self.resolve(world_id, prompt_keys::DIALOGUE_NARRATIVE_EVENT_FORMAT).await;
        
        // Build prompt using resolved templates instead of consts
        // ... existing logic, but use resolved strings
    }

    async fn resolve(&self, world_id: Option<WorldId>, key: &str) -> String {
        match world_id {
            Some(wid) => self.prompt_template_service.resolve_for_world(wid, key).await,
            None => self.prompt_template_service.resolve(key).await,
        }
    }
}
```

#### 1.2 Update `LLMService` in `llm/mod.rs`

```rust
pub struct LLMService<L: LlmPort> {
    llm_port: Arc<L>,
    prompt_builder: PromptBuilder,  // NEW
}

impl<L: LlmPort> LLMService<L> {
    pub fn new(
        llm_port: Arc<L>,
        prompt_template_service: Arc<PromptTemplateService>,  // NEW param
    ) -> Self {
        Self {
            llm_port,
            prompt_builder: PromptBuilder::new(prompt_template_service),
        }
    }

    pub async fn generate_npc_response(&self, request: GamePromptRequest) -> Result<...> {
        // Change from:
        //   let system_prompt = build_system_prompt_with_notes(...);
        // To:
        let system_prompt = self.prompt_builder.build_system_prompt_with_notes(
            Some(request.world_id),  // Pass world_id for per-world templates
            &request.scene,
            &request.character,
            &request.notes,
            // ...
        ).await;
        
        // Rest stays same
    }
}
```

#### 1.3 Update `LLMService` construction in `GameServices`

File: `engine-adapters/src/infrastructure/state/game_services.rs`

Need to pass `prompt_template_service` when constructing `LLMService`.

#### 1.4 Remove `RESPONSE_FORMAT_INSTRUCTIONS` const

After refactoring, the const is unused and can be deleted (the default is now in `domain/value_objects/prompt_templates.rs`).

### Status: [x] Complete

---

## Phase 2: Staging System

### Current State
- `staging_context_provider.rs` has `build_staging_prompt()` free function
- `staging_service.rs` has inline system prompt in `generate_llm_suggestions()`

### Target State
- `StagingService` holds `Arc<PromptTemplateService>`
- Resolves templates before building prompts

### Changes Required

#### 2.1 Add `PromptTemplateService` to `StagingService`

```rust
pub struct StagingService<L, R, N, S> {
    // ... existing fields
    prompt_template_service: Arc<PromptTemplateService>,  // NEW
}
```

#### 2.2 Update `generate_llm_suggestions()`

```rust
async fn generate_llm_suggestions(&self, ...) -> Result<Vec<StagedNpcProposal>> {
    // Resolve templates
    let system_prompt = self.prompt_template_service
        .resolve_for_world(world_id, prompt_keys::STAGING_SYSTEM_PROMPT)
        .await;
    let role_instructions = self.prompt_template_service
        .resolve_for_world(world_id, prompt_keys::STAGING_ROLE_INSTRUCTIONS)
        .await;
    let response_format = self.prompt_template_service
        .resolve_for_world(world_id, prompt_keys::STAGING_RESPONSE_FORMAT)
        .await;

    // Pass to build_staging_prompt
    let prompt = build_staging_prompt(
        context,
        rule_suggestions,
        dm_guidance,
        &role_instructions,   // NEW param
        &response_format,     // NEW param
    );

    let request = LlmRequest::new(vec![ChatMessage::user(prompt)])
        .with_system_prompt(system_prompt)  // Use resolved
        // ...
}
```

#### 2.3 Update `build_staging_prompt()` signature

```rust
pub fn build_staging_prompt(
    context: &StagingContext,
    rule_suggestions: &[RuleBasedSuggestion],
    dm_guidance: Option<&str>,
    role_instructions: &str,    // NEW
    response_format: &str,      // NEW
) -> String {
    // Use passed-in strings instead of inline literals
}
```

#### 2.4 Update `StagingService` construction in `AppState`

Thread `prompt_template_service` through to `StagingService::new()`.

### Status: [x] Complete

---

## Phase 3: Outcome Suggestion Service

### Current State
- `OutcomeSuggestionService` has `build_system_prompt()` and `build_branch_system_prompt()` methods
- Methods return hard-coded strings

### Target State
- Service holds `Arc<PromptTemplateService>`
- Methods resolve templates

### Changes Required

#### 3.1 Add `PromptTemplateService` to struct

```rust
pub struct OutcomeSuggestionService<L: LlmPort> {
    llm: Arc<L>,
    prompt_template_service: Arc<PromptTemplateService>,  // NEW
}
```

#### 3.2 Update `build_system_prompt()` to async

```rust
async fn build_system_prompt(&self, world_id: WorldId) -> String {
    self.prompt_template_service
        .resolve_for_world(world_id, prompt_keys::OUTCOME_SYSTEM_PROMPT)
        .await
}

async fn build_branch_system_prompt(&self, world_id: WorldId, branch_count: usize) -> String {
    let template = self.prompt_template_service
        .resolve_for_world(world_id, prompt_keys::OUTCOME_BRANCH_SYSTEM_PROMPT)
        .await;
    
    // Replace placeholder
    template.replace("{branch_count}", &branch_count.to_string())
}
```

#### 3.3 Update callers

`generate_suggestions()` and `generate_branches()` need to pass `world_id` and await the prompt methods.

**Note:** Need to verify if `OutcomeSuggestionRequest` contains `world_id`. If not, may need to add it or use global resolution.

### Status: [x] Complete

---

## Phase 4: Suggestion Service (Worldbuilding)

### Current State
- `SuggestionService` has 10 suggestion methods
- Each builds a prompt with `format!()` and inline template strings
- Templates use placeholders: `{entity_name}`, `{world_setting}`, `{hints}`, etc.

### Target State
- Service holds `Arc<PromptTemplateService>`
- Each method resolves its template key and performs placeholder substitution

### Changes Required

#### 4.1 Add `PromptTemplateService` to struct

```rust
pub struct SuggestionService<L: LlmPort> {
    llm: L,
    prompt_template_service: Arc<PromptTemplateService>,  // NEW
}
```

#### 4.2 Create helper for placeholder substitution

```rust
fn substitute_placeholders(template: &str, context: &SuggestionContext) -> String {
    template
        .replace("{entity_type}", context.entity_type.as_deref().unwrap_or("fantasy"))
        .replace("{entity_name}", context.entity_name.as_deref().unwrap_or("this character"))
        .replace("{world_setting}", context.world_setting.as_deref().unwrap_or("fantasy"))
        .replace("{hints}", context.hints.as_deref().unwrap_or(""))
        .replace("{additional_context}", context.additional_context.as_deref().unwrap_or(""))
}
```

#### 4.3 Update each suggestion method

Example for `suggest_character_names()`:

```rust
pub async fn suggest_character_names(
    &self,
    world_id: Option<WorldId>,  // May need to add this param
    context: &SuggestionContext,
) -> Result<Vec<String>> {
    let template = match world_id {
        Some(wid) => self.prompt_template_service
            .resolve_for_world(wid, prompt_keys::SUGGESTION_CHARACTER_NAME).await,
        None => self.prompt_template_service
            .resolve(prompt_keys::SUGGESTION_CHARACTER_NAME).await,
    };
    
    let prompt = substitute_placeholders(&template, context);
    self.generate_list(&prompt, 5).await
}
```

Repeat for all 10 suggestion methods with their respective template keys.

### Status: [x] Complete

---

## Phase 5: Service Construction Wiring

### Files to Update

1. **`engine-adapters/src/infrastructure/state/game_services.rs`**
   - `GameServices::new()` needs `prompt_template_service` param
   - Pass to `LLMService`, `OutcomeSuggestionService`, `SuggestionService`

2. **`engine-adapters/src/infrastructure/state/mod.rs`**
   - Pass `prompt_template_service` to `GameServices::new()`
   - Pass `prompt_template_service` to `StagingService::new()`

3. **HTTP route handlers** that construct services directly (if any)
   - Need to verify; most should go through `AppState`

### Status: [x] Complete

---

## Phase 6: Verification & Cleanup

### Tests to Update
- Unit tests in `prompt_builder.rs` need mocked `PromptTemplateService`
- Integration tests need the service available

### Cleanup
- Remove unused `const` declarations after migration
- Remove any duplicated default strings (now in `prompt_templates.rs`)

### Verification Steps
1. `cargo check --workspace` ✅
2. `cargo xtask arch-check` ✅
3. `cargo test --workspace` (if tests exist)
4. Manual testing: set an env override, verify prompt changes

### Status: [x] Complete

---

## Dependency Graph

```
AppState
  ├── prompt_template_service: Arc<PromptTemplateService>
  │
  ├── GameServices
  │     ├── LLMService
  │     │     └── PromptBuilder (holds prompt_template_service)
  │     ├── OutcomeSuggestionService (holds prompt_template_service)
  │     └── SuggestionService (holds prompt_template_service)
  │
  └── StagingService (holds prompt_template_service)
```

---

## Estimated Scope

| Phase | Files Modified | Lines Changed (est.) | Complexity |
|-------|---------------|---------------------|------------|
| 1. Dialogue | 3-4 | 80-120 | Medium |
| 2. Staging | 3 | 40-60 | Low |
| 3. Outcomes | 1 | 30-50 | Low |
| 4. Suggestions | 1 | 60-100 | Medium |
| 5. Wiring | 2 | 20-40 | Low |
| 6. Cleanup | 2-3 | -50 (deletions) | Low |
| **Total** | ~10 files | ~280-370 lines | Medium |

---

## Progress Log

- **2024-12-24**: Plan created, implementation starting with Phase 1
- **2024-12-24**: Phase 1 (Dialogue) complete
- **2024-12-24**: Phase 2 (Staging) complete
- **2024-12-24**: Phase 3 (Outcome Suggestions) complete
- **2024-12-24**: Phase 4 (Worldbuilding Suggestions) complete
- **2024-12-24**: Phase 5 (Wiring) complete
- **2024-12-24**: Phase 6 (Verification) complete - all checks passing
