# Sprint 6: Tier 3 Implementation Plan

**Created**: 2025-12-26  
**Status**: PLANNING  
**Estimated Effort**: 2-3 weeks  
**Priority**: P3 (Future Features - Architecture Improvements)

---

## Executive Summary

This sprint covers Tier 3 tasks representing larger architectural investments. These features address fundamental quality-of-life and maintainability concerns:

| Task | Effort | Status | Priority | Risk |
|------|--------|--------|----------|------|
| P3.1: Visual Trigger Condition Builder | 3-4 days | Not Started | Medium | Medium |
| P3.2: Advanced Workflow Parameter Editor | 2 days | Not Started | Low | Low |
| P3.3: Typed Error Handling | 3-4 days | Not Started | High | Medium |
| P3.4: Testing Infrastructure | 1-2 weeks | Not Started | High | High |

### Architecture Decision Summary

1. **P3.1** - Build a visual editor for narrative event triggers, replacing error-prone JSON editing
2. **P3.2** - Enhance ComfyUI workflow configuration with rich parameter editing
3. **P3.3** - Replace `anyhow::Result` with typed error enums for better error handling
4. **P3.4** - Establish testing infrastructure from scratch (currently 0 automated tests)

---

## P3.1: Visual Trigger Condition Builder (3-4 days)

### Problem Statement

Narrative event triggers are defined as JSON structures within `NarrativeEvent.trigger_conditions`. This requires users to:
- Manually construct complex nested JSON
- Know the exact schema for each trigger type
- Handle validation errors after submission

The `NarrativeTriggerType` enum (defined in `crates/domain/src/entities/narrative_event.rs:128-219`) has 15+ variants, each with different parameters - too complex for manual JSON editing.

### Current Architecture

**Domain Layer** (`narrative_event.rs:114-124`):
```rust
pub struct NarrativeTrigger {
    pub trigger_type: NarrativeTriggerType,
    pub description: String,
    pub is_required: bool,
    pub trigger_id: String,
}
```

**Trigger Types** (`narrative_event.rs:128-219`):
- `NpcAction { npc_id, npc_name, action_keywords, action_description }`
- `PlayerEntersLocation { location_id, location_name }`
- `TimeAtLocation { location_id, location_name, time_context }`
- `DialogueTopic { keywords, with_npc, npc_name }`
- `ChallengeCompleted { challenge_id, challenge_name, requires_success }`
- `RelationshipThreshold { character_id, character_name, with_character, with_character_name, min_sentiment, max_sentiment }`
- `HasItem { item_name, quantity }`
- `MissingItem { item_name }`
- `EventCompleted { event_id, event_name, outcome_name }`
- `TurnCount { turns, since_event }`
- `FlagSet { flag_name }`
- `FlagNotSet { flag_name }`
- `StatThreshold { character_id, stat_name, min_value, max_value }`
- `CombatResult { victory, involved_npc }`
- `Custom { description, llm_evaluation }`

**Trigger Logic** (`narrative_event.rs:102-111`):
```rust
pub enum TriggerLogic {
    All,           // AND - all conditions must match
    Any,           // OR - any single condition triggers
    AtLeast(u32),  // At least N conditions
}
```

### Implementation Plan

#### Step 1: Create Trigger Schema API Endpoint (2 hours)

**File**: NEW `crates/engine-adapters/src/infrastructure/http/trigger_routes.rs`

```rust
//! Trigger schema API routes
//!
//! Provides schema information for building trigger conditions visually.

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::infrastructure::state::AppState;

/// Trigger type schema for visual builder
#[derive(Debug, Clone, Serialize)]
pub struct TriggerTypeSchema {
    /// Machine-readable type name (e.g., "NpcAction")
    pub type_name: String,
    /// Human-readable display name (e.g., "NPC Performs Action")
    pub display_name: String,
    /// Description of when this trigger fires
    pub description: String,
    /// Category for grouping in UI
    pub category: TriggerCategory,
    /// Required parameters
    pub required_params: Vec<TriggerParam>,
    /// Optional parameters
    pub optional_params: Vec<TriggerParam>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerCategory {
    Location,
    Character,
    Dialogue,
    Challenge,
    Item,
    Event,
    Time,
    Custom,
}

#[derive(Debug, Clone, Serialize)]
pub struct TriggerParam {
    /// Parameter name
    pub name: String,
    /// Display label
    pub label: String,
    /// Parameter type for input rendering
    pub param_type: ParamType,
    /// Help text
    pub description: String,
    /// Default value (if any)
    pub default: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamType {
    /// UUID reference to entity
    EntityRef { entity_type: String },
    /// Text input
    Text,
    /// Multi-value text (keywords)
    TextList,
    /// Number input
    Number { min: Option<i32>, max: Option<i32> },
    /// Float input
    Float { min: Option<f32>, max: Option<f32> },
    /// Boolean checkbox
    Boolean,
    /// Selection from list
    Select { options: Vec<SelectOption> },
}

#[derive(Debug, Clone, Serialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

/// Get available trigger type schemas
pub async fn get_trigger_schema(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<TriggerTypeSchema>>, (StatusCode, String)> {
    Ok(Json(build_trigger_schemas()))
}

/// Build comprehensive trigger schemas from domain types
fn build_trigger_schemas() -> Vec<TriggerTypeSchema> {
    vec![
        TriggerTypeSchema {
            type_name: "NpcAction".to_string(),
            display_name: "NPC Performs Action".to_string(),
            description: "Triggers when a specific NPC performs an action or says something".to_string(),
            category: TriggerCategory::Character,
            required_params: vec![
                TriggerParam {
                    name: "npc_id".to_string(),
                    label: "NPC".to_string(),
                    param_type: ParamType::EntityRef { entity_type: "Character".to_string() },
                    description: "The NPC whose action triggers this event".to_string(),
                    default: None,
                },
                TriggerParam {
                    name: "action_description".to_string(),
                    label: "Action Description".to_string(),
                    param_type: ParamType::Text,
                    description: "Description of the action that triggers this event".to_string(),
                    default: None,
                },
            ],
            optional_params: vec![
                TriggerParam {
                    name: "action_keywords".to_string(),
                    label: "Action Keywords".to_string(),
                    param_type: ParamType::TextList,
                    description: "Keywords that identify the triggering action".to_string(),
                    default: Some(serde_json::json!([])),
                },
            ],
        },
        TriggerTypeSchema {
            type_name: "PlayerEntersLocation".to_string(),
            display_name: "Player Enters Location".to_string(),
            description: "Triggers when any player character enters a specific location".to_string(),
            category: TriggerCategory::Location,
            required_params: vec![
                TriggerParam {
                    name: "location_id".to_string(),
                    label: "Location".to_string(),
                    param_type: ParamType::EntityRef { entity_type: "Location".to_string() },
                    description: "The location that triggers this event when entered".to_string(),
                    default: None,
                },
            ],
            optional_params: vec![],
        },
        TriggerTypeSchema {
            type_name: "ChallengeCompleted".to_string(),
            display_name: "Challenge Completed".to_string(),
            description: "Triggers when a specific challenge is completed".to_string(),
            category: TriggerCategory::Challenge,
            required_params: vec![
                TriggerParam {
                    name: "challenge_id".to_string(),
                    label: "Challenge".to_string(),
                    param_type: ParamType::EntityRef { entity_type: "Challenge".to_string() },
                    description: "The challenge that triggers this event when completed".to_string(),
                    default: None,
                },
            ],
            optional_params: vec![
                TriggerParam {
                    name: "requires_success".to_string(),
                    label: "Requires Success".to_string(),
                    param_type: ParamType::Select { 
                        options: vec![
                            SelectOption { value: "null".to_string(), label: "Any Result".to_string() },
                            SelectOption { value: "true".to_string(), label: "Success Only".to_string() },
                            SelectOption { value: "false".to_string(), label: "Failure Only".to_string() },
                        ]
                    },
                    description: "Whether the challenge must succeed, fail, or either".to_string(),
                    default: Some(serde_json::json!(null)),
                },
            ],
        },
        TriggerTypeSchema {
            type_name: "HasItem".to_string(),
            display_name: "Player Has Item".to_string(),
            description: "Triggers when a player possesses a specific item".to_string(),
            category: TriggerCategory::Item,
            required_params: vec![
                TriggerParam {
                    name: "item_name".to_string(),
                    label: "Item Name".to_string(),
                    param_type: ParamType::Text,
                    description: "Name of the item the player must possess".to_string(),
                    default: None,
                },
            ],
            optional_params: vec![
                TriggerParam {
                    name: "quantity".to_string(),
                    label: "Minimum Quantity".to_string(),
                    param_type: ParamType::Number { min: Some(1), max: None },
                    description: "Minimum quantity required (default: 1)".to_string(),
                    default: Some(serde_json::json!(1)),
                },
            ],
        },
        TriggerTypeSchema {
            type_name: "FlagSet".to_string(),
            display_name: "Flag Is Set".to_string(),
            description: "Triggers when a game flag is set to true".to_string(),
            category: TriggerCategory::Event,
            required_params: vec![
                TriggerParam {
                    name: "flag_name".to_string(),
                    label: "Flag Name".to_string(),
                    param_type: ParamType::Text,
                    description: "Name of the flag that must be set".to_string(),
                    default: None,
                },
            ],
            optional_params: vec![],
        },
        TriggerTypeSchema {
            type_name: "EventCompleted".to_string(),
            display_name: "Event Completed".to_string(),
            description: "Triggers when another narrative event has been triggered".to_string(),
            category: TriggerCategory::Event,
            required_params: vec![
                TriggerParam {
                    name: "event_id".to_string(),
                    label: "Event".to_string(),
                    param_type: ParamType::EntityRef { entity_type: "NarrativeEvent".to_string() },
                    description: "The event that must be completed".to_string(),
                    default: None,
                },
            ],
            optional_params: vec![
                TriggerParam {
                    name: "outcome_name".to_string(),
                    label: "Specific Outcome".to_string(),
                    param_type: ParamType::Text,
                    description: "Optional: specific outcome that must have occurred".to_string(),
                    default: None,
                },
            ],
        },
        TriggerTypeSchema {
            type_name: "TurnCount".to_string(),
            display_name: "Turn Count Reached".to_string(),
            description: "Triggers after a certain number of turns".to_string(),
            category: TriggerCategory::Time,
            required_params: vec![
                TriggerParam {
                    name: "turns".to_string(),
                    label: "Number of Turns".to_string(),
                    param_type: ParamType::Number { min: Some(1), max: None },
                    description: "Number of turns to wait".to_string(),
                    default: None,
                },
            ],
            optional_params: vec![
                TriggerParam {
                    name: "since_event".to_string(),
                    label: "Since Event".to_string(),
                    param_type: ParamType::EntityRef { entity_type: "NarrativeEvent".to_string() },
                    description: "Count turns since this event (or since session start if empty)".to_string(),
                    default: None,
                },
            ],
        },
        TriggerTypeSchema {
            type_name: "Custom".to_string(),
            display_name: "Custom Condition".to_string(),
            description: "A free-form condition evaluated by the LLM or DM".to_string(),
            category: TriggerCategory::Custom,
            required_params: vec![
                TriggerParam {
                    name: "description".to_string(),
                    label: "Condition Description".to_string(),
                    param_type: ParamType::Text,
                    description: "Natural language description of the condition".to_string(),
                    default: None,
                },
            ],
            optional_params: vec![
                TriggerParam {
                    name: "llm_evaluation".to_string(),
                    label: "LLM Evaluates".to_string(),
                    param_type: ParamType::Boolean,
                    description: "If true, LLM will evaluate this condition against game context".to_string(),
                    default: Some(serde_json::json!(false)),
                },
            ],
        },
        // Add remaining trigger types following the same pattern...
        // DialogueTopic, TimeAtLocation, RelationshipThreshold, MissingItem, 
        // FlagNotSet, StatThreshold, CombatResult
    ]
}
```

