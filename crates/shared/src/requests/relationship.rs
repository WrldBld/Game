use serde::{Deserialize, Serialize};

use super::CreateRelationshipData;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RelationshipRequest {
    GetSocialNetwork { world_id: String },
    CreateRelationship { data: CreateRelationshipData },
    DeleteRelationship { relationship_id: String },
}
