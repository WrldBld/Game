use serde::{Deserialize, Serialize};

use super::{CreateChallengeData, UpdateChallengeData};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChallengeRequest {
    ListChallenges {
        world_id: String,
    },
    GetChallenge {
        challenge_id: String,
    },
    CreateChallenge {
        world_id: String,
        data: CreateChallengeData,
    },
    UpdateChallenge {
        challenge_id: String,
        data: UpdateChallengeData,
    },
    DeleteChallenge {
        challenge_id: String,
    },
    SetChallengeActive {
        challenge_id: String,
        active: bool,
    },
    SetChallengeFavorite {
        challenge_id: String,
        favorite: bool,
    },
}