**Register Route** in `crates/engine-adapters/src/infrastructure/http/mod.rs`:
```rust
// Add to imports
pub mod trigger_routes;

// Add to router configuration (around line 180)
.route("/api/triggers/schema", get(trigger_routes::get_trigger_schema))
```

**Verification**:
```bash
cargo check -p wrldbldr-engine-adapters
cargo test -p wrldbldr-engine-adapters
```

**Time**: 2 hours

---

#### Step 2: Create Visual Trigger Builder Component (8 hours)

**File**: NEW `crates/player-ui/src/presentation/components/story_arc/trigger_builder.rs`

**UI Mockup**:
```
┌─────────────────────────────────────────────────────────────────────┐
│ Trigger Conditions                                   [+ Add Trigger] │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Logic: (•) All must match  ( ) Any can match  ( ) At least [2] ▼   │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │ 1. Player Enters Location                              [×] [↕] │ │
│  │    ┌─────────────────────────────────────────────────────────┐ │ │
│  │    │ Location: [Abandoned Tower                         ▼]   │ │ │
│  │    └─────────────────────────────────────────────────────────┘ │ │
│  │    ☑ Required (must match even in "At least N" mode)          │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │ 2. Player Has Item                                      [×] [↕] │ │
│  │    ┌─────────────────────────────────────────────────────────┐ │ │
│  │    │ Item Name: [Ancient Key                               ]  │ │ │
│  │    │ Min Quantity: [1                                      ]  │ │ │
│  │    └─────────────────────────────────────────────────────────┘ │ │
│  │    ☐ Required                                                  │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │ 3. Flag Is Set                                          [×] [↕] │ │
│  │    ┌─────────────────────────────────────────────────────────┐ │ │
│  │    │ Flag Name: [tower_door_unlocked                       ]  │ │ │
│  │    └─────────────────────────────────────────────────────────┘ │ │
│  │    ☐ Required                                                  │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                      │
│  [+ Add Trigger Condition]                                           │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

**Component Structure**:

```rust
//! Visual Trigger Condition Builder
//!
//! Provides a visual interface for building narrative event trigger conditions
//! instead of manual JSON editing.

use dioxus::prelude::*;
use wrldbldr_player_app::application::services::{
    TriggerTypeSchema, TriggerCategory, ParamType,
};

/// Props for TriggerBuilder component
#[derive(Props, Clone, PartialEq)]
pub struct TriggerBuilderProps {
    /// World ID for entity lookups
    pub world_id: String,
    /// Current trigger conditions
    pub triggers: Vec<TriggerData>,
    /// Current trigger logic
    pub logic: TriggerLogicData,
    /// Callback when triggers change
    pub on_change: EventHandler<TriggerBuilderState>,
}

