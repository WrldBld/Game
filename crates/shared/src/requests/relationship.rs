use serde::{Deserialize, Serialize};

use super::CreateRelationshipData;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RelationshipRequest {
    GetSocialNetwork { world_id: String, #[serde(default)] limit: Option<u32>, #[serde(default)] offset: Option<u32> },
    CreateRelationship { data: CreateRelationshipData },
    DeleteRelationship { relationship_id: String },
}
