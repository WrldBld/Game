//! WebSocket handlers for character stat operations.

use super::*;

use crate::api::connections::ConnectionInfo;
use serde_json::json;
use wrldbldr_domain::{self as domain, CharacterId, RuleSystemConfig, RuleSystemVariant};
use wrldbldr_domain::entities::StatModifier;
use wrldbldr_protocol::{ErrorCode, ResponseResult, StatRequest};

/// Helper to fetch a character or return an appropriate error response.
async fn get_character_or_error(
    state: &WsState,
    character_id: CharacterId,
) -> Result<domain::Character, ResponseResult> {
    match state.app.entities.character.get(character_id).await {
        Ok(Some(character)) => Ok(character),
        Ok(None) => Err(ResponseResult::error(
            ErrorCode::NotFound,
            "Character not found",
        )),
        Err(e) => Err(ResponseResult::error(
            ErrorCode::InternalError,
            e.to_string(),
        )),
    }
}

/// Helper to save a character or return an appropriate error response.
async fn save_character_or_error(
    state: &WsState,
    character: &domain::Character,
) -> Result<(), ResponseResult> {
    state
        .app
        .entities
        .character
        .save(character)
        .await
        .map_err(|e| ResponseResult::error(ErrorCode::InternalError, e.to_string()))
}

