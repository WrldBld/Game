//! Helper functions for generating staging suggestions.

use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::infrastructure::ports::{
    ChatMessage, LlmPort, LlmRequest, NpcRegionRelationType, NpcWithRegionInfo,
};
use crate::repositories::staging::Staging;
use wrldbldr_domain::{CharacterId, RegionId};

use super::types::StagedNpc;

#[derive(Deserialize)]
struct LlmSuggestion {
    name: String,
    reason: String,
}

pub async fn generate_rule_based_suggestions(
    npcs_with_relationships: &[NpcWithRegionInfo],
    staging: &Staging,
    region_id: RegionId,
) -> Vec<StagedNpc> {
    let mut suggestions: Vec<StagedNpc> = npcs_with_relationships
        .iter()
        .filter(|n| n.relationship_type != NpcRegionRelationType::Avoids)
        .map(|npc| {
            let reasoning = match npc.relationship_type {
                NpcRegionRelationType::HomeRegion => "Lives here".to_string(),
                NpcRegionRelationType::WorksAt => match npc.shift.as_deref() {
                    Some("day") => "Works here (day shift)".to_string(),
                    Some("night") => "Works here (night shift)".to_string(),
                    _ => "Works here".to_string(),
                },
                NpcRegionRelationType::Frequents => {
                    let freq = npc.frequency.as_deref().unwrap_or("sometimes");
                    let time = npc.time_of_day.as_deref();
                    match time {
                        Some(t) => format!("Frequents this area {} ({})", freq, t),
                        None => format!("Frequents this area ({})", freq),
                    }
                }
                NpcRegionRelationType::Avoids => "Avoids this area".to_string(),
            };

            StagedNpc {
                character_id: npc.character_id,
                name: npc.name.clone(),
                sprite_asset: npc.sprite_asset.clone(),
                portrait_asset: npc.portrait_asset.clone(),
                is_present: true,
                reasoning,
                is_hidden_from_players: false,
                mood: Some(npc.default_mood.to_string()),
            }
        })
        .collect();

    // Issue 4.3 fix: Use HashSet for O(1) lookup instead of O(n) iter().any()
    let existing_ids: HashSet<CharacterId> = suggestions.iter().map(|s| s.character_id).collect();

    if let Ok(staged_npcs) = staging.get_staged_npcs(region_id).await {
        for staged in staged_npcs {
            if !existing_ids.contains(&staged.character_id) {
                suggestions.push(StagedNpc {
                    character_id: staged.character_id,
                    name: staged.name,
                    sprite_asset: staged.sprite_asset,
                    portrait_asset: staged.portrait_asset,
                    is_present: staged.is_present,
                    reasoning: staged.reasoning,
                    is_hidden_from_players: staged.is_hidden_from_players,
                    mood: Some(staged.mood.to_string()),
                });
            }
        }
    }

    suggestions
}

pub async fn generate_llm_based_suggestions(
    npcs_with_relationships: &[NpcWithRegionInfo],
    llm: &dyn LlmPort,
    region_name: &str,
    location_name: &str,
    guidance: Option<&str>,
) -> Vec<StagedNpc> {
    let candidates: Vec<_> = npcs_with_relationships
        .iter()
        .filter(|n| n.relationship_type != NpcRegionRelationType::Avoids)
        .collect();

    if candidates.is_empty() {
        return vec![];
    }

    let npc_list: String = candidates
        .iter()
        .enumerate()
        .map(|(i, npc)| {
            let relationship = match npc.relationship_type {
                NpcRegionRelationType::HomeRegion => "lives here",
                NpcRegionRelationType::WorksAt => "works here",
                NpcRegionRelationType::Frequents => "frequents this area",
                NpcRegionRelationType::Avoids => "avoids this area",
            };
            format!("{}. {} ({})", i + 1, npc.name, relationship)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let guidance_text = guidance
        .filter(|g| !g.is_empty())
        .map(|g| format!("\n\nDM's guidance: {}", g))
        .unwrap_or_default();

    let system_prompt = "You are a helpful TTRPG assistant helping decide which NPCs should be present in a scene. \
        Respond with a JSON array of objects, each with 'name' (exact name from the list) and 'reason' (brief explanation). \
        Select 1-4 NPCs that would logically be present. Only include NPCs from the provided list.";

    let user_prompt = format!(
        "Region: {} (in {})\n\nAvailable NPCs:\n{}{}\n\nWhich NPCs should be present? Respond with JSON only.",
        region_name, location_name, npc_list, guidance_text
    );

    let request = LlmRequest::new(vec![ChatMessage::user(&user_prompt)])
        .with_system_prompt(system_prompt)
        .with_temperature(0.7);

    let response = match llm.generate(request).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::warn!(error = %e, "LLM staging suggestion failed");
            return vec![];
        }
    };

    let suggestions = parse_llm_staging_response(&response.content, &candidates);

    tracing::info!(
        region = %region_name,
        suggestion_count = suggestions.len(),
        "Generated LLM staging suggestions"
    );

    suggestions
}