/// Serializable trigger data for the builder
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerData {
    pub id: String,
    pub trigger_type: String,
    pub params: std::collections::HashMap<String, serde_json::Value>,
    pub description: String,
    pub is_required: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerLogicData {
    All,
    Any,
    AtLeast(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TriggerBuilderState {
    pub triggers: Vec<TriggerData>,
    pub logic: TriggerLogicData,
}

/// Main trigger builder component
#[component]
pub fn TriggerBuilder(props: TriggerBuilderProps) -> Element {
    // Load trigger schemas from API
    let schema_resource = use_resource(move || async move {
        // Fetch from /api/triggers/schema
        fetch_trigger_schemas().await
    });
    
    // Local state for editing
    let mut triggers = use_signal(|| props.triggers.clone());
    let mut logic = use_signal(|| props.logic.clone());
    let mut adding_trigger = use_signal(|| false);
    
    // Notify parent on changes
    let on_change = props.on_change.clone();
    use_effect(move || {
        on_change.call(TriggerBuilderState {
            triggers: triggers.read().clone(),
            logic: logic.read().clone(),
        });
    });

    rsx! {
        div {
            class: "trigger-builder bg-dark-surface rounded-lg p-4",
            
            // Header with logic selector
            div {
                class: "flex items-center justify-between mb-4",
                
                h3 { class: "text-white text-lg font-medium", "Trigger Conditions" }
                
                button {
                    onclick: move |_| adding_trigger.set(true),
                    class: "py-2 px-3 bg-blue-500 text-white text-sm rounded-lg",
                    "+ Add Trigger"
                }
            }
            
            // Logic mode selector
            TriggerLogicSelector {
                logic: logic.read().clone(),
                on_change: move |new_logic| logic.set(new_logic),
            }
            
            // Trigger list
            div {
                class: "flex flex-col gap-3 mt-4",
                
                for (idx, trigger) in triggers.read().iter().enumerate() {
                    TriggerCard {
                        key: "{trigger.id}",
                        index: idx,
                        trigger: trigger.clone(),
                        schemas: schema_resource.read().as_ref()
                            .map(|s| s.clone())
                            .unwrap_or_default(),
                        world_id: props.world_id.clone(),
                        on_update: move |updated| {
                            let mut t = triggers.write();
                            t[idx] = updated;
                        },
                        on_remove: move |_| {
                            triggers.write().remove(idx);
                        },
                    }
                }
            }
            
            // Add trigger modal
            if *adding_trigger.read() {
                AddTriggerModal {
                    schemas: schema_resource.read().as_ref()
                        .map(|s| s.clone())
                        .unwrap_or_default(),
                    on_add: move |new_trigger| {
                        triggers.write().push(new_trigger);
                        adding_trigger.set(false);
                    },
                    on_cancel: move |_| adding_trigger.set(false),
                }
            }
        }
    }
}

/// Logic mode selector component
#[component]
fn TriggerLogicSelector(
    logic: TriggerLogicData,
    on_change: EventHandler<TriggerLogicData>,
) -> Element {
    let at_least_count = match &logic {
        TriggerLogicData::AtLeast(n) => *n,
        _ => 2,
    };
    
    rsx! {
        div {
            class: "flex items-center gap-4 p-3 bg-black bg-opacity-20 rounded-lg",
            
            span { class: "text-gray-400 text-sm", "Logic:" }
            
            label {
                class: "flex items-center gap-2 cursor-pointer",
                input {
                    r#type: "radio",
                    checked: matches!(logic, TriggerLogicData::All),
                    onchange: move |_| on_change.call(TriggerLogicData::All),
                }
                span { class: "text-white text-sm", "All must match" }
            }
            
            label {
                class: "flex items-center gap-2 cursor-pointer",
                input {
                    r#type: "radio",
                    checked: matches!(logic, TriggerLogicData::Any),
                    onchange: move |_| on_change.call(TriggerLogicData::Any),
                }
                span { class: "text-white text-sm", "Any can match" }
            }
            
            label {
                class: "flex items-center gap-2 cursor-pointer",
                input {
                    r#type: "radio",
                    checked: matches!(logic, TriggerLogicData::AtLeast(_)),
                    onchange: move |_| on_change.call(TriggerLogicData::AtLeast(at_least_count)),
                }
                span { class: "text-white text-sm", "At least" }
                
                if matches!(logic, TriggerLogicData::AtLeast(_)) {
                    input {
                        r#type: "number",
                        min: "1",
                        value: "{at_least_count}",
                        onchange: move |e| {
                            if let Ok(n) = e.value().parse::<u32>() {
                                on_change.call(TriggerLogicData::AtLeast(n));
                            }
                        },
                        class: "w-16 py-1 px-2 bg-dark-bg border border-gray-700 rounded text-white text-sm",
                    }
                }
            }
        }
    }
}

// Additional sub-components: TriggerCard, AddTriggerModal, ParamInput
// Implementation follows similar patterns to existing component structure
```

**Integration Point**: Modify `crates/player-ui/src/presentation/components/story_arc/narrative_event_card.rs` to use `TriggerBuilder` instead of raw JSON textarea.

**Verification**:
```bash
cargo check -p wrldbldr-player-ui
# Manual test in browser - create narrative event with triggers
```

**Time**: 8 hours

---

#### Step 3: Wire Builder to Narrative Event Editor (4 hours)

**File**: MODIFY `crates/player-ui/src/presentation/components/story_arc/narrative_event_card.rs`

**Changes**:
1. Import and use `TriggerBuilder` component
2. Convert between `TriggerData` ↔ `NarrativeTrigger` domain types
3. Add save/load for trigger state

**Location**: Replace the JSON textarea for triggers (around lines 200-250 based on component structure)

**Verification**:
```bash
cargo check -p wrldbldr-player-ui
dx serve --platform web
# Navigate to Story Arc → Narrative Events → Edit an event
# Verify trigger builder appears and works
```

**Time**: 4 hours

---

### Success Criteria

- [ ] `/api/triggers/schema` returns complete schema for all 15 trigger types
- [ ] Trigger builder renders with correct UI for each parameter type
- [ ] Entity reference fields (NPC, Location, Challenge) show searchable dropdowns
- [ ] Logic selector (All/Any/AtLeast) works correctly
- [ ] Triggers can be added, edited, removed, and reordered
- [ ] Builder state serializes to valid `NarrativeTrigger[]` JSON
- [ ] Existing triggers load correctly into builder from saved events

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Schema/UI mismatch | Medium | High | Comprehensive test coverage for all trigger types |
| Entity search performance | Low | Medium | Paginated search with debounce |
| Complex nested triggers | Low | Medium | Start with simple flat structure, add nesting later |
| Browser compatibility | Low | Low | Test on Chrome, Firefox, Safari |

---

## P3.2: Advanced Workflow Parameter Editor (2 days)

### Problem Statement

The current workflow configuration editor (`workflow_config_editor.rs:1-834`) provides basic editing but lacks:
- Visual parameter grouping by node
- Type-aware input controls (sliders for ranges, color pickers, etc.)
- Parameter preview showing effect on workflow
- Detection and highlight of style reference inputs

### Current Architecture

**Workflow Config Editor** (`workflow_config_editor.rs:36-423`):
- Fetches `WorkflowConfig` via `WorkflowService.get_workflow_config()`
- Displays prompt mappings (`PromptMappingRow`)
- Displays input defaults (`InputDefaultRow`) 
- Basic text inputs for all parameter types

**Workflow Analysis** (from `crates/player-app/src/application/services`):
```rust
pub struct WorkflowAnalysis {
    pub node_count: usize,
    pub inputs: Vec<WorkflowInput>,
    pub text_inputs: Vec<WorkflowInput>,
}

pub struct WorkflowInput {
    pub node_id: String,
    pub node_title: Option<String>,
    pub input_name: String,
    pub input_type: String,        // "integer", "float", "string", etc.
    pub current_value: serde_json::Value,
}
```

### Implementation Plan

#### Step 1: Enhance Input Type Detection (2 hours)

**File**: MODIFY `crates/player-app/src/application/services/workflow_service.rs`

Add pattern detection for special input types:
- Seed inputs (detect `seed` in name)
- Dimension inputs (detect `width`, `height`)
- Step counts (detect `steps`)
- CFG scale (detect `cfg`, `guidance`)
- Sampler/scheduler selection
- Image path inputs (style reference candidates)

```rust
/// Enhanced input type detection
fn detect_input_semantics(input: &WorkflowInput) -> InputSemantics {
    let name_lower = input.input_name.to_lowercase();
    let node_lower = input.node_title.as_ref()
        .map(|t| t.to_lowercase())
        .unwrap_or_default();
    
    if name_lower.contains("seed") {
        return InputSemantics::Seed;
    }
    if name_lower == "width" || name_lower == "height" {
        return InputSemantics::Dimension { 
            common_values: vec![512, 768, 1024, 1536, 2048] 
        };
    }
    if name_lower.contains("steps") {
        return InputSemantics::Steps { min: 1, max: 150, default: 20 };
    }
    if name_lower.contains("cfg") || name_lower.contains("guidance") {
        return InputSemantics::CfgScale { min: 1.0, max: 30.0, default: 7.0 };
    }
    if name_lower.contains("sampler") {
        return InputSemantics::Sampler { 
            options: vec!["euler", "euler_ancestral", "dpmpp_2m", "dpmpp_sde"] 
        };
    }
    if name_lower.contains("scheduler") {
        return InputSemantics::Scheduler { 
            options: vec!["normal", "karras", "exponential"] 
        };
    }
    if name_lower.contains("image") && (name_lower.contains("path") || name_lower.contains("load")) {
        return InputSemantics::ImagePath { is_style_ref_candidate: true };
    }
    
    InputSemantics::Generic
}

#[derive(Debug, Clone, Serialize)]
pub enum InputSemantics {
    Seed,
    Dimension { common_values: Vec<i32> },
    Steps { min: i32, max: i32, default: i32 },
    CfgScale { min: f32, max: f32, default: f32 },
    Sampler { options: Vec<&'static str> },
    Scheduler { options: Vec<&'static str> },
    ImagePath { is_style_ref_candidate: bool },
    Generic,
}
```

**Verification**:
```bash
cargo check -p wrldbldr-player-app
```

**Time**: 2 hours

---

#### Step 2: Create Type-Aware Input Components (6 hours)

**File**: MODIFY `crates/player-ui/src/presentation/components/settings/workflow_config_editor.rs`

**New Sub-Components**:

```rust
/// Semantic-aware input for workflow parameters
#[component]
fn SemanticParamInput(
    input: WorkflowInput,
    semantics: InputSemantics,
    value: serde_json::Value,
    locked: bool,
    on_change: EventHandler<serde_json::Value>,
) -> Element {
    match semantics {
        InputSemantics::Seed => rsx! {
            SeedInput { value, locked, on_change }
        },
        InputSemantics::Dimension { common_values } => rsx! {
            DimensionInput { value, common_values, locked, on_change }
        },
        InputSemantics::Steps { min, max, default } => rsx! {
            SliderInput { value, min, max, default, locked, on_change }
        },
        InputSemantics::CfgScale { min, max, default } => rsx! {
            SliderInput { value, min, max, default, locked, on_change, step: 0.5 }
        },
        InputSemantics::Sampler { options } | InputSemantics::Scheduler { options } => rsx! {
            SelectInput { value, options, locked, on_change }
        },
        InputSemantics::ImagePath { is_style_ref_candidate } => rsx! {
            ImagePathInput { 
                value, 
                locked, 
                is_style_ref_candidate,
                on_change 
            }
        },
        InputSemantics::Generic => rsx! {
            GenericTextInput { value, locked, on_change, input_type: input.input_type }
        },
    }
}

/// Seed input with randomize button
#[component]
fn SeedInput(
    value: serde_json::Value,
    locked: bool,
    on_change: EventHandler<serde_json::Value>,
) -> Element {
    let current_seed = value.as_i64().unwrap_or(0);
    
    rsx! {
        div {
            class: "flex items-center gap-2",
            
            input {
                r#type: "number",
                value: "{current_seed}",
                disabled: locked,
                onchange: move |e| {
                    if let Ok(n) = e.value().parse::<i64>() {
                        on_change.call(serde_json::json!(n));
                    }
                },
                class: "w-32 py-1 px-2 bg-dark-bg border border-gray-700 rounded text-white text-sm",
            }
            
            button {
                onclick: move |_| {
                    let random_seed = rand::random::<i64>().abs();
                    on_change.call(serde_json::json!(random_seed));
                },
                disabled: locked,
                class: "py-1 px-2 bg-gray-600 text-white text-xs rounded",
                "Random"
            }
            
            button {
                onclick: move |_| on_change.call(serde_json::json!(-1)),
                disabled: locked,
                class: "py-1 px-2 bg-gray-600 text-white text-xs rounded",
                "-1 (Random)"
            }
        }
    }
}

/// Slider input with numeric display
#[component]
fn SliderInput(
    value: serde_json::Value,
    min: f32,
    max: f32,
    default: f32,
    locked: bool,
    step: Option<f32>,
    on_change: EventHandler<serde_json::Value>,
) -> Element {
    let current = value.as_f64().unwrap_or(default as f64) as f32;
    let step_val = step.unwrap_or(1.0);
    
    rsx! {
        div {
            class: "flex items-center gap-2",
            
            input {
                r#type: "range",
                min: "{min}",
                max: "{max}",
                step: "{step_val}",
                value: "{current}",
                disabled: locked,
                oninput: move |e| {
                    if let Ok(n) = e.value().parse::<f64>() {
                        on_change.call(serde_json::json!(n));
                    }
                },
                class: "flex-1",
            }
            
            span {
                class: "text-white text-sm w-12 text-right",
                "{current:.1}"
            }
        }
    }
}
```

**Verification**:
```bash
cargo check -p wrldbldr-player-ui
dx serve --platform web
# Navigate to Settings → Workflows → Edit a workflow
# Verify type-aware inputs render correctly
```

**Time**: 6 hours

---

#### Step 3: Add Parameter Grouping by Node (4 hours)

**File**: MODIFY `crates/player-ui/src/presentation/components/settings/workflow_config_editor.rs`

Group inputs by their source node with collapsible sections:

```rust
/// Group inputs by node for organized display
fn group_inputs_by_node(inputs: &[WorkflowInput]) -> Vec<NodeInputGroup> {
    let mut groups: std::collections::HashMap<String, NodeInputGroup> = 
        std::collections::HashMap::new();
    
    for input in inputs {
        let node_key = input.node_id.clone();
        let group = groups.entry(node_key.clone()).or_insert_with(|| {
            NodeInputGroup {
                node_id: node_key,
                node_title: input.node_title.clone()
                    .unwrap_or_else(|| format!("Node {}", input.node_id)),
                inputs: Vec::new(),
            }
        });
        group.inputs.push(input.clone());
    }
    
    let mut result: Vec<NodeInputGroup> = groups.into_values().collect();
    result.sort_by(|a, b| a.node_title.cmp(&b.node_title));
    result
}

#[derive(Debug, Clone)]
struct NodeInputGroup {
    node_id: String,
    node_title: String,
    inputs: Vec<WorkflowInput>,
}

/// Collapsible node group component
#[component]
fn NodeParameterGroup(
    group: NodeInputGroup,
    defaults: Vec<InputDefault>,
    locked_inputs: Vec<String>,
    on_change: EventHandler<InputDefault>,
) -> Element {
    let mut expanded = use_signal(|| true);
    
    rsx! {
        div {
            class: "node-group mb-3 bg-black bg-opacity-20 rounded-lg overflow-hidden",
            
            button {
                onclick: move |_| expanded.set(!*expanded.read()),
                class: "w-full flex items-center justify-between py-2 px-3 bg-gray-800 text-white",
                
                span { class: "font-medium", "{group.node_title}" }
                span { 
                    class: "text-gray-500 text-sm", 
                    "{group.inputs.len()} parameters" 
                }
                span { 
                    class: "text-gray-500",
                    if *expanded.read() { "▼" } else { "▶" }
                }
            }
            
            if *expanded.read() {
                div {
                    class: "p-3 flex flex-col gap-2",
                    
                    for input in group.inputs.iter() {
                        // Render SemanticParamInput for each input
                    }
                }
            }
        }
    }
}
```

**Verification**:
```bash
cargo check -p wrldbldr-player-ui
dx serve --platform web
# Verify inputs are grouped by node with collapsible sections
```

**Time**: 4 hours

---

### Success Criteria

- [ ] Workflow inputs are grouped by source node with collapsible sections
- [ ] Seed inputs have "Random" and "-1" quick buttons
- [ ] Dimension inputs show common value presets (512, 768, 1024, etc.)
- [ ] Step/CFG inputs use sliders with numeric display
- [ ] Sampler/Scheduler inputs use dropdown selects
- [ ] Image path inputs are flagged as style reference candidates
- [ ] Parameter changes persist correctly

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| ComfyUI schema variations | Medium | Medium | Handle unknown input types gracefully |
| Performance with large workflows | Low | Low | Virtual scrolling for 50+ inputs |
| Value type mismatches | Medium | Medium | Validate against expected types |

---

## P3.3: Typed Error Handling (3-4 days)

### Problem Statement

The codebase extensively uses `anyhow::Result` (100+ occurrences), which:
- Loses error type information at compile time
- Makes error handling generic (`map_err(|e| e.to_string())`)
- Prevents proper HTTP status code mapping
- Complicates debugging without error context

### Current Error Handling Patterns

**Repository Layer** (`challenge_repository.rs:12`):
```rust
use anyhow::Result;

async fn get(&self, id: ChallengeId) -> Result<Option<Challenge>>
```

**HTTP Layer** (`challenge_routes.rs:36-44`):
```rust
async fn build_challenge_response(...) 
    -> Result<ChallengeResponseDto, (StatusCode, String)> {
    // ...
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
}
```

**Service Layer** (`dm_approval_queue_service.rs:675`):
```rust
) -> anyhow::Result<()>
```

### Target Architecture

Implement a three-tier error hierarchy matching the hexagonal architecture:

```
┌───────────────────────────────────────────────────────────────┐
│                     InfrastructureError                        │
│  (Database errors, HTTP client errors, WebSocket errors)       │
└─────────────────────────────┬─────────────────────────────────┘
                              │ wraps
┌─────────────────────────────▼─────────────────────────────────┐
│                     ApplicationError                           │
│  (Service errors, validation errors, authorization errors)     │
└─────────────────────────────┬─────────────────────────────────┘
                              │ wraps
┌─────────────────────────────▼─────────────────────────────────┐
│                       DomainError                              │
│  (Entity validation, business rule violations)                 │
└───────────────────────────────────────────────────────────────┘
```

### Implementation Plan

#### Step 1: Define Error Types in Domain Layer (4 hours)

**File**: NEW `crates/domain/src/errors.rs`

```rust
//! Domain error types for WrldBldr
//!
//! These errors represent business rule violations and domain invariant failures.
//! They are the lowest level of the error hierarchy.

use thiserror::Error;
use wrldbldr_domain::{
    ChallengeId, CharacterId, LocationId, NarrativeEventId, SceneId, WorldId,
};

/// Domain-level errors representing business rule violations
#[derive(Debug, Error)]
pub enum DomainError {
    // ============================================================
    // Entity Not Found Errors
    // ============================================================
    
    #[error("World not found: {0}")]
    WorldNotFound(WorldId),
    
    #[error("Character not found: {0}")]
    CharacterNotFound(CharacterId),
    
    #[error("Challenge not found: {0}")]
    ChallengeNotFound(ChallengeId),
    
    #[error("Location not found: {0}")]
    LocationNotFound(LocationId),
    
    #[error("Scene not found: {0}")]
    SceneNotFound(SceneId),
    
    #[error("Narrative event not found: {0}")]
    NarrativeEventNotFound(NarrativeEventId),
    
    // ============================================================
    // Validation Errors
    // ============================================================
    
    #[error("Invalid entity state: {entity_type} {entity_id} - {reason}")]
    InvalidEntityState {
        entity_type: &'static str,
        entity_id: String,
        reason: String,
    },
    
    #[error("Validation failed for {field}: {message}")]
    ValidationError {
        field: String,
        message: String,
    },
    
    #[error("Required field missing: {0}")]
    RequiredFieldMissing(String),
    
    // ============================================================
    // Business Rule Violations
    // ============================================================
    
    #[error("Challenge already resolved: {0}")]
    ChallengeAlreadyResolved(ChallengeId),
    
    #[error("Challenge not active: {0}")]
    ChallengeNotActive(ChallengeId),
    
    #[error("Narrative event already triggered: {0}")]
    EventAlreadyTriggered(NarrativeEventId),
    
    #[error("Prerequisite not met: {0}")]
    PrerequisiteNotMet(String),
    
    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),
    
    // ============================================================
    // Relationship Errors
    // ============================================================
    
    #[error("Relationship already exists: {from} -> {to}")]
    RelationshipExists {
        from: String,
        to: String,
    },
    
    #[error("Relationship not found: {from} -> {to}")]
    RelationshipNotFound {
        from: String,
        to: String,
    },
    
    // ============================================================
    // Generic Errors
    // ============================================================
    
    #[error("Domain error: {0}")]
    Other(String),
}

impl DomainError {
    /// Check if this error represents a "not found" condition
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            DomainError::WorldNotFound(_)
                | DomainError::CharacterNotFound(_)
                | DomainError::ChallengeNotFound(_)
                | DomainError::LocationNotFound(_)
                | DomainError::SceneNotFound(_)
                | DomainError::NarrativeEventNotFound(_)
                | DomainError::RelationshipNotFound { .. }
        )
    }
    
    /// Check if this error represents a validation failure
    pub fn is_validation_error(&self) -> bool {
        matches!(
            self,
            DomainError::InvalidEntityState { .. }
                | DomainError::ValidationError { .. }
                | DomainError::RequiredFieldMissing(_)
        )
    }
    
    /// Check if this error represents a business rule violation
    pub fn is_business_rule_violation(&self) -> bool {
        matches!(
            self,
            DomainError::ChallengeAlreadyResolved(_)
                | DomainError::ChallengeNotActive(_)
                | DomainError::EventAlreadyTriggered(_)
                | DomainError::PrerequisiteNotMet(_)
                | DomainError::InsufficientPermissions(_)
        )
    }
}
```

**Update `crates/domain/src/lib.rs`**:
```rust
pub mod errors;
pub use errors::DomainError;
```

**Verification**:
```bash
cargo check -p wrldbldr-domain
```

**Time**: 4 hours

---

#### Step 2: Define Application and Infrastructure Errors (4 hours)

**File**: NEW `crates/engine-ports/src/errors.rs`

```rust
//! Application-level error types
//!
//! These errors wrap domain errors and add service-layer context.

use thiserror::Error;
use wrldbldr_domain::DomainError;

/// Application-level errors for service operations
#[derive(Debug, Error)]
pub enum ApplicationError {
    // ============================================================
    // Domain Error Wrapper
    // ============================================================
    
    #[error(transparent)]
    Domain(#[from] DomainError),
    
    // ============================================================
    // Service Errors
    // ============================================================
    
    #[error("Service unavailable: {service_name} - {reason}")]
    ServiceUnavailable {
        service_name: &'static str,
        reason: String,
    },
    
    #[error("Operation timeout: {operation} after {duration_ms}ms")]
    OperationTimeout {
        operation: String,
        duration_ms: u64,
    },
    
    #[error("Rate limit exceeded for {resource}")]
    RateLimitExceeded {
        resource: String,
    },
    
    // ============================================================
    // Authorization Errors
    // ============================================================
    
    #[error("Unauthorized access to {resource}")]
    Unauthorized {
        resource: String,
    },
    
    #[error("Forbidden: {reason}")]
    Forbidden {
        reason: String,
    },
    
    // ============================================================
    // Input Errors
    // ============================================================
    
    #[error("Invalid input: {field} - {message}")]
    InvalidInput {
        field: String,
        message: String,
    },
    
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),
    
    #[error("Malformed request: {0}")]
    MalformedRequest(String),
    
    // ============================================================
    // State Errors
    // ============================================================
    
    #[error("Conflict: {resource} is in state {current_state}, expected {expected_state}")]
    StateConflict {
        resource: String,
        current_state: String,
        expected_state: String,
    },
    
    #[error("Concurrent modification detected for {resource}")]
    ConcurrentModification {
        resource: String,
    },
    
    // ============================================================
    // Infrastructure Wrapper
    // ============================================================
    
    #[error("Infrastructure error: {0}")]
    Infrastructure(#[from] InfrastructureError),
    
    // ============================================================
    // Generic
    // ============================================================
    
    #[error("Application error: {0}")]
    Other(String),
}

/// Infrastructure-level errors (databases, external services)
#[derive(Debug, Error)]
pub enum InfrastructureError {
    // ============================================================
    // Database Errors
    // ============================================================
    
    #[error("Database connection failed: {0}")]
    DatabaseConnection(String),
    
    #[error("Database query failed: {0}")]
    DatabaseQuery(String),
    
    #[error("Database transaction failed: {0}")]
    DatabaseTransaction(String),
    
    // ============================================================
    // External Service Errors
    // ============================================================
    
    #[error("LLM service error: {0}")]
    LlmService(String),
    
    #[error("ComfyUI service error: {0}")]
    ComfyUiService(String),
    
    #[error("HTTP client error: {0}")]
    HttpClient(String),
    
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    
    // ============================================================
    // Serialization Errors
    // ============================================================
    
    #[error("Serialization failed: {0}")]
    Serialization(String),
    
    #[error("Deserialization failed: {0}")]
    Deserialization(String),
    
    // ============================================================
    // I/O Errors
    // ============================================================
    
    #[error("File I/O error: {path} - {reason}")]
    FileIo {
        path: String,
        reason: String,
    },
    
    // ============================================================
    // Generic
    // ============================================================
    
    #[error("Infrastructure error: {0}")]
    Other(String),
}

impl ApplicationError {
    /// Map to HTTP status code
    pub fn status_code(&self) -> u16 {
        match self {
            // 4xx Client Errors
            ApplicationError::Domain(de) if de.is_not_found() => 404,
            ApplicationError::Domain(de) if de.is_validation_error() => 400,
            ApplicationError::Domain(de) if de.is_business_rule_violation() => 422,
            ApplicationError::InvalidInput { .. } => 400,
            ApplicationError::MissingParameter(_) => 400,
            ApplicationError::MalformedRequest(_) => 400,
            ApplicationError::Unauthorized { .. } => 401,
            ApplicationError::Forbidden { .. } => 403,
            ApplicationError::StateConflict { .. } => 409,
            ApplicationError::ConcurrentModification { .. } => 409,
            ApplicationError::RateLimitExceeded { .. } => 429,
            
            // 5xx Server Errors
            ApplicationError::ServiceUnavailable { .. } => 503,
            ApplicationError::OperationTimeout { .. } => 504,
            ApplicationError::Infrastructure(_) => 500,
            ApplicationError::Domain(_) => 500,
            ApplicationError::Other(_) => 500,
        }
    }
    
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ApplicationError::ServiceUnavailable { .. }
                | ApplicationError::OperationTimeout { .. }
                | ApplicationError::ConcurrentModification { .. }
                | ApplicationError::RateLimitExceeded { .. }
        )
    }
}

