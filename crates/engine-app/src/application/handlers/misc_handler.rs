//! Miscellaneous domain request handlers
//!
//! Handles: Skill, Goal, Want, Relationship, Disposition, and Actantial context operations

use std::sync::Arc;

use wrldbldr_domain::entities::SkillCategory;
use wrldbldr_domain::value_objects::{Relationship, RelationshipType};
use wrldbldr_engine_ports::inbound::RequestContext;
use wrldbldr_protocol::{
    ActantialRoleData, ActorTypeData, CreateGoalData, CreateRelationshipData, CreateSkillData,
    CreateWantData, ErrorCode, ResponseResult, UpdateGoalData, UpdateSkillData, UpdateWantData,
    WantTargetTypeData,
};

use super::common::{
    convert_actantial_role, convert_actor_type, convert_want_target_type, convert_want_visibility,
    npc_disposition_to_dto, parse_character_id, parse_disposition_level, parse_goal_id,
    parse_player_character_id, parse_relationship_id, parse_relationship_level, parse_skill_id,
    parse_want_id, parse_world_id,
};
use crate::application::dto::SkillResponseDto;
use crate::application::services::{
    ActantialContextService, CreateSkillRequest, CreateWantRequest, DispositionService,
    RelationshipService, SkillService, UpdateSkillRequest, UpdateWantRequest,
};

// =============================================================================
// Skill Operations
// =============================================================================

