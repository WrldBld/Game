# Asset System

## Overview

## Canonical vs Implementation

This document is canonical for how the system *should* behave in gameplay.
Implementation notes are included to track current status and may lag behind the spec.

**Legend**
- **Canonical**: Desired gameplay rule or behavior (source of truth)
- **Implemented**: Verified in code and wired end-to-end
- **Planned**: Designed but not fully implemented yet


The Asset System integrates with ComfyUI to generate images (character portraits, sprites, backdrops) on demand. It manages workflows, queues generation requests, tracks progress, and maintains a gallery of generated assets. The system includes resilience features like circuit breakers, retries, and health monitoring.

---

## Game Design

AI-generated art enhances immersion:

1. **On-Demand Generation**: Generate portraits for NPCs, backdrops for locations
2. **Workflow Flexibility**: Support multiple ComfyUI workflows for different asset types
3. **Style Consistency**: Style references ensure visual coherence
4. **Real-Time Progress**: WebSocket updates show generation status

---

## User Stories

### Implemented

- [x] **US-AST-001**: As a DM, I can upload ComfyUI workflows for different asset types
  - *Implementation*: WorkflowConfiguration entity with slot-based management
  - *Files*: `crates/domain/src/entities/workflow_config.rs`

- [x] **US-AST-002**: As a DM, I can queue image generation with prompts
  - *Implementation*: GenerationBatch entity, asset generation use cases
  - *Files*: `crates/engine/src/use_cases/assets/mod.rs`

- [x] **US-AST-003**: As a DM, I can see real-time generation progress
  - *Implementation*: GenerationQueued/Progress/Complete/Failed WebSocket messages
  - *Files*: `crates/player/src/ui/presentation/components/creator/generation_queue.rs`

- [x] **US-AST-004**: As a DM, I can browse generated assets in a gallery
  - *Implementation*: AssetGallery with filtering, thumbnails, context menu
  - *Files*: `crates/player/src/ui/presentation/components/creator/asset_gallery.rs`

- [x] **US-AST-005**: As a DM, I can set a generated asset as active for an entity
  - *Implementation*: Asset activation sets portrait_asset/sprite_asset/backdrop_asset
  - *Files*: `crates/engine/src/repositories/assets.rs`

- [x] **US-AST-006**: As a DM, I can cancel/retry/clear generation batches
  - *Implementation*: Batch management UI with status-appropriate actions
  - *Files*: `crates/player/src/ui/presentation/components/creator/generation_queue.rs`

- [x] **US-AST-007**: As a DM, I can use style references for consistent art
  - *Implementation*: StyleReferenceMapping, IPAdapter detection
  - *Files*: `crates/engine/src/infrastructure/comfyui.rs`

- [x] **US-AST-008**: As a DM, I can see ComfyUI connection status
  - *Implementation*: ComfyUIBanner component with health indicator
  - *Files*: `crates/player/src/ui/presentation/components/creator/comfyui_banner.rs`

- [x] **US-AST-009**: As a DM, I can quick-generate from Director Mode
  - *Implementation*: DirectorGenerateModal, generate buttons on NPC panel
  - *Files*: `crates/player/src/ui/presentation/components/dm_panel/director_generate_modal.rs`

- [x] **US-AST-010**: As a DM, I can edit workflow parameters in the UI
  - *Implementation*: Enhanced WorkflowConfigEditor with editable mappings, lock toggles, style detection
  - *Files*: `crates/player/src/ui/presentation/components/settings/workflow_config_editor.rs`

### Pending

---

## UI Mockups

### Asset Gallery

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Asset Gallery                                    [Filter ▼] [+ Generate]   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  Entity: Marcus the Bartender                                               │
│                                                                             │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐               │
│  │         │ │         │ │         │ │         │ │         │               │
│  │ [img1]  │ │ [img2]  │ │ [img3]  │ │ [img4]  │ │ [img5]  │               │
│  │         │ │   ★     │ │         │ │         │ │         │               │
│  │         │ │ ACTIVE  │ │         │ │         │ │         │               │
│  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘               │
│                                                                             │
│  Right-click menu:                                                          │
│  ┌─────────────────┐                                                        │
│  │ Set as Active   │                                                        │
│  │ Use as Style    │                                                        │
│  │ Download        │                                                        │
│  │ Delete          │                                                        │
│  └─────────────────┘                                                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented

### Generation Queue

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Generation Queue                                    [Clear All Completed]  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ● Processing   Marcus Portrait (2/4)                                │   │
│  │   ████████████░░░░░░░░░░░░░░░░░░ 50%                                │   │
│  │                                                      [Cancel]        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ○ Queued       Tavern Backdrop (0/2)                                │   │
│  │   Position: 2                                       [Cancel]        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ✓ Complete     Baron Portrait (4/4)                                 │   │
│  │   Generated 4 images                        [View] [Select] [Clear] │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │ ✗ Failed       Market Scene (0/2)                                   │   │
│  │   Error: ComfyUI timeout              [▼ Details] [Retry] [Clear]   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented

### Advanced Workflow Parameter Editor

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  Workflow: Portrait Generator (portrait slot)                       [Save]  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ─── Prompt Mappings ───────────────────────────────────────────────────── │
│  Map entity data to workflow input nodes:                                   │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ Node: "CLIPTextEncode.positive"                                       │ │
│  │ Source: [Entity Description ▼]                                        │ │
│  │ Template: "{description}, portrait, detailed face, high quality"      │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ Node: "CLIPTextEncode.negative"                                       │ │
│  │ Source: [Static Text ▼]                                               │ │
│  │ Template: "blurry, low quality, deformed"                             │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  [+ Add Mapping]                                                            │
│                                                                             │
│  ─── Locked Inputs ─────────────────────────────────────────────────────── │
│  Parameters that should not change:                                         │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ KSampler.steps: 30            KSampler.cfg: 7.5                       │ │
│  │ KSampler.sampler: "euler"     EmptyLatentImage.width: 512             │ │
│  │ EmptyLatentImage.height: 512                                          │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ─── Style Reference ───────────────────────────────────────────────────── │
│  ┌───────────────────────────────────────────────────────────────────────┐ │
│  │ Detection: [Auto-detect IPAdapter ▼]                                  │ │
│  │ Found node: IPAdapterApply at position 5                              │ │
│  └───────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ─── Raw JSON ──────────────────────────────────────────────────────────── │
│  [▶ Show Raw Workflow JSON]                                                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Status**: ✅ Implemented (US-AST-010)

---

## Data Model

### Neo4j Nodes

```cypher
(:GalleryAsset {
    id: "uuid",
    file_path: "/assets/generated/abc123.png",
    thumbnail_path: "/assets/generated/abc123_thumb.png",
    asset_type: "Portrait",
    entity_type: "Character",
    entity_id: "character-uuid",
    prompt: "A grizzled bartender with gray hair...",
    workflow_slot: "portrait",
    is_active: true,
    created_at: datetime()
})

(:GenerationBatch {
    id: "uuid",
    world_id: "world-uuid",
    entity_type: "Character",
    entity_id: "character-uuid",
    asset_type: "Portrait",
    prompt: "A grizzled bartender...",
    count: 4,
    status: "processing",
    completed_count: 2,
    error_message: null,
    created_at: datetime()
})

(:WorkflowConfiguration {
    id: "uuid",
    slot: "portrait",
    workflow_json: "{...}",
    prompt_mappings: "[...]",
    locked_inputs: "[...]",
    style_reference_mapping: "AutoDetect"
})
```

### Workflow Slots

| Slot | Purpose | Typical Workflow |
|------|---------|------------------|
| `portrait` | Character face portraits | Face-focused, square aspect |
| `sprite` | Character full-body sprites | Full body, transparent BG |
| `backdrop` | Location backgrounds | Wide aspect, environmental |
| `item` | Item illustrations | Object-focused |
| `custom` | User-defined | Any workflow |

### Batch Status

```rust
pub enum BatchStatus {
    Queued,
    Processing,
    Complete,
    Failed,
    Cancelled,
}
```

---

## API

### REST Endpoints