// Convenient conversion from anyhow::Error for gradual migration
impl From<anyhow::Error> for ApplicationError {
    fn from(err: anyhow::Error) -> Self {
        ApplicationError::Other(err.to_string())
    }
}

impl From<anyhow::Error> for InfrastructureError {
    fn from(err: anyhow::Error) -> Self {
        InfrastructureError::Other(err.to_string())
    }
}
```

**Verification**:
```bash
cargo check -p wrldbldr-engine-ports
```

**Time**: 4 hours

---

#### Step 3: Migrate Challenge System as Vertical Slice (8 hours)

Start with one complete vertical slice to validate the approach before wider rollout.

**Files to Modify**:

1. **Repository Port** - `crates/engine-ports/src/outbound/repository_port.rs`
   - Change `ChallengeRepositoryPort` methods from `Result<T>` to `Result<T, ApplicationError>`

2. **Repository Implementation** - `crates/engine-adapters/src/infrastructure/persistence/challenge_repository.rs`
   - Map Neo4j errors to `InfrastructureError::DatabaseQuery`
   - Map JSON parsing errors to `InfrastructureError::Deserialization`

3. **Service Layer** - `crates/engine-app/src/application/services/challenge_service.rs`
   - Update service trait and implementation
   - Add proper error context

4. **HTTP Routes** - `crates/engine-adapters/src/infrastructure/http/challenge_routes.rs`
   - Use `ApplicationError::status_code()` for HTTP response mapping

**Example Migration Pattern**:

```rust
// BEFORE (challenge_repository.rs):
async fn get(&self, id: ChallengeId) -> Result<Option<Challenge>> {
    let q = query("MATCH (c:Challenge {id: $id}) RETURN c")
        .param("id", id.to_string());
    
    let mut result = self.connection.graph().execute(q).await?;
    
    if let Some(row) = result.next().await? {
        Ok(Some(row_to_challenge(row)?))
    } else {
        Ok(None)
    }
}

