use serde::{Deserialize, Serialize};

use super::default_true;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TimeRequest {
    GetGameTime {
        world_id: String,
    },
    AdvanceGameTime {
        world_id: String,
        hours: u32,
    },
    AdvanceGameTimeMinutes {
        world_id: String,
        minutes: u32,
        reason: Option<String>,
    },
    SetGameTime {
        world_id: String,
        day: u32,
        hour: u8,
        #[serde(default = "default_true")]
        notify_players: bool,
    },
    SkipToPeriod {
        world_id: String,
        period: String,
    },
    GetTimeConfig {
        world_id: String,
    },
    UpdateTimeConfig {
        world_id: String,
        config: crate::types::GameTimeConfig,
    },
}
