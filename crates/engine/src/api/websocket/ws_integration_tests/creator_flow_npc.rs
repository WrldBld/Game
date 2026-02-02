use super::*;

use crate::api::websocket::ProtoWorldRole;
use wrldbldr_domain::types::{ChangeType, EntityType};
use wrldbldr_domain::{
    Character, CharacterId, DispositionLevel, MoodState, PlayerCharacterId, RegionId,
    RelationshipLevel, WorldId,
};
use wrldbldr_shared::{ClientMessage, ErrorCode, RequestPayload, ResponseResult, ServerMessage};

// NOTE: Helper functions moved to crate::test_fixtures::world_seeder for reuse across tests.
// Use crate::test_fixtures::world_seeder::{create_test_world, create_test_npc, create_test_pc} instead.