// AFTER:
async fn get(&self, id: ChallengeId) -> Result<Option<Challenge>, ApplicationError> {
    let q = query("MATCH (c:Challenge {id: $id}) RETURN c")
        .param("id", id.to_string());
    
    let mut result = self.connection.graph().execute(q).await
        .map_err(|e| InfrastructureError::DatabaseQuery(
            format!("Failed to fetch challenge {}: {}", id, e)
        ))?;
    
    if let Some(row) = result.next().await
        .map_err(|e| InfrastructureError::DatabaseQuery(e.to_string()))? 
    {
        Ok(Some(row_to_challenge(row)
            .map_err(|e| InfrastructureError::Deserialization(e.to_string()))?))
    } else {
        Ok(None)
    }
}

// HTTP route (challenge_routes.rs):
// BEFORE:
.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?

// AFTER:
.map_err(|e: ApplicationError| {
    (StatusCode::from_u16(e.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), 
     e.to_string())
})?
```

**Verification**:
```bash
cargo check --workspace
cargo test -p wrldbldr-engine-adapters
# Manual test: Create, update, delete challenges via API
```

**Time**: 8 hours

---

#### Step 4: Create Error Handling Utilities (4 hours)

**File**: NEW `crates/engine-adapters/src/infrastructure/http/error_handling.rs`

```rust
//! HTTP error handling utilities
//!
//! Provides consistent error responses and logging for all HTTP endpoints.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::error;