pub(super) async fn handle_stat_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: StatRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        StatRequest::GetCharacterStats { character_id } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;
            let character = match get_character_or_error(state, character_id_typed).await {
                Ok(c) => c,
                Err(resp) => return Ok(resp),
            };

            tracing::debug!(
                character_id = %character_id,
                "Retrieved character stats"
            );

            Ok(ResponseResult::success(character_stats_to_json(&character)))
        }

        StatRequest::SetBaseStat {
            character_id,
            stat_name,
            value,
        } => {
            require_dm_for_request(conn_info, request_id)?;
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            let mut character = match get_character_or_error(state, character_id_typed).await {
                Ok(c) => c,
                Err(resp) => return Ok(resp),
            };

            // Validate value against rule system bounds if world has stat definitions
            let final_value = match state.app.entities.world.get(character.world_id).await {
                Ok(Some(world)) => {
                    // Find stat definition by name or abbreviation
                    if let Some(stat_def) = world.rule_system.stat_definitions.iter().find(|s| {
                        s.name.eq_ignore_ascii_case(&stat_name)
                            || s.abbreviation.eq_ignore_ascii_case(&stat_name)
                    }) {
                        let clamped = value.max(stat_def.min_value).min(stat_def.max_value);
                        if value != clamped {
                            tracing::warn!(
                                stat = %stat_name,
                                requested = %value,
                                clamped = %clamped,
                                min = %stat_def.min_value,
                                max = %stat_def.max_value,
                                character_id = %character_id,
                                "Stat value clamped to template bounds"
                            );
                        }
                        clamped
                    } else {
                        // Unknown stat name - allow any value
                        value
                    }
                }
                _ => {
                    // World not found or error - allow any value
                    value
                }
            };

            character.stats.set_stat(&stat_name, final_value);

            if let Err(resp) = save_character_or_error(state, &character).await {
                return Ok(resp);
            }

            tracing::info!(
                character_id = %character_id,
                stat_name = %stat_name,
                value = %final_value,
                original_value = %value,
                "Set base stat value"
            );

            Ok(ResponseResult::success(character_stats_to_json(&character)))
        }

        StatRequest::AddModifier { character_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            let mut character = match get_character_or_error(state, character_id_typed).await {
                Ok(c) => c,
                Err(resp) => return Ok(resp),
            };

            let modifier = if data.active {
                StatModifier::new(&data.source, data.value)
            } else {
                StatModifier::inactive(&data.source, data.value)
            };
            let modifier_id = modifier.id;
            character.stats.add_modifier(&data.stat_name, modifier);

            if let Err(resp) = save_character_or_error(state, &character).await {
                return Ok(resp);
            }

            tracing::info!(
                character_id = %character_id,
                stat_name = %data.stat_name,
                modifier_id = %modifier_id,
                source = %data.source,
                value = %data.value,
                active = %data.active,
                "Added stat modifier"
            );

            Ok(ResponseResult::success(json!({
                "modifier_id": modifier_id.to_string(),
                "stats": character_stats_to_json(&character),
            })))
        }

        StatRequest::RemoveModifier {
            character_id,
            stat_name,
            modifier_id,
        } => {
            require_dm_for_request(conn_info, request_id)?;
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;
            let modifier_uuid = match Uuid::parse_str(&modifier_id) {
                Ok(id) => id,
                Err(_) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        "Invalid modifier ID format",
                    ));
                }
            };

            let mut character = match get_character_or_error(state, character_id_typed).await {
                Ok(c) => c,
                Err(resp) => return Ok(resp),
            };

            if !character.stats.remove_modifier(&stat_name, modifier_uuid) {
                return Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Modifier not found",
                ));
            }

            if let Err(resp) = save_character_or_error(state, &character).await {
                return Ok(resp);
            }

            tracing::info!(
                character_id = %character_id,
                stat_name = %stat_name,
                modifier_id = %modifier_id,
                "Removed stat modifier"
            );

            Ok(ResponseResult::success(character_stats_to_json(&character)))
        }

        StatRequest::ToggleModifier {
            character_id,
            stat_name,
            modifier_id,
        } => {
            require_dm_for_request(conn_info, request_id)?;
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;
            let modifier_uuid = match Uuid::parse_str(&modifier_id) {
                Ok(id) => id,
                Err(_) => {
                    return Ok(ResponseResult::error(
                        ErrorCode::BadRequest,
                        "Invalid modifier ID format",
                    ));
                }
            };

            let mut character = match get_character_or_error(state, character_id_typed).await {
                Ok(c) => c,
                Err(resp) => return Ok(resp),
            };

            if !character.stats.toggle_modifier(&stat_name, modifier_uuid) {
                return Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Modifier not found",
                ));
            }

            if let Err(resp) = save_character_or_error(state, &character).await {
                return Ok(resp);
            }

            tracing::info!(
                character_id = %character_id,
                stat_name = %stat_name,
                modifier_id = %modifier_id,
                "Toggled stat modifier"
            );

            Ok(ResponseResult::success(character_stats_to_json(&character)))
        }

        StatRequest::ClearStatModifiers {
            character_id,
            stat_name,
        } => {
            require_dm_for_request(conn_info, request_id)?;
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            let mut character = match get_character_or_error(state, character_id_typed).await {
                Ok(c) => c,
                Err(resp) => return Ok(resp),
            };

            character.stats.clear_modifiers(&stat_name);

            if let Err(resp) = save_character_or_error(state, &character).await {
                return Ok(resp);
            }

            tracing::info!(
                character_id = %character_id,
                stat_name = %stat_name,
                "Cleared all modifiers for stat"
            );

            Ok(ResponseResult::success(character_stats_to_json(&character)))
        }

        StatRequest::ClearAllModifiers { character_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            let mut character = match get_character_or_error(state, character_id_typed).await {
                Ok(c) => c,
                Err(resp) => return Ok(resp),
            };

            character.stats.clear_all_modifiers();

            if let Err(resp) = save_character_or_error(state, &character).await {
                return Ok(resp);
            }

            tracing::info!(
                character_id = %character_id,
                "Cleared all stat modifiers"
            );

            Ok(ResponseResult::success(character_stats_to_json(&character)))
        }

        StatRequest::GetStatTemplates { variant } => {
            let templates = if let Some(variant_str) = variant {
                // Parse the variant and get specific template
                let rule_variant = parse_rule_system_variant(&variant_str);
                vec![RuleSystemConfig::from_variant(rule_variant)]
            } else {
                // Return all available templates
                vec![
                    RuleSystemConfig::dnd_5e(),
                    RuleSystemConfig::pathfinder_2e(),
                    RuleSystemConfig::call_of_cthulhu_7e(),
                    RuleSystemConfig::fate_core(),
                    RuleSystemConfig::powered_by_apocalypse(),
                    RuleSystemConfig::blades_in_the_dark(),
                    RuleSystemConfig::kids_on_bikes(),
                    RuleSystemConfig::runequest(),
                    RuleSystemConfig::generic_d20(),
                    RuleSystemConfig::generic_d100(),
                ]
            };

            let templates_json: Vec<serde_json::Value> = templates
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "variant": format!("{:?}", t.variant).to_lowercase(),
                        "system_type": format!("{:?}", t.system_type).to_lowercase(),
                        "stat_definitions": t.stat_definitions.iter().map(|s| json!({
                            "name": s.name,
                            "abbreviation": s.abbreviation,
                            "min_value": s.min_value,
                            "max_value": s.max_value,
                            "default_value": s.default_value,
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect();

            Ok(ResponseResult::success(json!(templates_json)))
        }

        StatRequest::InitializeFromTemplate {
            character_id,
            variant,
        } => {
            require_dm_for_request(conn_info, request_id)?;
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;
            let rule_variant = parse_rule_system_variant(&variant);
            let config = RuleSystemConfig::from_variant(rule_variant.clone());

            let mut character = match get_character_or_error(state, character_id_typed).await {
                Ok(c) => c,
                Err(resp) => return Ok(resp),
            };

            // Initialize stats from the template defaults
            for stat_def in &config.stat_definitions {
                character.stats.set_stat(&stat_def.name, stat_def.default_value);
            }

            if let Err(resp) = save_character_or_error(state, &character).await {
                return Ok(resp);
            }

            tracing::info!(
                character_id = %character_id,
                template = %config.name,
                variant = ?rule_variant,
                stat_count = %config.stat_definitions.len(),
                "Initialized character stats from template"
            );

            Ok(ResponseResult::success(json!({
                "template": config.name,
                "stats": character_stats_to_json(&character),
            })))
        }
    }
}

/// Convert character stats to JSON format for API response
fn character_stats_to_json(character: &domain::Character) -> serde_json::Value {
    let all_stats = character.stats.get_all_stats();
    let stats_json: serde_json::Map<String, serde_json::Value> = all_stats
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                json!({
                    "base": value.base,
                    "modifier_total": value.modifier_total,
                    "effective": value.effective,
                }),
            )
        })
        .collect();

    let modifiers_json: serde_json::Map<String, serde_json::Value> = character
        .stats
        .modifiers()
        .iter()
        .map(|(stat_name, mods)| {
            (
                stat_name.clone(),
                json!(mods
                    .iter()
                    .map(|m| json!({
                        "id": m.id.to_string(),
                        "source": m.source,
                        "value": m.value,
                        "active": m.active,
                    }))
                    .collect::<Vec<_>>()),
            )
        })
        .collect();

    // Build HP info with base, modifiers, and effective values
    let hp_info = json!({
        "current_hp": {
            "base": character.stats.get_base_current_hp(),
            "modifier_total": character.stats.get_modifier_total("current_hp"),
            "effective": character.stats.get_current_hp(),
        },
        "max_hp": {
            "base": character.stats.get_base_max_hp(),
            "modifier_total": character.stats.get_modifier_total("max_hp"),
            "effective": character.stats.get_max_hp(),
        },
    });

    json!({
        "character_id": character.id.to_string(),
        "stats": stats_json,
        "modifiers": modifiers_json,
        "hp": hp_info,
    })
}

