**Created:** January 22, 2026
**Status:** Implemented (WebSocket only)
**Owner:** OpenCode
**Scope:** Visual State Catalog + generation workflow protocol

---

## Summary

Defines protocol contracts and endpoints for:
- Visual state catalog (list/get/create/update/delete)
- Active state management
- Visual state preview
- Visual state generation (asset workflow)

**Implementation Status:**
- ✅ Protocol contracts (`shared/src/requests/visual_state.rs`)
- ✅ WebSocket messages and handlers (`engine/src/api/websocket/ws_visual_state.rs`)
- ✅ Use case implementation (`engine/src/use_cases/visual_state/catalog.rs`)
- ❌ REST endpoints (WebSocket-only API currently)

---

## Implemented Protocol

**File:** `crates/shared/src/requests/visual_state.rs`

### Request Payloads (Implemented)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VisualStateRequest {
    /// Get visual state catalog for location/region
    GetCatalog {
        request: GetVisualStateCatalogRequest,
    },
    /// Get details of a specific visual state
    GetDetails {
        request: GetVisualStateDetailsRequest,
    },
    /// Create a new visual state
    Create { request: CreateVisualStateRequest },
    /// Update an existing visual state
    Update { request: UpdateVisualStateRequest },
    /// Delete a visual state
    Delete { request: DeleteVisualStateRequest },
    /// Set active visual state for location/region
    SetActive {
        request: SetActiveVisualStateRequest,
    },
    /// Generate a new visual state with assets
    Generate { request: GenerateVisualStateRequest },
}
```

### Key Request Types (Implemented)

```rust
/// Request to list all available visual states for a location/region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetVisualStateCatalogRequest {
    pub location_id: Option<Uuid>,
    pub region_id: Option<Uuid>,
}

/// Request to create a new visual state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVisualStateRequest {
    pub state_type: VisualStateType,  // Location | Region
    pub location_id: Option<Uuid>,
    pub region_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub backdrop_asset: Option<String>,
    pub atmosphere: Option<String>,
    pub ambient_sound: Option<String>,
    pub map_overlay: Option<String>,
    pub activation_rules: Option<serde_json::Value>,
    pub activation_logic: Option<String>,  // "All" | "Any" | "AtLeast"
    pub priority: i32,
    pub is_default: bool,
}

/// Request to generate a new visual state with assets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateVisualStateRequest {
    pub state_type: VisualStateType,
    pub location_id: Option<Uuid>,
    pub region_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub workflow: String,
    pub negative_prompt: Option<String>,
    pub tags: Vec<String>,
    pub generate_backdrop: bool,
    pub generate_map: bool,
    pub activation_rules: Option<serde_json::Value>,
    pub activation_logic: Option<String>,
    pub priority: i32,
    pub is_default: bool,
}
```

### Response Data Types (Implemented)

```rust
/// Visual state catalog data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualStateCatalogData {
    pub location_states: Vec<LocationStateData>,
    pub region_states: Vec<RegionStateData>,
}

/// Generated visual state result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedVisualStateData {
    pub location_state: Option<LocationStateData>,
    pub region_state: Option<RegionStateData>,
    pub generation_batch_id: String,
    pub is_complete: bool,
}

