//! Converters between domain DirectorialNotes and protocol DirectorialContext
//!
//! These converters bridge the gap between:
//! - Domain layer: `DirectorialNotes` with typed enums (ToneGuidance, PacingGuidance)
//! - Protocol layer: `DirectorialContext` with string fields for wire transport

use std::collections::HashMap;

use wrldbldr_domain::value_objects::{
    DirectorialNotes, DomainNpcMotivation, PacingGuidance, ToneGuidance,
};
use wrldbldr_protocol::{DirectorialContext, NpcMotivationData};

/// Convert domain DirectorialNotes to protocol DirectorialContext
pub fn directorial_notes_to_context(notes: DirectorialNotes) -> DirectorialContext {
    DirectorialContext {
        scene_notes: notes.general_notes,
        tone: tone_to_string(&notes.tone),
        npc_motivations: notes
            .npc_motivations
            .into_iter()
            .map(|(char_id, motivation)| NpcMotivationData {
                character_id: char_id,
                emotional_guidance: motivation.current_mood,
                immediate_goal: motivation.immediate_goal,
                secret_agenda: motivation.secret_agenda,
            })
            .collect(),
        forbidden_topics: notes.forbidden_topics,
    }
}

/// Convert protocol DirectorialContext to domain DirectorialNotes
pub fn directorial_context_to_notes(ctx: DirectorialContext) -> DirectorialNotes {
    let npc_motivations: HashMap<String, DomainNpcMotivation> = ctx
        .npc_motivations
        .into_iter()
        .map(|m| {
            let motivation = DomainNpcMotivation::new(m.emotional_guidance, m.immediate_goal)
                .with_attitude("Neutral"); // Default, not in protocol
            let motivation = if let Some(secret) = m.secret_agenda {
                motivation.with_secret(secret)
            } else {
                motivation
            };
            (m.character_id, motivation)
        })
        .collect();

    DirectorialNotes {
        general_notes: ctx.scene_notes,
        tone: parse_tone(&ctx.tone),
        npc_motivations,
        forbidden_topics: ctx.forbidden_topics,
        allowed_tools: Vec::new(),      // Not in protocol
        suggested_beats: Vec::new(),    // Not in protocol
        pacing: PacingGuidance::Natural, // Not in protocol, default to Natural
    }
}

/// Parse a tone string to ToneGuidance enum
pub fn parse_tone(s: &str) -> ToneGuidance {
    match s.to_lowercase().as_str() {
        "neutral" | "" => ToneGuidance::Neutral,
        "serious" => ToneGuidance::Serious,
        "lighthearted" => ToneGuidance::Lighthearted,
        "tense" => ToneGuidance::Tense,
        "mysterious" => ToneGuidance::Mysterious,
        "exciting" => ToneGuidance::Exciting,
        "contemplative" => ToneGuidance::Contemplative,
        "creepy" => ToneGuidance::Creepy,
        "romantic" => ToneGuidance::Romantic,
        "comedic" => ToneGuidance::Comedic,
        _ => ToneGuidance::Custom(s.to_string()),
    }
}

/// Convert ToneGuidance enum to string
fn tone_to_string(tone: &ToneGuidance) -> String {
    match tone {
        ToneGuidance::Neutral => "neutral".to_string(),
        ToneGuidance::Serious => "serious".to_string(),
        ToneGuidance::Lighthearted => "lighthearted".to_string(),
        ToneGuidance::Tense => "tense".to_string(),
        ToneGuidance::Mysterious => "mysterious".to_string(),
        ToneGuidance::Exciting => "exciting".to_string(),
        ToneGuidance::Contemplative => "contemplative".to_string(),
        ToneGuidance::Creepy => "creepy".to_string(),
        ToneGuidance::Romantic => "romantic".to_string(),
        ToneGuidance::Comedic => "comedic".to_string(),
        ToneGuidance::Custom(s) => s.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip_conversion() {
        let notes = DirectorialNotes::new()
            .with_general_notes("Test scene notes")
            .with_tone(ToneGuidance::Tense)
            .with_forbidden_topic("violence");

        let ctx = directorial_notes_to_context(notes.clone());
        let back = directorial_context_to_notes(ctx);

        assert_eq!(back.general_notes, "Test scene notes");
        assert_eq!(back.tone, ToneGuidance::Tense);
        assert_eq!(back.forbidden_topics, vec!["violence"]);
    }

    #[test]
    fn test_parse_tone() {
        assert_eq!(parse_tone("tense"), ToneGuidance::Tense);
        assert_eq!(parse_tone("TENSE"), ToneGuidance::Tense);
        assert_eq!(parse_tone(""), ToneGuidance::Neutral);
        assert_eq!(parse_tone("custom vibe"), ToneGuidance::Custom("custom vibe".to_string()));
    }
}