use wrldbldr_engine_ports::errors::ApplicationError;

/// Standard error response format
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// HTTP status code
    pub status: u16,
    /// Error code for client handling
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Additional error details (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Request ID for support (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl IntoResponse for ApplicationError {
    fn into_response(self) -> Response {
        let status_code = self.status_code();
        let error_code = derive_error_code(&self);
        
        // Log server errors
        if status_code >= 500 {
            error!(
                error_code = %error_code,
                status = status_code,
                "Server error: {}", self
            );
        }
        
        let response = ErrorResponse {
            status: status_code,
            code: error_code,
            message: self.to_string(),
            details: None,
            request_id: None,
        };
        
        let status = StatusCode::from_u16(status_code)
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        
        (status, Json(response)).into_response()
    }
}

/// Derive a machine-readable error code from the error type
fn derive_error_code(error: &ApplicationError) -> String {
    match error {
        ApplicationError::Domain(de) => {
            if de.is_not_found() {
                "NOT_FOUND".to_string()
            } else if de.is_validation_error() {
                "VALIDATION_ERROR".to_string()
            } else if de.is_business_rule_violation() {
                "BUSINESS_RULE_VIOLATION".to_string()
            } else {
                "DOMAIN_ERROR".to_string()
            }
        }
        ApplicationError::InvalidInput { .. } => "INVALID_INPUT".to_string(),
        ApplicationError::MissingParameter(_) => "MISSING_PARAMETER".to_string(),
        ApplicationError::Unauthorized { .. } => "UNAUTHORIZED".to_string(),
        ApplicationError::Forbidden { .. } => "FORBIDDEN".to_string(),
        ApplicationError::StateConflict { .. } => "STATE_CONFLICT".to_string(),
        ApplicationError::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED".to_string(),
        ApplicationError::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE".to_string(),
        ApplicationError::OperationTimeout { .. } => "TIMEOUT".to_string(),
        ApplicationError::Infrastructure(_) => "INFRASTRUCTURE_ERROR".to_string(),
        _ => "INTERNAL_ERROR".to_string(),
    }
}

/// Extension trait for Result types with ApplicationError
pub trait ResultExt<T> {
    /// Add context to an error
    fn with_context(self, context: impl FnOnce() -> String) -> Result<T, ApplicationError>;
}

impl<T, E: Into<ApplicationError>> ResultExt<T> for Result<T, E> {
    fn with_context(self, context: impl FnOnce() -> String) -> Result<T, ApplicationError> {
        self.map_err(|e| {
            let err: ApplicationError = e.into();
            // Wrap with context if needed
            ApplicationError::Other(format!("{}: {}", context(), err))
        })
    }
}
```

**Verification**:
```bash
cargo check -p wrldbldr-engine-adapters
```

**Time**: 4 hours

---

### Success Criteria

- [ ] `DomainError`, `ApplicationError`, `InfrastructureError` types defined
- [ ] Challenge system fully migrated to typed errors
- [ ] HTTP responses use consistent error format
- [ ] Error codes map correctly to HTTP status codes
- [ ] Server errors (5xx) are logged with context
- [ ] Validation errors (4xx) include field information
- [ ] `anyhow::Error` can still be converted for gradual migration

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking changes during migration | High | Medium | Migrate one vertical slice first |
| Increased boilerplate | Medium | Low | Create helper macros/extensions |
| Error message exposure | Medium | Medium | Sanitize internal details in production |
| Incomplete migration | Medium | Low | Keep `From<anyhow::Error>` impls for fallback |

### Rollout Plan

1. **Week 1**: Define error types, migrate Challenge system
2. **Week 2**: Migrate Scene and Character systems
3. **Week 3**: Migrate remaining services
4. **Week 4**: Remove `anyhow` from critical paths, update documentation

---

## P3.4: Testing Infrastructure (1-2 weeks)

### Problem Statement

The codebase has **zero automated tests**:
- No unit tests for domain entities
- No integration tests for repositories
- No API tests for HTTP endpoints
- All testing is manual

This creates significant risk for:
- Regressions during refactoring
- Inconsistent behavior after changes
- Slow development velocity

### Current State Analysis

**Existing Test Patterns**: None in project crates (only external dependencies have tests)

**Testing Dependencies Available** (`Cargo.toml`):
- `tokio = { version = "1.42", features = ["full"] }` - Async test runtime
- No test utilities currently defined

**Crate Structure** (test layers):

```
┌─────────────────────────────────────────────────────────────┐
│  API Tests (engine-adapters)                                │
│  - HTTP endpoint tests                                       │
│  - WebSocket message tests                                   │
└─────────────────────────────┬───────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│  Integration Tests (engine-app)                              │
│  - Service layer tests with mocked repositories              │
│  - Cross-service interaction tests                           │
└─────────────────────────────┬───────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│  Unit Tests (domain)                                         │
│  - Entity validation tests                                   │
│  - Value object tests                                        │
│  - Business logic tests                                      │
└─────────────────────────────────────────────────────────────┘
```

### Implementation Plan

#### Phase 1: Test Infrastructure Setup (4 hours)

**Step 1.1: Add Test Dependencies**

**File**: MODIFY `Cargo.toml`

```toml
[workspace.dependencies]
# Testing
mockall = "0.12"
fake = { version = "2.9", features = ["derive", "chrono", "uuid"] }
rstest = "0.18"
test-case = "3.3"
axum-test = "14.0"
wiremock = "0.6"

# Test containers for Neo4j
testcontainers = "0.18"
testcontainers-modules = { version = "0.6", features = ["neo4j"] }
```

**Step 1.2: Create Test Utilities Crate**

**File**: NEW `crates/test-utils/Cargo.toml`

```toml
[package]
name = "wrldbldr-test-utils"
version.workspace = true
edition.workspace = true

[dependencies]
wrldbldr-domain.workspace = true
wrldbldr-engine-ports.workspace = true

fake = { workspace = true }
uuid.workspace = true
chrono.workspace = true

[dev-dependencies]
tokio.workspace = true
```

**File**: NEW `crates/test-utils/src/lib.rs`

```rust
//! Test utilities for WrldBldr
//!
//! Provides fixtures, factories, and helpers for testing.

pub mod fixtures;
pub mod factories;
pub mod assertions;
```

**Time**: 2 hours

---

**Step 1.3: Create Entity Fixtures**

**File**: NEW `crates/test-utils/src/fixtures.rs`

```rust
//! Test fixtures for common entities

use fake::{Fake, Faker};
use wrldbldr_domain::{
    entities::{Challenge, Character, Location, Scene, World},
    ChallengeId, CharacterId, LocationId, SceneId, WorldId,
};

/// Create a test world with default values
pub fn test_world() -> World {
    World {
        id: WorldId::new(),
        name: Faker.fake(),
        description: Faker.fake(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        // ... other fields
    }
}

/// Create a test character in the given world
pub fn test_character(world_id: WorldId) -> Character {
    Character {
        id: CharacterId::new(),
        world_id,
        name: Faker.fake(),
        // ... other fields
    }
}

/// Create a test challenge in the given world
pub fn test_challenge(world_id: WorldId) -> Challenge {
    Challenge::new(
        world_id,
        Faker.fake::<String>(),
        wrldbldr_domain::entities::Difficulty::DC(15),
    )
}

/// Create a test location in the given world
pub fn test_location(world_id: WorldId) -> Location {
    Location {
        id: LocationId::new(),
        world_id,
        name: Faker.fake(),
        description: Faker.fake(),
        // ... other fields
    }
}

/// Create a test scene at the given location
pub fn test_scene(world_id: WorldId, location_id: LocationId) -> Scene {
    Scene {
        id: SceneId::new(),
        world_id,
        name: Faker.fake(),
        // ... other fields
    }
}
```

**Time**: 2 hours

---

#### Phase 2: Domain Unit Tests (1 day)

**Step 2.1: Challenge Entity Tests**

**File**: NEW `crates/domain/src/entities/challenge_tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_domain::WorldId;

    #[test]
    fn new_challenge_has_generated_id() {
        let world_id = WorldId::new();
        let challenge = Challenge::new(world_id, "Test Challenge", Difficulty::DC(15));
        
        assert!(!challenge.id.to_string().is_empty());
        assert_eq!(challenge.world_id, world_id);
        assert_eq!(challenge.name, "Test Challenge");
        assert_eq!(challenge.difficulty, Difficulty::DC(15));
        assert!(challenge.active);
    }

    #[test]
    fn challenge_builder_pattern_works() {
        let world_id = WorldId::new();
        let challenge = Challenge::new(world_id, "Test", Difficulty::DC(10))
            .with_description("A test challenge")
            .with_challenge_type(ChallengeType::AbilityCheck)
            .with_tag("combat");
        
        assert_eq!(challenge.description, "A test challenge");
        assert_eq!(challenge.challenge_type, ChallengeType::AbilityCheck);
        assert!(challenge.tags.contains(&"combat".to_string()));
    }

    #[test]
    fn difficulty_dc_comparison() {
        assert!(matches!(Difficulty::DC(15), Difficulty::DC(15)));
        assert!(!matches!(Difficulty::DC(15), Difficulty::DC(20)));
    }

    #[test]
    fn difficulty_percentage_valid_range() {
        let valid = Difficulty::Percentage(50);
        assert!(matches!(valid, Difficulty::Percentage(50)));
        
        // Percentage should be 0-100
        let edge_low = Difficulty::Percentage(0);
        let edge_high = Difficulty::Percentage(100);
        assert!(matches!(edge_low, Difficulty::Percentage(0)));
        assert!(matches!(edge_high, Difficulty::Percentage(100)));
    }

    #[test]
    fn trigger_condition_matches_action() {
        let trigger = TriggerCondition::Action {
            action: "search".to_string(),
            context: Some("room".to_string()),
        };
        
        assert!(trigger.matches("search", "room"));
        assert!(!trigger.matches("examine", "room"));
        assert!(!trigger.matches("search", "corridor"));
    }
}
```

**Step 2.2: NarrativeEvent Trigger Tests**

**File**: NEW `crates/domain/src/entities/narrative_event_tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn trigger_evaluation_all_logic() {
        let world_id = WorldId::new();
        let mut event = NarrativeEvent::new(world_id, "Test Event");
        event.trigger_logic = TriggerLogic::All;
        event.trigger_conditions = vec![
            NarrativeTrigger {
                trigger_id: "1".to_string(),
                trigger_type: NarrativeTriggerType::FlagSet { 
                    flag_name: "flag_a".to_string() 
                },
                description: "Flag A must be set".to_string(),
                is_required: false,
            },
            NarrativeTrigger {
                trigger_id: "2".to_string(),
                trigger_type: NarrativeTriggerType::FlagSet { 
                    flag_name: "flag_b".to_string() 
                },
                description: "Flag B must be set".to_string(),
                is_required: false,
            },
        ];

        // Both flags set - should trigger
        let mut context = TriggerContext::default();
        context.flags.insert("flag_a".to_string(), true);
        context.flags.insert("flag_b".to_string(), true);
        let eval = event.evaluate_triggers(&context);
        assert!(eval.is_triggered);
        assert_eq!(eval.matched_triggers.len(), 2);

        // Only one flag set - should NOT trigger with All logic
        context.flags.remove("flag_b");
        let eval = event.evaluate_triggers(&context);
        assert!(!eval.is_triggered);
        assert_eq!(eval.matched_triggers.len(), 1);
    }

    #[test]
    fn trigger_evaluation_any_logic() {
        let world_id = WorldId::new();
        let mut event = NarrativeEvent::new(world_id, "Test Event");
        event.trigger_logic = TriggerLogic::Any;
        event.trigger_conditions = vec![
            NarrativeTrigger {
                trigger_id: "1".to_string(),
                trigger_type: NarrativeTriggerType::FlagSet { 
                    flag_name: "flag_a".to_string() 
                },
                description: "Flag A".to_string(),
                is_required: false,
            },
            NarrativeTrigger {
                trigger_id: "2".to_string(),
                trigger_type: NarrativeTriggerType::FlagSet { 
                    flag_name: "flag_b".to_string() 
                },
                description: "Flag B".to_string(),
                is_required: false,
            },
        ];

        // Only one flag set - should trigger with Any logic
        let mut context = TriggerContext::default();
        context.flags.insert("flag_a".to_string(), true);
        let eval = event.evaluate_triggers(&context);
        assert!(eval.is_triggered);
        assert_eq!(eval.matched_triggers.len(), 1);
    }

    #[test]
    fn trigger_evaluation_at_least_n_logic() {
        let world_id = WorldId::new();
        let mut event = NarrativeEvent::new(world_id, "Test Event");
        event.trigger_logic = TriggerLogic::AtLeast(2);
        event.trigger_conditions = vec![
            // 3 trigger conditions
            NarrativeTrigger {
                trigger_id: "1".to_string(),
                trigger_type: NarrativeTriggerType::FlagSet { 
                    flag_name: "flag_a".to_string() 
                },
                description: "".to_string(),
                is_required: false,
            },
            NarrativeTrigger {
                trigger_id: "2".to_string(),
                trigger_type: NarrativeTriggerType::FlagSet { 
                    flag_name: "flag_b".to_string() 
                },
                description: "".to_string(),
                is_required: false,
            },
            NarrativeTrigger {
                trigger_id: "3".to_string(),
                trigger_type: NarrativeTriggerType::FlagSet { 
                    flag_name: "flag_c".to_string() 
                },
                description: "".to_string(),
                is_required: false,
            },
        ];

        // 1 of 3 - should NOT trigger
        let mut context = TriggerContext::default();
        context.flags.insert("flag_a".to_string(), true);
        let eval = event.evaluate_triggers(&context);
        assert!(!eval.is_triggered);

        // 2 of 3 - should trigger
        context.flags.insert("flag_b".to_string(), true);
        let eval = event.evaluate_triggers(&context);
        assert!(eval.is_triggered);
    }

    #[test]
    fn required_trigger_must_match() {
        let world_id = WorldId::new();
        let mut event = NarrativeEvent::new(world_id, "Test Event");
        event.trigger_logic = TriggerLogic::AtLeast(1);
        event.trigger_conditions = vec![
            NarrativeTrigger {
                trigger_id: "1".to_string(),
                trigger_type: NarrativeTriggerType::FlagSet { 
                    flag_name: "optional_flag".to_string() 
                },
                description: "".to_string(),
                is_required: false,
            },
            NarrativeTrigger {
                trigger_id: "2".to_string(),
                trigger_type: NarrativeTriggerType::FlagSet { 
                    flag_name: "required_flag".to_string() 
                },
                description: "".to_string(),
                is_required: true,  // REQUIRED
            },
        ];

        // Optional matches but required doesn't - should NOT trigger
        let mut context = TriggerContext::default();
        context.flags.insert("optional_flag".to_string(), true);
        let eval = event.evaluate_triggers(&context);
        assert!(!eval.is_triggered);

        // Required matches - should trigger
        context.flags.insert("required_flag".to_string(), true);
        let eval = event.evaluate_triggers(&context);
        assert!(eval.is_triggered);
    }
}
```

**Time**: 8 hours

---

#### Phase 3: Repository Integration Tests (2 days)

**Step 3.1: Set Up Test Containers**

**File**: NEW `crates/engine-adapters/tests/common/mod.rs`

```rust
//! Common test utilities for integration tests

use testcontainers::{clients::Cli, Container};
use testcontainers_modules::neo4j::Neo4j;
use std::sync::Arc;

use wrldbldr_engine_adapters::infrastructure::persistence::{
    Neo4jConnection, Neo4jRepository,
};

/// Shared Docker client (expensive to create)
pub fn docker_client() -> Cli {
    Cli::default()
}

/// Start Neo4j container and return connection
pub async fn setup_neo4j_test_db(docker: &Cli) -> (Container<'_, Neo4j>, Arc<Neo4jRepository>) {
    let neo4j = docker.run(Neo4j::default());
    
    let bolt_url = format!(
        "bolt://127.0.0.1:{}",
        neo4j.get_host_port_ipv4(7687)
    );
    
    let connection = Neo4jConnection::new(&bolt_url, "neo4j", "neo4j")
        .await
        .expect("Failed to connect to Neo4j");
    
    let repo = Arc::new(Neo4jRepository::new(connection));
    
    (neo4j, repo)
}

/// Clean up test data between tests
pub async fn cleanup_test_data(repo: &Neo4jRepository) {
    repo.execute_raw("MATCH (n) DETACH DELETE n")
        .await
        .expect("Failed to clean test data");
}
```

**Step 3.2: Challenge Repository Tests**

**File**: NEW `crates/engine-adapters/tests/challenge_repository_test.rs`

```rust
mod common;

use wrldbldr_domain::{entities::Challenge, WorldId};
use wrldbldr_engine_ports::outbound::ChallengeRepositoryPort;
use wrldbldr_test_utils::fixtures;

#[tokio::test]
async fn create_and_get_challenge() {
    let docker = common::docker_client();
    let (_container, repo) = common::setup_neo4j_test_db(&docker).await;
    
    // Create a world first
    let world = fixtures::test_world();
    repo.worlds().create(&world).await.unwrap();
    
    // Create challenge
    let challenge = fixtures::test_challenge(world.id);
    repo.challenges().create(&challenge).await.unwrap();
    
    // Retrieve it
    let retrieved = repo.challenges().get(challenge.id).await.unwrap();
    
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, challenge.id);
    assert_eq!(retrieved.name, challenge.name);
    assert_eq!(retrieved.world_id, world.id);
}

