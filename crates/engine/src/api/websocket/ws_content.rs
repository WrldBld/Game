//! WebSocket handlers for game content operations.
//!
//! Handles requests for races, classes, backgrounds, and other game content
//! through the unified CompendiumProvider API.

use super::*;

use crate::api::connections::ConnectionInfo;
use serde_json::json;
use wrldbldr_domain::{ContentFilter, ContentType};
use wrldbldr_protocol::{requests::content::ContentRequest, ErrorCode, ResponseResult};

/// Convert a string content type to the domain ContentType enum.
fn parse_content_type(content_type: &str) -> ContentType {
    match content_type.to_lowercase().as_str() {
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
    }
}

pub(super) async fn handle_content_request(
    state: &WsState,
    request_id: &str,
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
            let ct = parse_content_type(&content_type);

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
                        df = df.with_limit(limit);
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
                    format!("Failed to load content: {}", e),
                )),
            }
        }

        ContentRequest::GetContent {
            system_id,
            content_type,
            content_id,
        } => {
            let ct = parse_content_type(&content_type);

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
                    format!("Failed to load content: {}", e),
                )),
            }
        }

        ContentRequest::SearchContent {
            system_id,
            query,
            limit,
        } => match state.app.content.search_content(&system_id, &query, limit) {
            Ok(items) => Ok(ResponseResult::success(json!({
                "query": query,
                "items": items,
                "count": items.len()
            }))),
            Err(e) => Ok(ResponseResult::error(
                ErrorCode::NotFound,
                format!("Search failed: {}", e),
            )),
        },

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
