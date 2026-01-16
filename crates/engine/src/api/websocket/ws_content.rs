//! WebSocket handlers for game content operations.
//!
//! Handles requests for races, classes, backgrounds, and other game content
//! through the unified CompendiumProvider API.

use super::*;

use crate::api::connections::ConnectionInfo;
use crate::api::websocket::error_sanitizer::sanitize_repo_error;
use serde_json::json;
use wrldbldr_shared::game_systems::{ContentFilter, ContentType};
use wrldbldr_shared::requests::content::ContentRequest;
use wrldbldr_shared::{ErrorCode, ResponseResult};

// Input validation limits to prevent DoS attacks
const MAX_QUERY_LENGTH: usize = 1024;
const MAX_LIMIT: usize = 1000;
const MAX_TAGS: usize = 100;
const MAX_TAG_LENGTH: usize = 50;
const MAX_CONTENT_TYPE_LENGTH: usize = 50;

/// Convert a string content type to the domain ContentType enum.
fn parse_content_type(content_type: &str) -> Option<ContentType> {
    // Reject overly long custom content types
    if content_type.len() > MAX_CONTENT_TYPE_LENGTH {
        return None;
    }

    Some(match content_type.to_lowercase().as_str() {
        "origin" | "race" | "ancestry" => ContentType::CharacterOrigin,
        "class" | "playbook" => ContentType::CharacterClass,
        "background" => ContentType::CharacterBackground,
        "suborigin" | "subrace" => ContentType::CharacterSuborigin,
        "subclass" => ContentType::CharacterSubclass,
        "spell" => ContentType::Spell,
        "feat" => ContentType::Feat,
        "ability" => ContentType::Ability,
        "class_feature" => ContentType::ClassFeature,
        "weapon" => ContentType::Weapon,
        "armor" => ContentType::Armor,
        "item" => ContentType::Item,
        "magic_item" => ContentType::MagicItem,
        other => ContentType::Custom(other.to_string()),
    })
}

/// Validate filter inputs to prevent DoS attacks.
fn validate_filter(
    filter: &wrldbldr_shared::requests::content::ContentFilterRequest,
) -> Result<(), String> {
    if let Some(ref search) = filter.search {
        if search.len() > MAX_QUERY_LENGTH {
            return Err(format!(
                "Search query too long (max {} characters)",
                MAX_QUERY_LENGTH
            ));
        }
    }

    if let Some(limit) = filter.limit {
        if limit > MAX_LIMIT {
            return Err(format!("Limit too high (max {})", MAX_LIMIT));
        }
    }

    if let Some(ref tags) = filter.tags {
        if tags.len() > MAX_TAGS {
            return Err(format!("Too many tags (max {})", MAX_TAGS));
        }
        for tag in tags {
            if tag.len() > MAX_TAG_LENGTH {
                return Err(format!("Tag too long (max {} characters)", MAX_TAG_LENGTH));
            }
        }
    }

    Ok(())
}

pub(super) async fn handle_content_request(
    state: &WsState,
    _request_id: &str,
    _conn_info: &ConnectionInfo,
    request: ContentRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        ContentRequest::ListProviders => {
            let systems = state.app.content.registered_systems();
            Ok(ResponseResult::success(json!({
                "providers": systems,
                "count": systems.len()
            })))
        }

        ContentRequest::ListContentTypes { system_id } => {
            let content_types = state.app.content.content_types_for_system(&system_id);
            let type_names: Vec<String> = content_types.iter().map(|ct| ct.slug()).collect();
            Ok(ResponseResult::success(json!({
                "system_id": system_id,
                "content_types": type_names,
                "count": type_names.len()
            })))
        }

        ContentRequest::ListContent {
            system_id,
            content_type,
            filter,
        } => {
            // Validate content type
            let ct = match parse_content_type(&content_type) {
                Some(ct) => ct,
                None => {
                    return Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        "Content type too long or invalid",
                    ))
                }
            };

            // Validate filter if present
            if let Some(ref f) = filter {
                if let Err(e) = validate_filter(f) {
                    return Ok(ResponseResult::error(ErrorCode::BadRequest, e));
                }
            }

            // Convert filter
            let domain_filter = filter
                .map(|f| {
                    let mut df = ContentFilter::new();
                    if let Some(source) = f.source {
                        df = df.with_source(source);
                    }
                    if let Some(search) = f.search {
                        df = df.with_search(search);
                    }
                    if let Some(tags) = f.tags {
                        df = df.with_tags(tags);
                    }
                    if let Some(limit) = f.limit {
                        // Clamp limit to MAX_LIMIT
                        df = df.with_limit(limit.min(MAX_LIMIT));
                    }
                    if let Some(offset) = f.offset {
                        df = df.with_offset(offset);
                    }
                    df
                })
                .unwrap_or_default();

            match state
                .app
                .content
                .get_content(&system_id, &ct, &domain_filter)
            {
                Ok(items) => Ok(ResponseResult::success(json!({
                    "content_type": content_type,
                    "items": items,
                    "count": items.len()
                }))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    sanitize_repo_error(&e, "load content"),
                )),
            }
        }

        ContentRequest::GetContent {
            system_id,
            content_type,
            content_id,
        } => {
            // Validate content type
            let ct = match parse_content_type(&content_type) {
                Some(ct) => ct,
                None => {
                    return Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        "Content type too long or invalid",
                    ))
                }
            };

            match state
                .app
                .content
                .get_content_by_id(&system_id, &ct, &content_id)
            {
                Ok(Some(item)) => Ok(ResponseResult::success(json!({
                    "item": item
                }))),
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    format!("Content not found: {}", content_id),
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    sanitize_repo_error(&e, "load content"),
                )),
            }
        }

        ContentRequest::SearchContent {
            system_id,
            query,
            limit,
        } => {
            // Validate query length
            if query.len() > MAX_QUERY_LENGTH {
                return Ok(ResponseResult::error(
                    ErrorCode::BadRequest,
                    format!("Query too long (max {} characters)", MAX_QUERY_LENGTH),
                ));
            }

            // Clamp limit
            let safe_limit = limit.min(MAX_LIMIT);

            match state
                .app
                .content
                .search_content(&system_id, &query, safe_limit)
            {
                Ok(items) => Ok(ResponseResult::success(json!({
                    "query": query,
                    "items": items,
                    "count": items.len()
                }))),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    sanitize_repo_error(&e, "search content"),
                )),
            }
        }

        ContentRequest::GetContentStats { system_id } => {
            let stats = state.app.content.stats();

            Ok(ResponseResult::success(json!({
                "system_id": system_id,
                "total_systems": stats.systems,
                "total_items": stats.total_items
            })))
        }
    }
}