#[tokio::test]
async fn list_challenges_by_world() {
    let docker = common::docker_client();
    let (_container, repo) = common::setup_neo4j_test_db(&docker).await;
    
    let world = fixtures::test_world();
    repo.worlds().create(&world).await.unwrap();
    
    // Create multiple challenges
    for i in 0..5 {
        let mut challenge = fixtures::test_challenge(world.id);
        challenge.name = format!("Challenge {}", i);
        repo.challenges().create(&challenge).await.unwrap();
    }
    
    let challenges = repo.challenges().list_by_world(world.id).await.unwrap();
    
    assert_eq!(challenges.len(), 5);
}

#[tokio::test]
async fn delete_challenge() {
    let docker = common::docker_client();
    let (_container, repo) = common::setup_neo4j_test_db(&docker).await;
    
    let world = fixtures::test_world();
    repo.worlds().create(&world).await.unwrap();
    
    let challenge = fixtures::test_challenge(world.id);
    repo.challenges().create(&challenge).await.unwrap();
    
    // Verify it exists
    assert!(repo.challenges().get(challenge.id).await.unwrap().is_some());
    
    // Delete it
    repo.challenges().delete(challenge.id).await.unwrap();
    
    // Verify it's gone
    assert!(repo.challenges().get(challenge.id).await.unwrap().is_none());
}