| Method | Path | Description | Status |
|--------|------|-------------|--------|
| GET | `/api/worlds/{id}/assets` | List assets | ✅ |
| POST | `/api/assets/generate` | Queue generation | ✅ |
| GET | `/api/assets/{id}` | Get asset | ✅ |
| DELETE | `/api/assets/{id}` | Delete asset | ✅ |
| PUT | `/api/assets/{id}/activate` | Set as active | ✅ |
| GET | `/api/assets/batches` | List batches | ✅ |
| DELETE | `/api/assets/batch/{id}` | Cancel batch | ✅ |
| POST | `/api/assets/batch/{id}/retry` | Retry batch | ✅ |
| GET | `/api/workflows` | List workflows | ✅ |
| PUT | `/api/workflows/{slot}` | Update workflow | ✅ |
| GET | `/api/status/comfyui` | ComfyUI health | ✅ |

### WebSocket Messages

#### Server → Client

| Message | Fields | Purpose |
|---------|--------|---------|
| `GenerationQueued` | `batch_id`, `position` | Batch queued |
| `GenerationProgress` | `batch_id`, `completed`, `total` | Progress update |
| `GenerationComplete` | `batch_id`, `assets` | Batch finished |
| `GenerationFailed` | `batch_id`, `error` | Batch failed |
| `ComfyUIStateChanged` | `state`, `message`, `retry_in` | Connection status |

---

## Implementation Status

| Component | Engine | Player | Notes |
|-----------|--------|--------|-------|
| GalleryAsset Entity | ✅ | ✅ | Full metadata |
| GenerationBatch Entity | ✅ | ✅ | Status tracking |
| WorkflowConfiguration | ✅ | ✅ | Slot-based |
| AssetRepository | ✅ | - | Neo4j impl |
| AssetService | ✅ | ✅ | CRUD + activation |
| GenerationService | ✅ | ✅ | Queue + progress |
| AssetGenerationQueue | ✅ | - | Background processing |
| ComfyUI Client | ✅ | - | Circuit breaker, retries |
| Asset Gallery UI | - | ✅ | Filter, context menu |
| Generation Queue UI | - | ✅ | Progress, actions |
| ComfyUI Banner | - | ✅ | Health indicator |
| Workflow Editor | - | ✅ | Full editor with mappings, locks, style detection |

---

## Key Files

### Engine

| Layer | File | Purpose |
|-------|------|---------|
| Domain | `crates/domain/src/entities/gallery_asset.rs` | Asset entity |
| Domain | `crates/domain/src/entities/generation_batch.rs` | Batch entity |
| Domain | `crates/domain/src/entities/workflow_config.rs` | Workflow entity |
| Repository | `crates/engine/src/repositories/assets.rs` | Asset operations |
| Use Case | `crates/engine/src/use_cases/assets/mod.rs` | Generation queue |
| Infrastructure | `crates/engine/src/infrastructure/comfyui.rs` | ComfyUI client |
| Infrastructure | `crates/engine/src/infrastructure/neo4j/asset_repo.rs` | Neo4j persistence |

### Player

| Layer | File | Purpose |
|-------|------|---------|
| Application | `src/application/services/asset_service.rs` | API calls |
| Application | `src/application/services/generation_service.rs` | Queue tracking |
| Presentation | `src/presentation/components/creator/asset_gallery.rs` | Gallery UI |
| Presentation | `src/presentation/components/creator/generation_queue.rs` | Queue UI |
| Presentation | `src/presentation/components/creator/comfyui_banner.rs` | Banner |
| Presentation | `src/presentation/components/dm_panel/director_generate_modal.rs` | Quick gen |
| Presentation | `src/presentation/components/settings/workflow_config_editor.rs` | Editor |
| Presentation | `src/presentation/state/generation_state.rs` | Queue state |

---

## Related Systems

- **Depends on**: [Character System](./character-system.md) (character assets), [Navigation System](./navigation-system.md) (location assets)
- **Used by**: [Scene System](./scene-system.md) (backdrop display)

---

## Revision History

| Date | Change |
|------|--------|
| 2026-01-05 | US-AST-010 complete - Advanced Workflow Parameter Editor |
| 2025-12-18 | Initial version extracted from MVP.md |