/// Handle ListSkills request
pub async fn list_skills(skill_service: &Arc<dyn SkillService>, world_id: &str) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match skill_service.list_skills(id).await {
        Ok(skills) => {
            let dtos: Vec<SkillResponseDto> = skills.into_iter().map(|s| s.into()).collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetSkill request
pub async fn get_skill(skill_service: &Arc<dyn SkillService>, skill_id: &str) -> ResponseResult {
    let id = match parse_skill_id(skill_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match skill_service.get_skill(id).await {
        Ok(Some(skill)) => {
            let dto: SkillResponseDto = skill.into();
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Skill not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteSkill request (DM only)
pub async fn delete_skill(
    skill_service: &Arc<dyn SkillService>,
    ctx: &RequestContext,
    skill_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_skill_id(skill_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match skill_service.delete_skill(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateSkill request (DM only)
pub async fn create_skill(
    skill_service: &Arc<dyn SkillService>,
    ctx: &RequestContext,
    world_id: &str,
    data: CreateSkillData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Parse category from string or default to Physical
    let category = data
        .category
        .as_deref()
        .and_then(|c| c.parse().ok())
        .unwrap_or(SkillCategory::Physical);
    let request = CreateSkillRequest {
        name: data.name,
        description: data.description.unwrap_or_default(),
        category,
        base_attribute: data.attribute,
    };
    match skill_service.create_skill(id, request).await {
        Ok(skill) => {
            let dto: SkillResponseDto = skill.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdateSkill request (DM only)
pub async fn update_skill(
    skill_service: &Arc<dyn SkillService>,
    ctx: &RequestContext,
    skill_id: &str,
    data: UpdateSkillData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_skill_id(skill_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Parse category from string if provided
    let category = data.category.as_deref().and_then(|c| c.parse().ok());
    let request = UpdateSkillRequest {
        name: data.name,
        description: data.description,
        category,
        base_attribute: data.attribute,
        is_hidden: data.is_hidden,
        order: None,
    };
    match skill_service.update_skill(id, request).await {
        Ok(skill) => {
            let dto: SkillResponseDto = skill.into();
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

// =============================================================================
// Relationship Operations
// =============================================================================

/// Handle GetSocialNetwork request
pub async fn get_social_network(
    relationship_service: &Arc<dyn RelationshipService>,
    world_id: &str,
) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match relationship_service.get_social_network(id).await {
        Ok(network) => ResponseResult::success(network),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteRelationship request (DM only)
pub async fn delete_relationship(
    relationship_service: &Arc<dyn RelationshipService>,
    ctx: &RequestContext,
    relationship_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_relationship_id(relationship_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match relationship_service.delete_relationship(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateRelationship request (DM only)
pub async fn create_relationship(
    relationship_service: &Arc<dyn RelationshipService>,
    ctx: &RequestContext,
    data: CreateRelationshipData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let from_id = match parse_character_id(&data.from_character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let to_id = match parse_character_id(&data.to_character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let relationship_type: RelationshipType = data
        .relationship_type
        .parse()
        .unwrap_or_else(|_| RelationshipType::Custom(data.relationship_type.clone()));
    let relationship = Relationship::new(from_id, to_id, relationship_type);
    match relationship_service
        .create_relationship(&relationship)
        .await
    {
        Ok(()) => ResponseResult::success(serde_json::json!({
            "id": relationship.id.to_string(),
            "from_character_id": data.from_character_id,
            "to_character_id": data.to_character_id,
            "relationship_type": data.relationship_type,
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

// =============================================================================
// Actantial Context Operations
// =============================================================================

/// Handle GetActantialContext request
pub async fn get_actantial_context(
    actantial_service: &Arc<dyn ActantialContextService>,
    character_id: &str,
) -> ResponseResult {
    let id = match parse_character_id(character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match actantial_service.get_context(id).await {
        Ok(context) => ResponseResult::success(context),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle AddActantialView request (DM only)
pub async fn add_actantial_view(
    actantial_service: &Arc<dyn ActantialContextService>,
    ctx: &RequestContext,
    character_id: String,
    want_id: String,
    target_id: String,
    target_type: ActorTypeData,
    role: ActantialRoleData,
    reason: String,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let cid = match parse_character_id(&character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let wid = match parse_want_id(&want_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let target_type_converted = convert_actor_type(target_type);
    let role_converted = convert_actantial_role(role);
    match actantial_service
        .add_actantial_view(
            cid,
            wid,
            &target_id,
            target_type_converted,
            role_converted,
            reason,
        )
        .await
    {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle RemoveActantialView request (DM only)
pub async fn remove_actantial_view(
    actantial_service: &Arc<dyn ActantialContextService>,
    ctx: &RequestContext,
    character_id: String,
    want_id: String,
    target_id: String,
    target_type: ActorTypeData,
    role: ActantialRoleData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let cid = match parse_character_id(&character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let wid = match parse_want_id(&want_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let target_type_converted = convert_actor_type(target_type);
    let role_converted = convert_actantial_role(role);
    match actantial_service
        .remove_actantial_view(cid, wid, &target_id, target_type_converted, role_converted)
        .await
    {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

// =============================================================================
// NPC Disposition Operations
// =============================================================================

/// Handle GetNpcDispositions request
pub async fn get_npc_dispositions(
    disposition_service: &Arc<dyn DispositionService>,
    pc_id: &str,
) -> ResponseResult {
    let id = match parse_player_character_id(pc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match disposition_service.get_all_relationships(id).await {
        Ok(dispositions) => {
            let dtos: Vec<wrldbldr_protocol::NpcDispositionStateDto> =
                dispositions.iter().map(npc_disposition_to_dto).collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SetNpcDisposition request (DM only)
pub async fn set_npc_disposition(
    disposition_service: &Arc<dyn DispositionService>,
    ctx: &RequestContext,
    npc_id: String,
    pc_id: String,
    disposition: String,
    reason: Option<String>,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let nid = match parse_character_id(&npc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let pid = match parse_player_character_id(&pc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let disposition_level = parse_disposition_level(&disposition);
    match disposition_service
        .set_disposition(nid, pid, disposition_level, reason)
        .await
    {
        Ok(state) => {
            let dto = npc_disposition_to_dto(&state);
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SetNpcRelationship request (DM only)
pub async fn set_npc_relationship(
    disposition_service: &Arc<dyn DispositionService>,
    ctx: &RequestContext,
    npc_id: String,
    pc_id: String,
    relationship: String,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let nid = match parse_character_id(&npc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let pid = match parse_player_character_id(&pc_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let rel_level = parse_relationship_level(&relationship);
    match disposition_service
        .set_relationship(nid, pid, rel_level)
        .await
    {
        Ok(state) => {
            let dto = npc_disposition_to_dto(&state);
            ResponseResult::success(dto)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

// =============================================================================
// Goal Operations
// =============================================================================

/// Handle ListGoals request
pub async fn list_goals(
    actantial_service: &Arc<dyn ActantialContextService>,
    world_id: &str,
) -> ResponseResult {
    let id = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match actantial_service.get_world_goals(id).await {
        Ok(goals) => {
            let dtos: Vec<serde_json::Value> = goals
                .iter()
                .map(|g| {
                    serde_json::json!({
                        "id": g.id.to_string(),
                        "name": g.name,
                        "description": g.description,
                    })
                })
                .collect();
            ResponseResult::success(dtos)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetGoal request
pub async fn get_goal(
    actantial_service: &Arc<dyn ActantialContextService>,
    goal_id: &str,
) -> ResponseResult {
    let id = match parse_goal_id(goal_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match actantial_service.get_goal(id).await {
        Ok(Some(goal)) => {
            let dto = serde_json::json!({
                "id": goal.id.to_string(),
                "name": goal.name,
                "description": goal.description,
            });
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Goal not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateGoal request (DM only)
pub async fn create_goal(
    actantial_service: &Arc<dyn ActantialContextService>,
    ctx: &RequestContext,
    world_id: &str,
    data: CreateGoalData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let wid = match parse_world_id(world_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match actantial_service
        .create_goal(wid, data.name, data.description)
        .await
    {
        Ok(goal_id) => ResponseResult::success(serde_json::json!({
            "id": goal_id.to_string(),
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdateGoal request (DM only)
pub async fn update_goal(
    actantial_service: &Arc<dyn ActantialContextService>,
    ctx: &RequestContext,
    goal_id: &str,
    data: UpdateGoalData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_goal_id(goal_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match actantial_service
        .update_goal(id, data.name, data.description)
        .await
    {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteGoal request (DM only)
pub async fn delete_goal(
    actantial_service: &Arc<dyn ActantialContextService>,
    ctx: &RequestContext,
    goal_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_goal_id(goal_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match actantial_service.delete_goal(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

// =============================================================================
// Want Operations
// =============================================================================

/// Handle ListWants request
pub async fn list_wants(
    actantial_service: &Arc<dyn ActantialContextService>,
    character_id: &str,
) -> ResponseResult {
    let id = match parse_character_id(character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    // Get full context which includes wants
    match actantial_service.get_context(id).await {
        Ok(context) => {
            // Extract wants from context
            ResponseResult::success(context.wants)
        }
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle GetWant request
pub async fn get_want(
    actantial_service: &Arc<dyn ActantialContextService>,
    want_id: &str,
) -> ResponseResult {
    let id = match parse_want_id(want_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match actantial_service.get_want(id).await {
        Ok(Some(want)) => {
            let dto = serde_json::json!({
                "id": want.id.to_string(),
                "description": want.description,
                "intensity": want.intensity,
                "visibility": format!("{:?}", want.visibility),
                "deflection_behavior": want.deflection_behavior,
                "tells": want.tells,
            });
            ResponseResult::success(dto)
        }
        Ok(None) => ResponseResult::error(ErrorCode::NotFound, "Want not found"),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle CreateWant request (DM only)
pub async fn create_want(
    actantial_service: &Arc<dyn ActantialContextService>,
    ctx: &RequestContext,
    character_id: &str,
    data: CreateWantData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let cid = match parse_character_id(character_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let visibility = convert_want_visibility(data.visibility);
    let request = CreateWantRequest {
        description: data.description,
        intensity: data.intensity,
        priority: data.priority,
        visibility,
        target_id: data.target_id,
        target_type: data
            .target_type
            .map(|t| convert_want_target_type(t).to_string()),
        deflection_behavior: data.deflection_behavior,
        tells: data.tells,
    };
    match actantial_service.create_want(cid, request).await {
        Ok(want_id) => ResponseResult::success(serde_json::json!({
            "id": want_id.to_string(),
        })),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle UpdateWant request (DM only)
pub async fn update_want(
    actantial_service: &Arc<dyn ActantialContextService>,
    ctx: &RequestContext,
    want_id: &str,
    data: UpdateWantData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_want_id(want_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let request = UpdateWantRequest {
        description: data.description,
        intensity: data.intensity,
        priority: data.priority,
        visibility: data.visibility.map(convert_want_visibility),
        deflection_behavior: data.deflection_behavior,
        tells: data.tells,
    };
    match actantial_service.update_want(id, request).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle DeleteWant request (DM only)
pub async fn delete_want(
    actantial_service: &Arc<dyn ActantialContextService>,
    ctx: &RequestContext,
    want_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_want_id(want_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match actantial_service.delete_want(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle SetWantTarget request (DM only)
pub async fn set_want_target(
    actantial_service: &Arc<dyn ActantialContextService>,
    ctx: &RequestContext,
    want_id: String,
    target_id: String,
    target_type: WantTargetTypeData,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_want_id(&want_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    let target_type_str = convert_want_target_type(target_type);
    match actantial_service
        .set_want_target(id, &target_id, target_type_str)
        .await
    {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}

/// Handle RemoveWantTarget request (DM only)
pub async fn remove_want_target(
    actantial_service: &Arc<dyn ActantialContextService>,
    ctx: &RequestContext,
    want_id: &str,
) -> ResponseResult {
    if let Err(e) = ctx.require_dm() {
        return e;
    }
    let id = match parse_want_id(want_id) {
        Ok(id) => id,
        Err(e) => return e,
    };
    match actantial_service.remove_want_target(id).await {
        Ok(()) => ResponseResult::success_empty(),
        Err(e) => ResponseResult::error(ErrorCode::InternalError, e.to_string()),
    }
}