#[tokio::test]
async fn challenge_skill_edge() {
    let docker = common::docker_client();
    let (_container, repo) = common::setup_neo4j_test_db(&docker).await;
    
    let world = fixtures::test_world();
    repo.worlds().create(&world).await.unwrap();
    
    let skill = fixtures::test_skill(world.id);
    repo.skills().create(&skill).await.unwrap();
    
    let challenge = fixtures::test_challenge(world.id);
    repo.challenges().create(&challenge).await.unwrap();
    
    // Set skill edge
    repo.challenges()
        .set_required_skill(challenge.id, skill.id)
        .await
        .unwrap();
    
    // Verify edge exists
    let linked_skill = repo.challenges()
        .get_required_skill(challenge.id)
        .await
        .unwrap();
    
    assert_eq!(linked_skill, Some(skill.id));
}
```

**Time**: 16 hours

---

#### Phase 4: API Tests (2 days)

**Step 4.1: Set Up axum-test**

**File**: NEW `crates/engine-adapters/tests/api/mod.rs`

```rust
//! API test utilities

use axum_test::TestServer;
use std::sync::Arc;

use wrldbldr_engine_adapters::infrastructure::state::AppState;
use wrldbldr_engine_adapters::run::create_router;

pub async fn test_server() -> TestServer {
    // Use in-memory mocks for API tests (faster than testcontainers)
    let state = create_test_app_state().await;
    let router = create_router(state);
    
    TestServer::new(router).unwrap()
}

async fn create_test_app_state() -> Arc<AppState> {
    // Create mock repositories and services
    // ...
}
```

**Step 4.2: Challenge API Tests**

**File**: NEW `crates/engine-adapters/tests/api/challenge_api_test.rs`

```rust
mod common;

use axum::http::StatusCode;
use serde_json::json;
use wrldbldr_test_utils::fixtures;

#[tokio::test]
async fn get_nonexistent_challenge_returns_404() {
    let server = common::test_server().await;
    
    let response = server
        .get("/api/worlds/00000000-0000-0000-0000-000000000000/challenges/00000000-0000-0000-0000-000000000001")
        .await;
    
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_challenge_returns_201() {
    let server = common::test_server().await;
    
    // First create a world
    let world_response = server
        .post("/api/worlds")
        .json(&json!({
            "name": "Test World",
            "description": "A test world"
        }))
        .await;
    
    assert_eq!(world_response.status_code(), StatusCode::CREATED);
    let world: serde_json::Value = world_response.json();
    let world_id = world["id"].as_str().unwrap();
    
    // Then create challenge
    let response = server
        .post(&format!("/api/worlds/{}/challenges", world_id))
        .json(&json!({
            "name": "Test Challenge",
            "description": "A test challenge",
            "difficulty": { "dc": 15 }
        }))
        .await;
    
    assert_eq!(response.status_code(), StatusCode::CREATED);
    let challenge: serde_json::Value = response.json();
    assert_eq!(challenge["name"], "Test Challenge");
}

#[tokio::test]
async fn invalid_world_id_returns_400() {
    let server = common::test_server().await;
    
    let response = server
        .get("/api/worlds/not-a-uuid/challenges")
        .await;
    
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_challenge_validates_input() {
    let server = common::test_server().await;
    
    // Create world first
    let world_response = server
        .post("/api/worlds")
        .json(&json!({ "name": "Test", "description": "" }))
        .await;
    let world: serde_json::Value = world_response.json();
    let world_id = world["id"].as_str().unwrap();
    
    // Try to create challenge with missing required field
    let response = server
        .post(&format!("/api/worlds/{}/challenges", world_id))
        .json(&json!({
            "description": "Missing name"
        }))
        .await;
    
    assert_eq!(response.status_code(), StatusCode::BAD_REQUEST);
}
```

**Time**: 16 hours

---

### Success Criteria

- [ ] `cargo test --workspace` runs without errors
- [ ] Domain layer has 80%+ test coverage for entities
- [ ] Repository integration tests cover CRUD + edge operations
- [ ] API tests verify status codes and response formats
- [ ] Test fixtures make it easy to create test data
- [ ] CI pipeline runs tests on every PR

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Test containers slow | High | Medium | Use in-memory mocks for unit tests, containers only for integration |
| Neo4j container availability | Medium | High | Fallback to mock repositories |
| Flaky tests | Medium | Medium | Use proper test isolation, deterministic data |
| Large test suite maintenance | Low | Medium | Focus on critical paths first |

### Verification Commands

```bash
# Run all tests
cargo test --workspace

# Run domain unit tests only
cargo test -p wrldbldr-domain

# Run integration tests (requires Docker)
cargo test -p wrldbldr-engine-adapters --test '*'

# Run with verbose output
cargo test --workspace -- --nocapture

# Generate coverage report (requires cargo-llvm-cov)
cargo llvm-cov --workspace --html
```

---

## Dependencies Between Tasks

```
P3.4 Testing Infrastructure ──┐
         │                    │
         ▼                    │
P3.3 Typed Error Handling ────┼── (Can start independently)
         │                    │
         │                    │
P3.1 Trigger Builder ─────────┤
         │                    │
         ▼                    │
P3.2 Workflow Editor ─────────┘
```

**Recommended Order**:
1. **P3.4** - Start testing infrastructure early (long lead time)
2. **P3.3** - Typed errors (enables better test assertions)
3. **P3.1** - Trigger builder (high user impact)
4. **P3.2** - Workflow editor (polish feature)

---

## Progress Log

| Date | Task | Status | Notes |
|------|------|--------|-------|
| 2025-12-26 | Sprint 6 Tier 3 Planning | Complete | Created this document |

---

## Appendix A: File Reference

### P3.1 Files
| File | Action | Lines |
|------|--------|-------|
| `crates/domain/src/entities/narrative_event.rs` | Reference | 114-219 |
| `crates/engine-adapters/src/infrastructure/http/trigger_routes.rs` | NEW | ~200 |
| `crates/engine-adapters/src/infrastructure/http/mod.rs` | MODIFY | +5 |
| `crates/player-ui/src/presentation/components/story_arc/trigger_builder.rs` | NEW | ~500 |
| `crates/player-ui/src/presentation/components/story_arc/narrative_event_card.rs` | MODIFY | ~50 |

### P3.2 Files
| File | Action | Lines |
|------|--------|-------|
| `crates/player-ui/src/presentation/components/settings/workflow_config_editor.rs` | MODIFY | ~200 |
| `crates/player-app/src/application/services/workflow_service.rs` | MODIFY | ~50 |

### P3.3 Files
| File | Action | Lines |
|------|--------|-------|
| `crates/domain/src/errors.rs` | NEW | ~150 |
| `crates/engine-ports/src/errors.rs` | NEW | ~200 |
| `crates/engine-adapters/src/infrastructure/http/error_handling.rs` | NEW | ~100 |
| `crates/engine-ports/src/outbound/repository_port.rs` | MODIFY | ~50 |
| `crates/engine-adapters/src/infrastructure/persistence/challenge_repository.rs` | MODIFY | ~100 |
| `crates/engine-app/src/application/services/challenge_service.rs` | MODIFY | ~50 |
| `crates/engine-adapters/src/infrastructure/http/challenge_routes.rs` | MODIFY | ~50 |

### P3.4 Files
| File | Action | Lines |
|------|--------|-------|
| `Cargo.toml` | MODIFY | +10 |
| `crates/test-utils/Cargo.toml` | NEW | ~20 |
| `crates/test-utils/src/lib.rs` | NEW | ~20 |
| `crates/test-utils/src/fixtures.rs` | NEW | ~100 |
| `crates/domain/src/entities/challenge_tests.rs` | NEW | ~100 |
| `crates/domain/src/entities/narrative_event_tests.rs` | NEW | ~150 |
| `crates/engine-adapters/tests/common/mod.rs` | NEW | ~50 |
| `crates/engine-adapters/tests/challenge_repository_test.rs` | NEW | ~150 |
| `crates/engine-adapters/tests/api/mod.rs` | NEW | ~50 |
| `crates/engine-adapters/tests/api/challenge_api_test.rs` | NEW | ~100 |
