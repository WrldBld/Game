use serde::{Deserialize, Serialize};

use super::{CreateSceneData, UpdateSceneData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneRequest {
    ListScenes {
        act_id: String,
    },
    GetScene {
        scene_id: String,
    },
    CreateScene {
        act_id: String,
        data: CreateSceneData,
    },
    UpdateScene {
        scene_id: String,
        data: UpdateSceneData,
    },
    DeleteScene {
        scene_id: String,
    },
}
