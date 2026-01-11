//! WebSocket handlers for character stat operations.

use super::*;

use crate::api::connections::ConnectionInfo;
use serde_json::json;
use wrldbldr_domain::{self as domain, RuleSystemConfig, RuleSystemVariant};
use wrldbldr_domain::entities::StatModifier;
use wrldbldr_protocol::{ErrorCode, ResponseResult, StatRequest};

pub(super) async fn handle_stat_request(
    state: &WsState,
    request_id: &str,
    conn_info: &ConnectionInfo,
    request: StatRequest,
) -> Result<ResponseResult, ServerMessage> {
    match request {
        StatRequest::GetCharacterStats { character_id } => {
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;
            match state.app.entities.character.get(character_id_typed).await {
                Ok(Some(character)) => {
                    let stats_data = character_stats_to_json(&character);
                    Ok(ResponseResult::success(stats_data))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        StatRequest::SetBaseStat {
            character_id,
            stat_name,
            value,
        } => {
            require_dm_for_request(conn_info, request_id)?;
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            match state.app.entities.character.get(character_id_typed).await {
                Ok(Some(mut character)) => {
                    character.stats.set_stat(&stat_name, value);
                    if let Err(e) = state.app.entities.character.save(&character).await {
                        return Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        ));
                    }
                    Ok(ResponseResult::success(character_stats_to_json(&character)))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        StatRequest::AddModifier { character_id, data } => {
            require_dm_for_request(conn_info, request_id)?;
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            match state.app.entities.character.get(character_id_typed).await {
                Ok(Some(mut character)) => {
                    let modifier = if data.active {
                        StatModifier::new(&data.source, data.value)
                    } else {
                        StatModifier::inactive(&data.source, data.value)
                    };
                    let modifier_id = modifier.id;
                    character.stats.add_modifier(&data.stat_name, modifier);

                    if let Err(e) = state.app.entities.character.save(&character).await {
                        return Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        ));
                    }

                    Ok(ResponseResult::success(json!({
                        "modifier_id": modifier_id.to_string(),
                        "stats": character_stats_to_json(&character),
                    })))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
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

            match state.app.entities.character.get(character_id_typed).await {
                Ok(Some(mut character)) => {
                    if !character.stats.remove_modifier(&stat_name, modifier_uuid) {
                        return Ok(ResponseResult::error(
                            ErrorCode::NotFound,
                            "Modifier not found",
                        ));
                    }

                    if let Err(e) = state.app.entities.character.save(&character).await {
                        return Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        ));
                    }

                    Ok(ResponseResult::success(character_stats_to_json(&character)))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
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

            match state.app.entities.character.get(character_id_typed).await {
                Ok(Some(mut character)) => {
                    if !character.stats.toggle_modifier(&stat_name, modifier_uuid) {
                        return Ok(ResponseResult::error(
                            ErrorCode::NotFound,
                            "Modifier not found",
                        ));
                    }

                    if let Err(e) = state.app.entities.character.save(&character).await {
                        return Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        ));
                    }

                    Ok(ResponseResult::success(character_stats_to_json(&character)))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        StatRequest::ClearStatModifiers {
            character_id,
            stat_name,
        } => {
            require_dm_for_request(conn_info, request_id)?;
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            match state.app.entities.character.get(character_id_typed).await {
                Ok(Some(mut character)) => {
                    character.stats.clear_modifiers(&stat_name);

                    if let Err(e) = state.app.entities.character.save(&character).await {
                        return Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        ));
                    }

                    Ok(ResponseResult::success(character_stats_to_json(&character)))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
        }

        StatRequest::ClearAllModifiers { character_id } => {
            require_dm_for_request(conn_info, request_id)?;
            let character_id_typed = parse_character_id_for_request(&character_id, request_id)?;

            match state.app.entities.character.get(character_id_typed).await {
                Ok(Some(mut character)) => {
                    character.stats.clear_all_modifiers();

                    if let Err(e) = state.app.entities.character.save(&character).await {
                        return Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        ));
                    }

                    Ok(ResponseResult::success(character_stats_to_json(&character)))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
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
            let config = RuleSystemConfig::from_variant(rule_variant);

            match state.app.entities.character.get(character_id_typed).await {
                Ok(Some(mut character)) => {
                    // Initialize stats from the template defaults
                    for stat_def in &config.stat_definitions {
                        character.stats.set_stat(&stat_def.name, stat_def.default_value);
                    }

                    if let Err(e) = state.app.entities.character.save(&character).await {
                        return Ok(ResponseResult::error(
                            ErrorCode::InternalError,
                            e.to_string(),
                        ));
                    }

                    Ok(ResponseResult::success(json!({
                        "template": config.name,
                        "stats": character_stats_to_json(&character),
                    })))
                }
                Ok(None) => Ok(ResponseResult::error(
                    ErrorCode::NotFound,
                    "Character not found",
                )),
                Err(e) => Ok(ResponseResult::error(
                    ErrorCode::InternalError,
                    e.to_string(),
                )),
            }
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
        .modifiers
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

    json!({
        "character_id": character.id.to_string(),
        "stats": stats_json,
        "modifiers": modifiers_json,
        "current_hp": character.stats.current_hp,
        "max_hp": character.stats.max_hp,
    })
}

/// Parse a rule system variant from string
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
        _ => RuleSystemVariant::GenericD20, // Default fallback
    }
}
