**Created:** January 22, 2026
**Status:** Draft spec
**Owner:** OpenCode
**Scope:** Visual State Catalog + generation workflow protocol

---

## Summary

Defines protocol contracts and endpoints for:
- Visual state catalog (list/get/create/update/delete)
- Active state management
- Visual state preview
- Visual state generation (asset workflow)

This spec follows existing request/response patterns in `shared` and the asset generation pipeline.

---

## Shared Protocol (Draft)

### Request Payloads

Add a new request group for visual states or new request enums for location/region state operations.

Example (new group):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VisualStateRequest {
    GetCatalog {
        location_id: String,
        region_id: String,
    },
    GetLocationState {
        location_state_id: String,
    },
    GetRegionState {
        region_state_id: String,
    },
    CreateLocationState {
        location_id: String,
        data: CreateLocationStateData,
    },
    CreateRegionState {
        region_id: String,
        data: CreateRegionStateData,
    },
    UpdateLocationState {
        location_state_id: String,
        data: UpdateLocationStateData,
    },
    UpdateRegionState {
        region_state_id: String,
        data: UpdateRegionStateData,
    },
    DeleteLocationState {
        location_state_id: String,
    },
    DeleteRegionState {
        region_state_id: String,
    },
    SetActiveLocationState {
        location_id: String,
        location_state_id: String,
    },
    SetActiveRegionState {
        region_id: String,
        region_state_id: String,
    },
    PreviewLocationState {
        location_id: String,
        context: StatePreviewContext,
    },
    PreviewRegionState {
        region_id: String,
        context: StatePreviewContext,
    },
    GenerateVisualState {
        request: GenerateVisualStateRequest,
    },
}
```

### Data Types (Draft)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLocationStateData {
    pub name: String,
    pub description: Option<String>,
    pub backdrop_override: Option<String>,
    pub atmosphere_override: Option<String>,
    pub ambient_sound: Option<String>,
    pub map_overlay: Option<String>,
    pub activation_rules: Vec<ActivationRuleData>,
    pub activation_logic: ActivationLogicData,
    pub priority: i32,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRegionStateData {
    pub name: String,
    pub description: Option<String>,
    pub backdrop_override: Option<String>,
    pub atmosphere_override: Option<String>,
    pub ambient_sound: Option<String>,
    pub activation_rules: Vec<ActivationRuleData>,
    pub activation_logic: ActivationLogicData,
    pub priority: i32,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatePreviewContext {
    pub game_time_seconds: i64,
    pub world_flags: Vec<String>,
    pub pc_flags: Vec<String>,
    pub triggered_events: Vec<String>,
    pub present_characters: Vec<String>,
    pub evaluate_soft_rules: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateVisualStateRequest {
    pub location_id: String,
    pub region_id: String,
    pub state_type: StateType,
    pub name: String,
    pub description: String,
    pub prompt: String,
    pub workflow: String,
    pub negative_prompt: Option<String>,
    pub tags: Vec<String>,
    pub generate_backdrop: bool,
    pub generate_atmosphere_text: bool,
    pub generate_ambient_sound: bool,
}
```

### Server Messages

```rust
pub enum ServerMessage {
    VisualStateCatalog(VisualStateCatalogData),
    GeneratedVisualState(GeneratedVisualStateData),
    VisualStateDetails {
        location_state: Option<LocationStateData>,
        region_state: Option<RegionStateData>,
    },
}
```

### Catalog Data

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualStateCatalogData {
    pub location_id: String,
    pub region_id: String,
    pub location_states: Vec<LocationStateData>,
    pub region_states: Vec<RegionStateData>,
}
```

---

## Engine (Draft)

### Use Case

`engine/src/use_cases/visual_state/catalog.rs`
- List location/region states
- Get specific state details
- Create/update/delete state
- Set active state
- Preview resolution

### WebSocket Handlers

`engine/src/api/websocket/ws_visual_state.rs`
- Handles VisualStateRequest
- DM validation for create/update/delete/generate

### Generation Flow

Generation uses existing asset pipeline:
- `use_cases/assets` for `GenerateAsset`
- `GalleryAsset` + `GenerationBatch`

---

## Player (Draft)

### Services

`player/src/application/services/visual_state_service.rs`
- Fetch catalog
- Request details
- Generate new state

### UI

See `docs/designs/visual-state-catalog-ui.md` for mockups and interaction notes.

---

## Notes

- This draft assumes VisualState IDs are derived from prompt/workflow hash.
- If deterministic IDs require domain changes, extend `LocationStateId`/`RegionStateId` creation helpers.