/// Parse a rule system variant from string.
///
/// Unknown variants will log a warning and fall back to GenericD20.
fn parse_rule_system_variant(variant_str: &str) -> RuleSystemVariant {
    match variant_str.to_lowercase().as_str() {
        "dnd5e" | "dnd_5e" | "d&d5e" => RuleSystemVariant::Dnd5e,
        "pathfinder2e" | "pathfinder_2e" | "pf2e" => RuleSystemVariant::Pathfinder2e,
        "callofcthulhu7e" | "call_of_cthulhu_7e" | "coc7e" | "coc" => {
            RuleSystemVariant::CallOfCthulhu7e
        }
        "runequest" | "rq" => RuleSystemVariant::RuneQuest,
        "kidsonbikes" | "kids_on_bikes" | "kob" => RuleSystemVariant::KidsOnBikes,
        "fatecore" | "fate_core" | "fate" => RuleSystemVariant::FateCore,
        "poweredbyapocalypse" | "powered_by_apocalypse" | "pbta" => {
            RuleSystemVariant::PoweredByApocalypse
        }
        "bladesinthedark" | "blades_in_the_dark" | "blades" | "bitd" => {
            RuleSystemVariant::BladesInTheDark
        }
        "genericd20" | "generic_d20" | "d20" => RuleSystemVariant::GenericD20,
        "genericd100" | "generic_d100" | "d100" => RuleSystemVariant::GenericD100,
        unknown => {
            tracing::warn!(
                variant = %unknown,
                fallback = "GenericD20",
                "Unknown rule system variant requested, falling back to GenericD20"
            );
            RuleSystemVariant::GenericD20
        }
    }
}