/// Normalizes a name for matching by trimming whitespace, converting to lowercase,
/// and collapsing multiple consecutive whitespace characters into single spaces.
fn normalize_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn parse_llm_staging_response(content: &str, candidates: &[&NpcWithRegionInfo]) -> Vec<StagedNpc> {
    let json_start = content.find('[');
    let json_end = content.rfind(']');

    let json_str = match (json_start, json_end) {
        (Some(start), Some(end)) if end > start => &content[start..=end],
        _ => {
            tracing::warn!(
                content = %content,
                "LLM staging response did not contain a valid JSON array - returning empty suggestions"
            );
            return vec![];
        }
    };

    let parsed: Vec<LlmSuggestion> = match serde_json::from_str(json_str) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                error = %e,
                json = %json_str,
                "Failed to parse LLM staging JSON response - returning empty suggestions"
            );
            return vec![];
        }
    };

    // Issue 4.4 fix: Pre-build HashMap of normalized name -> NPC for O(1) lookup
    // instead of calling normalize_name O(n*m) times in the filter_map loop
    let normalized_name_map: HashMap<String, &NpcWithRegionInfo> = candidates
        .iter()
        .map(|&npc| (normalize_name(&npc.name), npc))
        .collect();

    parsed
        .into_iter()
        .filter_map(|suggestion| {
            let normalized_suggestion_name = normalize_name(&suggestion.name);
            let npc = normalized_name_map.get(&normalized_suggestion_name)?;

            Some(StagedNpc {
                character_id: npc.character_id,
                name: npc.name.clone(),
                sprite_asset: npc.sprite_asset.clone(),
                portrait_asset: npc.portrait_asset.clone(),
                is_present: true,
                reasoning: format!("[LLM] {}", suggestion.reason),
                is_hidden_from_players: false,
                mood: Some(npc.default_mood.to_string()),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_name_trims_whitespace() {
        assert_eq!(normalize_name("  John Smith  "), "john smith");
    }

    #[test]
    fn normalize_name_collapses_multiple_spaces() {
        assert_eq!(normalize_name("John    Smith"), "john smith");
    }

    #[test]
    fn normalize_name_handles_tabs_and_newlines() {
        assert_eq!(normalize_name("John\t\nSmith"), "john smith");
    }

    #[test]
    fn normalize_name_lowercases() {
        assert_eq!(normalize_name("JOHN SMITH"), "john smith");
    }

    #[test]
    fn normalize_name_combined() {
        assert_eq!(
            normalize_name("  Marcus   the   Bartender  "),
            "marcus the bartender"
        );
    }

    #[test]
    fn normalize_name_empty_string() {
        assert_eq!(normalize_name(""), "");
    }

    #[test]
    fn normalize_name_whitespace_only() {
        assert_eq!(normalize_name("   \t\n   "), "");
    }

    #[test]
    fn normalize_name_unicode_characters() {
        assert_eq!(normalize_name("José García"), "josé garcía");
        assert_eq!(normalize_name("Müller"), "müller");
        assert_eq!(normalize_name("北京"), "北京");
    }

    #[test]
    fn normalize_name_unicode_whitespace() {
        assert_eq!(normalize_name("John\u{00A0}Smith"), "john smith");
        assert_eq!(normalize_name("John\u{2003}Smith"), "john smith");
    }
}
