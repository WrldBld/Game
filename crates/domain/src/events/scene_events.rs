//! Scene mutation outcomes.

use crate::entities::{SceneCondition, TimeContext};
use crate::value_objects::{AssetPath, SceneName};

/// Outcome of updating scene fields or state.
///
/// Note: Location and featured character changes are managed via graph edges,
/// not through aggregate mutations. Use `scene_repo.set_location()` and
/// `scene_repo.set_featured_characters()` instead.
#[derive(Debug, Clone)]
pub enum SceneUpdate {
    NameChanged {
        from: SceneName,
        to: SceneName,
    },
    TimeContextChanged {
        from: TimeContext,
        to: TimeContext,
    },
    BackdropOverrideChanged {
        from: Option<AssetPath>,
        to: Option<AssetPath>,
    },
    DirectorialNotesChanged {
        from: String,
        to: String,
    },
    OrderChanged {
        from: u32,
        to: u32,
    },
    EntryConditionAdded {
        condition: SceneCondition,
    },
    EntryConditionsCleared {
        previous_count: usize,
    },
}