/// Location state data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationStateData {
    pub id: Uuid,
    pub location_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub backdrop_override: Option<String>,
    pub atmosphere_override: Option<String>,
    pub ambient_sound: Option<String>,
    pub map_overlay: Option<String>,
    pub priority: i32,
    pub is_default: bool,
    pub is_active: bool,
    pub activation_rules: Option<serde_json::Value>,
    pub activation_logic: Option<String>,
    pub generation_prompt: Option<String>,
    pub workflow_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Region state data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionStateData {
    pub id: Uuid,
    pub region_id: Uuid,
    pub location_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub backdrop_override: Option<String>,
    pub atmosphere_override: Option<String>,
    pub ambient_sound: Option<String>,
    pub priority: i32,
    pub is_default: bool,
    pub is_active: bool,
    pub activation_rules: Option<serde_json::Value>,
    pub activation_logic: Option<String>,
    pub generation_prompt: Option<String>,
    pub workflow_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

---

## Engine Implementation

### Use Case

**File:** `crates/engine/src/use_cases/visual_state/catalog.rs`

The `VisualStateCatalog` use case provides:
- `get_catalog()` - List location/region states for a given location/region
- `get_details()` - Get details of a specific visual state
- `create_location_state()` - Create new location state
- `create_region_state()` - Create new region state
- `update_location_state()` - Update location state with optional fields
- `update_region_state()` - Update region state with optional fields
- `delete()` - Delete a visual state
- `set_active()` - Set active visual state for location/region
- `generate_visual_state()` - Generate new visual state with asset generation

### WebSocket Handler

**File:** `crates/engine/src/api/websocket/ws_visual_state.rs`

Handles all `VisualStateRequest` variants:
- `handle_get_catalog()` - Returns catalog with available states
- `handle_get_details()` - Returns specific state details
- `handle_create_visual_state()` - Creates new state (DM only)
- `handle_update_visual_state()` - Updates existing state
- `handle_delete_visual_state()` - Deletes state (DM only)
- `handle_set_active_visual_state()` - Sets active state (DM only)
- `handle_generate_visual_state()` - Generates state and queues assets (DM only)

All handlers include:
- Input validation (field lengths, asset paths)
- DM permission checks for mutation operations
- Domain-to-protocol type conversion
- Error mapping to `ErrorCode`

### State Resolution

**File:** `crates/engine/src/use_cases/visual_state/resolve_state.rs`

The `ResolveVisualState` use case:
- Evaluates activation rules against a `StateResolutionContext`
- Returns `StateResolutionResult` with:
  - Resolved location/region states (best match by priority)
  - All available states with evaluation results
  - Pending soft rules requiring LLM evaluation
- Supports activation rules: `Always`, `DateExact`, `DateRange`, `TimeOfDay`, `EventTriggered`, `FlagSet`, `CharacterPresent`, `Custom` (soft rule)
- Activation logic: `All`, `Any`, `AtLeast(n)`

### Generation Flow

The `generate_visual_state()` method:
1. Validates location/region exists
2. Creates new state with provided parameters
3. Queues asset generation via `QueuePort` (if `generate_backdrop` is true)
4. Returns `GeneratedVisualState` with state data and `generation_batch_id`
5. Map generation is reserved for future implementation (`generate_map` flag exists but not used)

---

## Player Implementation

### UI Components

**File:** `crates/player/src/ui/presentation/components/dm_panel/`

- `visual_state_dropdown.rs` - Dropdown of available visual states with "Generate New" button
- `visual_state_details_modal.rs` - Shows full state details (assets, activation rules, prompt)
- `visual_state_generation_modal.rs` - Modal for generating new visual states with AI
- `visual_state_preview.rs` - Inline thumbnail and description display

**File:** `crates/player/src/ui/presentation/components/visual_novel/`

- `visual_state_indicator.rs` - Small overlay showing current visual state

### State Management

**File:** `crates/player/src/ui/presentation/state/game_state.rs`

- `visual_state_override` - Signal for staging-override visual state
- `set_visual_state_override()` - Apply visual state from staging approval
- `clear_visual_state_override()` - Clear override on scene change

---

## Implementation Files Reference

| Component | File |
|-----------|------|
| Protocol types | `crates/shared/src/requests/visual_state.rs` |
| Catalog use case | `crates/engine/src/use_cases/visual_state/catalog.rs` |
| Resolution use case | `crates/engine/src/use_cases/visual_state/resolve_state.rs` |
| WebSocket handler | `crates/engine/src/api/websocket/ws_visual_state.rs` |
| Visual state dropdown | `crates/player/src/ui/presentation/components/dm_panel/visual_state_dropdown.rs` |
| Visual state generation modal | `crates/player/src/ui/presentation/components/dm_panel/visual_state_generation_modal.rs` |
| Visual state details modal | `crates/player/src/ui/presentation/components/dm_panel/visual_state_details_modal.rs` |
| Visual state preview | `crates/player/src/ui/presentation/components/dm_panel/visual_state_preview.rs` |
| Visual state indicator | `crates/player/src/ui/presentation/components/visual_novel/visual_state_indicator.rs` |

---

## Future Work

1. **REST endpoints** - Add HTTP routes for catalog operations (currently WebSocket-only)
2. **Map asset generation** - Implement `generate_map` functionality in generation workflow
3. **Soft rule LLM evaluation** - Integrate LLM evaluation for custom activation rules
4. **Deterministic state IDs** - Consider hash-based IDs from prompt/workflow if needed
