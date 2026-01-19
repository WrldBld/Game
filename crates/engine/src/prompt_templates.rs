//! Configurable LLM prompt templates used by the engine.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Categories for organizing prompt templates in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PromptTemplateCategory {
    /// NPC dialogue and roleplay.
    Dialogue,
    /// NPC presence/staging decisions.
    Staging,
    /// Challenge outcome suggestions.
    Outcomes,
    /// Worldbuilding content suggestions.
    Suggestions,
    /// Context summarization.
    Summarization,
}

impl PromptTemplateCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dialogue => "dialogue",
            Self::Staging => "staging",
            Self::Outcomes => "outcomes",
            Self::Suggestions => "suggestions",
            Self::Summarization => "summarization",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Dialogue => "NPC Dialogue",
            Self::Staging => "NPC Staging",
            Self::Outcomes => "Challenge Outcomes",
            Self::Suggestions => "Worldbuilding Suggestions",
            Self::Summarization => "Context Summarization",
        }
    }
}

/// Metadata about a prompt template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplateMetadata {
    /// Unique key for this template.
    pub key: String,
    /// Human-readable label.
    pub label: String,
    /// Description of what this template is used for.
    pub description: String,
    /// Category for UI grouping.
    pub category: PromptTemplateCategory,
    /// The hard-coded default value.
    pub default_value: String,
    /// Environment variable name for override.
    pub env_var: String,
}

/// All prompt template keys as constants.
pub mod keys {
    // === Dialogue System ===
    /// The response format instructions shown to the LLM for NPC dialogue.
    pub const DIALOGUE_RESPONSE_FORMAT: &str = "dialogue.response_format";
    /// Challenge suggestion format instructions.
    pub const DIALOGUE_CHALLENGE_SUGGESTION_FORMAT: &str = "dialogue.challenge_suggestion_format";
    /// Narrative event suggestion format instructions.
    pub const DIALOGUE_NARRATIVE_EVENT_FORMAT: &str = "dialogue.narrative_event_format";

    // === Staging System ===
    /// System prompt for NPC staging decisions.
    pub const STAGING_SYSTEM_PROMPT: &str = "staging.system_prompt";
    /// Instructions for staging response format.
    pub const STAGING_RESPONSE_FORMAT: &str = "staging.response_format";
    /// Role description for staging LLM.
    pub const STAGING_ROLE_INSTRUCTIONS: &str = "staging.role_instructions";

    // === Outcome Suggestions ===
    /// System prompt for outcome description generation.
    pub const OUTCOME_SYSTEM_PROMPT: &str = "outcome.system_prompt";
    /// System prompt for outcome branch generation.
    pub const OUTCOME_BRANCH_SYSTEM_PROMPT: &str = "outcome.branch_system_prompt";

    // === Worldbuilding Suggestions ===
    /// Character name generation prompt.
    pub const SUGGESTION_CHARACTER_NAME: &str = "suggestion.character_name";
    /// Character description generation prompt.
    pub const SUGGESTION_CHARACTER_DESCRIPTION: &str = "suggestion.character_description";
    /// Character wants/desires generation prompt.
    pub const SUGGESTION_CHARACTER_WANTS: &str = "suggestion.character_wants";
    /// Character fears generation prompt.
    pub const SUGGESTION_CHARACTER_FEARS: &str = "suggestion.character_fears";
    /// Character backstory generation prompt.
    pub const SUGGESTION_CHARACTER_BACKSTORY: &str = "suggestion.character_backstory";
    /// Location name generation prompt.
    pub const SUGGESTION_LOCATION_NAME: &str = "suggestion.location_name";
    /// Location description generation prompt.
    pub const SUGGESTION_LOCATION_DESCRIPTION: &str = "suggestion.location_description";
    /// Location atmosphere generation prompt.
    pub const SUGGESTION_LOCATION_ATMOSPHERE: &str = "suggestion.location_atmosphere";
    /// Location features generation prompt.
    pub const SUGGESTION_LOCATION_FEATURES: &str = "suggestion.location_features";
    /// Location secrets generation prompt.
    pub const SUGGESTION_LOCATION_SECRETS: &str = "suggestion.location_secrets";

    // === Actantial Model Suggestions ===
    /// Deflection behavior suggestion prompt.
    pub const SUGGESTION_DEFLECTION_BEHAVIOR: &str = "suggestion.deflection_behavior";
    /// Behavioral tells suggestion prompt.
    pub const SUGGESTION_BEHAVIORAL_TELLS: &str = "suggestion.behavioral_tells";
    /// Want description suggestion prompt (actantial-aware).
    pub const SUGGESTION_WANT_DESCRIPTION: &str = "suggestion.want_description";
    /// Actantial reason suggestion prompt.
    pub const SUGGESTION_ACTANTIAL_REASON: &str = "suggestion.actantial_reason";
}

/// Default values for all prompt templates.
pub mod defaults {
    /// Response format instructions for NPC dialogue.
    pub const DIALOGUE_RESPONSE_FORMAT: &str = r#"
RESPONSE FORMAT:
You must respond in the following format:

<reasoning>
Your internal thoughts about how to respond. Consider:
- What does your character know about the situation?
- How does your character feel about this moment?
- What are your character's immediate goals in this conversation?
- Are any game mechanics or tool calls dramatically appropriate?
- How do the directorial notes influence your response?
- Could the player's action trigger any of the active challenges?
- Could the player's action or dialogue trigger any narrative events?
This section is hidden from the player but shown to the Game Master for review.
</reasoning>

<dialogue>
Your character's spoken response. Stay in character.
Write naturally as the character would speak. Use appropriate dialect or speech patterns.
Keep responses concise but meaningful (1-3 sentences typically).

EXPRESSION MARKERS (use these to show character emotions and actions):
- Use *expression* to change the character's visual expression (e.g., *happy*, *suspicious*)
- Use *action* for physical actions that don't have sprites (e.g., *sighs*, *crosses arms*)
- Use *expression|fallback* if the expression might not exist (e.g., *nervous|worried*)
- Place markers naturally within the dialogue text
- The expression shown will persist until the next marker or end of dialogue

Example: *curious* "You seek the Heartstone?" *narrows eyes* *suspicious* "But why?"

Available expressions depend on the character - use only expressions from the character's available list.
Actions (like *sighs*, *crosses arms*) are shown as italicized text and don't change the sprite.
</dialogue>

<topics>
List 1-3 key topics discussed in this exchange, one per line.
Choose specific, meaningful topics like:
- quest_information
- local_history
- directions
- trade_negotiation
- rumors
- personal_story
- threat_warning
- request_for_help
</topics>

<suggested_beats>
Optional narrative suggestions for the Game Master, one per line.
These help shape the story direction and are only suggestions.
</suggested_beats>

AVAILABLE TOOLS:
You may propose tool calls to affect game state. Use XML format:
<tool name="tool_name">{"param": "value", "other_param": "value"}</tool>

Available tools:
- give_item: Give an item to the player (item_name: string, description: string)
- reveal_info: Reveal plot-relevant information (info_type: string, content: string, importance: "minor"|"major"|"critical")
- change_relationship: Modify relationship level with player (change: "improve"|"worsen", amount: "slight"|"moderate"|"significant", reason: string)
- change_disposition: Change NPC's emotional stance toward player (new_disposition: "friendly"|"neutral"|"suspicious"|"hostile"|"grateful"|"respectful"|"dismissive", reason: string)
- change_mood: Change NPC's current emotional state (new_mood: "happy"|"calm"|"anxious"|"excited"|"melancholic"|"irritated"|"alert"|"bored"|"fearful"|"hopeful"|"curious"|"contemplative"|"amused"|"weary"|"confident"|"nervous", reason: string)
- trigger_event: Trigger a game event (event_type: string, description: string)

Example tool call:
<tool name="give_item">{"item_name": "Rusty Key", "description": "An old iron key, slightly rusted"}</tool>

Only propose tool calls when dramatically appropriate. The Game Master will approve or reject them.
Disposition/mood changes require DM approval and should reflect significant emotional shifts.
"#;

    /// Challenge suggestion format for dialogue.
    pub const DIALOGUE_CHALLENGE_SUGGESTION_FORMAT: &str = r#"CHALLENGE ANALYSIS (REQUIRED when challenges are listed above):
For EACH active challenge, output a <challenge> block:

<challenge name="[exact challenge name]">
  <quote>[player's exact words that match a trigger keyword, or "none"]</quote>
  <trigger>[YES or NO]</trigger>
</challenge>

Example:
<challenge name="Convince Grom to Share His Past">
  <quote>tell me about your past</quote>
  <trigger>YES</trigger>
</challenge>
<challenge name="Investigate the Mill">
  <quote>none</quote>
  <trigger>NO</trigger>
</challenge>"#;

    /// Narrative event suggestion format for dialogue.
    pub const DIALOGUE_NARRATIVE_EVENT_FORMAT: &str = r#"NARRATIVE EVENT ANALYSIS (REQUIRED when events are listed above):
For EACH active narrative event, output an <event> block:

<event name="[exact event name]">
  <quote>[player's exact words that match a trigger keyword, or "none"]</quote>
  <trigger>[YES or NO]</trigger>
</event>

Example:
<event name="Marta's Knowledge">
  <quote>what happened at the mill</quote>
  <trigger>YES</trigger>
</event>
<event name="The Stranger's Warning">
  <quote>none</quote>
  <trigger>NO</trigger>
</event>"#;

    /// System prompt for staging decisions.
    pub const STAGING_SYSTEM_PROMPT: &str =
        "You are a game master assistant helping determine NPC presence.";

    /// Role instructions for staging.
    pub const STAGING_ROLE_INSTRUCTIONS: &str = r#"## Your Role
You may AGREE with or OVERRIDE the rules based on narrative considerations.
Consider: story reasons, interesting opportunities, conflicts, current context.
"#;

    /// Response format for staging.
    pub const STAGING_RESPONSE_FORMAT: &str = r#"## Response Format
Respond in JSON format with an array of objects:
[{"name": "NPC Name", "is_present": true/false, "is_hidden_from_players": true/false, "reasoning": "Brief explanation"}]

Use is_hidden_from_players=true for NPCs that should not be visible to players yet (e.g. watching from shadows, disguised, behind a curtain).
Be realistic and consistent. Don't have everyone present at once unless it makes sense."#;

    /// System prompt for outcome suggestions.
    pub const OUTCOME_SYSTEM_PROMPT: &str = r#"You are a creative TTRPG game master assistant specializing in vivid challenge outcomes.

Your task is to generate engaging outcome descriptions for skill challenges. Each description should:
- Be 2-3 sentences of evocative narrative
- Match the outcome tier (critical success, success, failure, critical failure)
- Describe what happens as a result of the roll
- Be written in second person ("You...")
- Add sensory details and dramatic tension

Format: Return exactly 3 suggestions, each on its own line. Do not number them or add prefixes."#;

    /// System prompt for outcome branch generation (with placeholder for branch_count).
    pub const OUTCOME_BRANCH_SYSTEM_PROMPT: &str = r#"You are a creative TTRPG game master assistant specializing in vivid challenge outcomes.

Your task is to generate {branch_count} distinct outcome branches for skill challenges. Each branch should offer a different narrative direction while staying consistent with the outcome tier.

For each branch, provide:
1. A SHORT TITLE (3-5 words) summarizing the outcome
2. A DESCRIPTION (2-3 sentences) of evocative narrative in second person ("You...")

The branches should offer meaningfully different narrative paths, not just rephrased versions of the same outcome.

FORMAT: Use this exact format for each branch, separated by blank lines:
TITLE: [short title]
DESCRIPTION: [narrative description]

Do not number the branches or add any other formatting."#;

    /// Character name suggestion prompt.
    pub const SUGGESTION_CHARACTER_NAME: &str = r#"Generate 5 unique character names for a {entity_type} character in a {world_setting} setting. The character archetype is: {hints}. Return only the names, one per line, no numbering or explanations."#;

    /// Character description suggestion prompt.
    pub const SUGGESTION_CHARACTER_DESCRIPTION: &str = r#"Generate 3 different physical descriptions for {entity_name}. Setting: {world_setting}. Archetype: {hints}. Each description should be 2-3 sentences covering appearance, mannerisms, and voice. Return each description on its own line, separated by blank lines."#;

    /// Character wants suggestion prompt.
    pub const SUGGESTION_CHARACTER_WANTS: &str = r#"Generate 4 different character motivations/wants for {entity_name}. Archetype: {hints}. Description: {additional_context}. Each want should be a single sentence describing what the character desires. Return each want on its own line."#;

    /// Character fears suggestion prompt.
    pub const SUGGESTION_CHARACTER_FEARS: &str = r#"Generate 4 different fears for {entity_name}. Archetype: {hints}. Wants: {additional_context}. Each fear should be a single sentence. Consider fears that create interesting dramatic tension. Return each fear on its own line."#;

    /// Character backstory suggestion prompt.
    pub const SUGGESTION_CHARACTER_BACKSTORY: &str = r#"Generate 2 different backstory options for {entity_name}. Archetype: {hints}. Wants: {additional_context}. Fears: {world_setting}. Each backstory should be 3-4 sentences covering origin, key events, and how they became who they are. Return each backstory separated by a blank line."#;

    /// Location name suggestion prompt.
    pub const SUGGESTION_LOCATION_NAME: &str = r#"Generate 5 unique names for a {entity_type} in a {world_setting} setting. Names should be evocative and memorable. Return only the names, one per line."#;

    /// Location description suggestion prompt.
    pub const SUGGESTION_LOCATION_DESCRIPTION: &str = r#"Generate 3 different descriptions for {entity_name} (a {entity_type}). Setting: {world_setting}. Each description should be 2-3 sentences covering what stands out visually, sounds, and smells. Return each description separated by a blank line."#;

    /// Location atmosphere suggestion prompt.
    pub const SUGGESTION_LOCATION_ATMOSPHERE: &str = r#"Generate 4 different atmosphere/mood options for {entity_name}. Location type: {entity_type}. Description: {additional_context}. Each should be a short phrase (2-5 words) capturing the feel. Examples: 'Tense and watchful', 'Cozy but cluttered', 'Eerily silent'. Return each atmosphere on its own line."#;

    /// Location features suggestion prompt.
    pub const SUGGESTION_LOCATION_FEATURES: &str = r#"Generate 5 notable features or points of interest for {entity_name}. Location type: {entity_type}. Atmosphere: {hints}. Each should be a single sentence describing something players might interact with or notice. Return each feature on its own line."#;

    /// Location secrets suggestion prompt.
    pub const SUGGESTION_LOCATION_SECRETS: &str = r#"Generate 3 hidden secrets that could be discovered in {entity_name}. Location type: {entity_type}. Features: {additional_context}. Each secret should be something players might find with investigation, 1-2 sentences. Return each secret separated by a blank line."#;

    // === Actantial Model Suggestions ===
    /// Deflection behavior suggestion prompt.
    pub const SUGGESTION_DEFLECTION_BEHAVIOR: &str = r#"Generate 3 different deflection behaviors for {entity_name} when trying to hide their desire for: {hints}.
Setting: {world_setting}.
Character context: {additional_context}.

A deflection behavior is how a character acts to conceal their true want - nervous habits, diversionary topics, or defensive responses.
Each suggestion should be 1-2 sentences describing the specific behavior.
Return each suggestion on its own line."#;

    /// Behavioral tells suggestion prompt.
    pub const SUGGESTION_BEHAVIORAL_TELLS: &str = r#"Generate 3 different behavioral tells for {entity_name} that reveal their hidden desire for: {hints}.
Setting: {world_setting}.
Character context: {additional_context}.

A behavioral tell is a subtle sign that betrays the character's true motivation - a glance, a pause, an involuntary reaction.
These are clues perceptive players might notice.
Each suggestion should be 1-2 sentences describing the specific tell.
Return each suggestion on its own line."#;

    /// Want description suggestion prompt (actantial-aware).
    pub const SUGGESTION_WANT_DESCRIPTION: &str = r#"Generate 3 different want descriptions for {entity_name} in a {world_setting} setting.
Character archetype: {hints}.
Additional context: {additional_context}.

Each want should be phrased as a specific desire or goal, not a personality trait.
Focus on what the character actively pursues or needs.
Each description should be a single compelling sentence.
Return each want on its own line."#;

    /// Actantial reason suggestion prompt.
    pub const SUGGESTION_ACTANTIAL_REASON: &str = r#"Generate 3 different reasons why {entity_name} views {hints} as {additional_context} regarding their current goal.
Setting: {world_setting}.

Provide narrative justifications for this actantial relationship that could drive interesting roleplay.
Each reason should explain the history, incident, or belief that created this dynamic.
Each suggestion should be 1-2 sentences.
Return each reason on its own line."#;
}

/// Convert a template key to its environment variable name.
pub fn key_to_env_var(key: &str) -> String {
    format!("WRLDBLDR_PROMPT_{}", key.to_uppercase().replace('.', "_"))
}

/// Get the default value for a template key.
pub fn get_default(key: &str) -> Option<&'static str> {
    match key {
        keys::DIALOGUE_RESPONSE_FORMAT => Some(defaults::DIALOGUE_RESPONSE_FORMAT),
        keys::DIALOGUE_CHALLENGE_SUGGESTION_FORMAT => {
            Some(defaults::DIALOGUE_CHALLENGE_SUGGESTION_FORMAT)
        }
        keys::DIALOGUE_NARRATIVE_EVENT_FORMAT => Some(defaults::DIALOGUE_NARRATIVE_EVENT_FORMAT),
        keys::STAGING_SYSTEM_PROMPT => Some(defaults::STAGING_SYSTEM_PROMPT),
        keys::STAGING_ROLE_INSTRUCTIONS => Some(defaults::STAGING_ROLE_INSTRUCTIONS),
        keys::STAGING_RESPONSE_FORMAT => Some(defaults::STAGING_RESPONSE_FORMAT),
        keys::OUTCOME_SYSTEM_PROMPT => Some(defaults::OUTCOME_SYSTEM_PROMPT),
        keys::OUTCOME_BRANCH_SYSTEM_PROMPT => Some(defaults::OUTCOME_BRANCH_SYSTEM_PROMPT),
        keys::SUGGESTION_CHARACTER_NAME => Some(defaults::SUGGESTION_CHARACTER_NAME),
        keys::SUGGESTION_CHARACTER_DESCRIPTION => Some(defaults::SUGGESTION_CHARACTER_DESCRIPTION),
        keys::SUGGESTION_CHARACTER_WANTS => Some(defaults::SUGGESTION_CHARACTER_WANTS),
        keys::SUGGESTION_CHARACTER_FEARS => Some(defaults::SUGGESTION_CHARACTER_FEARS),
        keys::SUGGESTION_CHARACTER_BACKSTORY => Some(defaults::SUGGESTION_CHARACTER_BACKSTORY),
        keys::SUGGESTION_LOCATION_NAME => Some(defaults::SUGGESTION_LOCATION_NAME),
        keys::SUGGESTION_LOCATION_DESCRIPTION => Some(defaults::SUGGESTION_LOCATION_DESCRIPTION),
        keys::SUGGESTION_LOCATION_ATMOSPHERE => Some(defaults::SUGGESTION_LOCATION_ATMOSPHERE),
        keys::SUGGESTION_LOCATION_FEATURES => Some(defaults::SUGGESTION_LOCATION_FEATURES),
        keys::SUGGESTION_LOCATION_SECRETS => Some(defaults::SUGGESTION_LOCATION_SECRETS),
        // Actantial suggestions
        keys::SUGGESTION_DEFLECTION_BEHAVIOR => Some(defaults::SUGGESTION_DEFLECTION_BEHAVIOR),
        keys::SUGGESTION_BEHAVIORAL_TELLS => Some(defaults::SUGGESTION_BEHAVIORAL_TELLS),
        keys::SUGGESTION_WANT_DESCRIPTION => Some(defaults::SUGGESTION_WANT_DESCRIPTION),
        keys::SUGGESTION_ACTANTIAL_REASON => Some(defaults::SUGGESTION_ACTANTIAL_REASON),
        _ => None,
    }
}

/// Get metadata for all prompt templates.
pub fn prompt_template_metadata() -> Vec<PromptTemplateMetadata> {
    vec![
        // Dialogue
        PromptTemplateMetadata {
            key: keys::DIALOGUE_RESPONSE_FORMAT.to_string(),
            label: "Response Format Instructions".to_string(),
            description: "Instructions shown to the LLM for how to format NPC dialogue responses"
                .to_string(),
            category: PromptTemplateCategory::Dialogue,
            default_value: defaults::DIALOGUE_RESPONSE_FORMAT.to_string(),
            env_var: key_to_env_var(keys::DIALOGUE_RESPONSE_FORMAT),
        },
        PromptTemplateMetadata {
            key: keys::DIALOGUE_CHALLENGE_SUGGESTION_FORMAT.to_string(),
            label: "Challenge Suggestion Format".to_string(),
            description: "Format instructions for suggesting skill challenges during dialogue"
                .to_string(),
            category: PromptTemplateCategory::Dialogue,
            default_value: defaults::DIALOGUE_CHALLENGE_SUGGESTION_FORMAT.to_string(),
            env_var: key_to_env_var(keys::DIALOGUE_CHALLENGE_SUGGESTION_FORMAT),
        },
        PromptTemplateMetadata {
            key: keys::DIALOGUE_NARRATIVE_EVENT_FORMAT.to_string(),
            label: "Narrative Event Suggestion Format".to_string(),
            description: "Format instructions for suggesting narrative events during dialogue"
                .to_string(),
            category: PromptTemplateCategory::Dialogue,
            default_value: defaults::DIALOGUE_NARRATIVE_EVENT_FORMAT.to_string(),
            env_var: key_to_env_var(keys::DIALOGUE_NARRATIVE_EVENT_FORMAT),
        },
        // Staging
        PromptTemplateMetadata {
            key: keys::STAGING_SYSTEM_PROMPT.to_string(),
            label: "Staging System Prompt".to_string(),
            description: "System prompt for NPC presence/staging decisions".to_string(),
            category: PromptTemplateCategory::Staging,
            default_value: defaults::STAGING_SYSTEM_PROMPT.to_string(),
            env_var: key_to_env_var(keys::STAGING_SYSTEM_PROMPT),
        },
        PromptTemplateMetadata {
            key: keys::STAGING_ROLE_INSTRUCTIONS.to_string(),
            label: "Staging Role Instructions".to_string(),
            description: "Instructions explaining the LLM's role in staging decisions".to_string(),
            category: PromptTemplateCategory::Staging,
            default_value: defaults::STAGING_ROLE_INSTRUCTIONS.to_string(),
            env_var: key_to_env_var(keys::STAGING_ROLE_INSTRUCTIONS),
        },
        PromptTemplateMetadata {
            key: keys::STAGING_RESPONSE_FORMAT.to_string(),
            label: "Staging Response Format".to_string(),
            description: "Expected JSON response format for staging decisions".to_string(),
            category: PromptTemplateCategory::Staging,
            default_value: defaults::STAGING_RESPONSE_FORMAT.to_string(),
            env_var: key_to_env_var(keys::STAGING_RESPONSE_FORMAT),
        },
        // Outcomes
        PromptTemplateMetadata {
            key: keys::OUTCOME_SYSTEM_PROMPT.to_string(),
            label: "Outcome System Prompt".to_string(),
            description: "System prompt for generating challenge outcome descriptions".to_string(),
            category: PromptTemplateCategory::Outcomes,
            default_value: defaults::OUTCOME_SYSTEM_PROMPT.to_string(),
            env_var: key_to_env_var(keys::OUTCOME_SYSTEM_PROMPT),
        },
        PromptTemplateMetadata {
            key: keys::OUTCOME_BRANCH_SYSTEM_PROMPT.to_string(),
            label: "Outcome Branch System Prompt".to_string(),
            description:
                "System prompt for generating outcome branches (use {branch_count} placeholder)"
                    .to_string(),
            category: PromptTemplateCategory::Outcomes,
            default_value: defaults::OUTCOME_BRANCH_SYSTEM_PROMPT.to_string(),
            env_var: key_to_env_var(keys::OUTCOME_BRANCH_SYSTEM_PROMPT),
        },
        // Suggestions
        PromptTemplateMetadata {
            key: keys::SUGGESTION_CHARACTER_NAME.to_string(),
            label: "Character Name Suggestions".to_string(),
            description: "Prompt for generating character name suggestions".to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_CHARACTER_NAME.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_CHARACTER_NAME),
        },
        PromptTemplateMetadata {
            key: keys::SUGGESTION_CHARACTER_DESCRIPTION.to_string(),
            label: "Character Description Suggestions".to_string(),
            description: "Prompt for generating character description suggestions".to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_CHARACTER_DESCRIPTION.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_CHARACTER_DESCRIPTION),
        },
        PromptTemplateMetadata {
            key: keys::SUGGESTION_CHARACTER_WANTS.to_string(),
            label: "Character Wants Suggestions".to_string(),
            description: "Prompt for generating character motivation suggestions".to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_CHARACTER_WANTS.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_CHARACTER_WANTS),
        },
        PromptTemplateMetadata {
            key: keys::SUGGESTION_CHARACTER_FEARS.to_string(),
            label: "Character Fears Suggestions".to_string(),
            description: "Prompt for generating character fear suggestions".to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_CHARACTER_FEARS.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_CHARACTER_FEARS),
        },
        PromptTemplateMetadata {
            key: keys::SUGGESTION_CHARACTER_BACKSTORY.to_string(),
            label: "Character Backstory Suggestions".to_string(),
            description: "Prompt for generating character backstory suggestions".to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_CHARACTER_BACKSTORY.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_CHARACTER_BACKSTORY),
        },
        PromptTemplateMetadata {
            key: keys::SUGGESTION_LOCATION_NAME.to_string(),
            label: "Location Name Suggestions".to_string(),
            description: "Prompt for generating location name suggestions".to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_LOCATION_NAME.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_LOCATION_NAME),
        },
        PromptTemplateMetadata {
            key: keys::SUGGESTION_LOCATION_DESCRIPTION.to_string(),
            label: "Location Description Suggestions".to_string(),
            description: "Prompt for generating location description suggestions".to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_LOCATION_DESCRIPTION.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_LOCATION_DESCRIPTION),
        },
        PromptTemplateMetadata {
            key: keys::SUGGESTION_LOCATION_ATMOSPHERE.to_string(),
            label: "Location Atmosphere Suggestions".to_string(),
            description: "Prompt for generating location atmosphere suggestions".to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_LOCATION_ATMOSPHERE.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_LOCATION_ATMOSPHERE),
        },
        PromptTemplateMetadata {
            key: keys::SUGGESTION_LOCATION_FEATURES.to_string(),
            label: "Location Features Suggestions".to_string(),
            description: "Prompt for generating location feature suggestions".to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_LOCATION_FEATURES.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_LOCATION_FEATURES),
        },
        PromptTemplateMetadata {
            key: keys::SUGGESTION_LOCATION_SECRETS.to_string(),
            label: "Location Secrets Suggestions".to_string(),
            description: "Prompt for generating location secret suggestions".to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_LOCATION_SECRETS.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_LOCATION_SECRETS),
        },
        // Actantial Model Suggestions
        PromptTemplateMetadata {
            key: keys::SUGGESTION_DEFLECTION_BEHAVIOR.to_string(),
            label: "Deflection Behavior Suggestions".to_string(),
            description: "Prompt for generating NPC deflection behaviors for hidden wants"
                .to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_DEFLECTION_BEHAVIOR.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_DEFLECTION_BEHAVIOR),
        },
        PromptTemplateMetadata {
            key: keys::SUGGESTION_BEHAVIORAL_TELLS.to_string(),
            label: "Behavioral Tells Suggestions".to_string(),
            description: "Prompt for generating behavioral tells that reveal hidden NPC wants"
                .to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_BEHAVIORAL_TELLS.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_BEHAVIORAL_TELLS),
        },
        PromptTemplateMetadata {
            key: keys::SUGGESTION_WANT_DESCRIPTION.to_string(),
            label: "Want Description Suggestions".to_string(),
            description: "Prompt for generating actantial-aware want descriptions".to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_WANT_DESCRIPTION.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_WANT_DESCRIPTION),
        },
        PromptTemplateMetadata {
            key: keys::SUGGESTION_ACTANTIAL_REASON.to_string(),
            label: "Actantial Reason Suggestions".to_string(),
            description: "Prompt for generating reasons for actantial relationships".to_string(),
            category: PromptTemplateCategory::Suggestions,
            default_value: defaults::SUGGESTION_ACTANTIAL_REASON.to_string(),
            env_var: key_to_env_var(keys::SUGGESTION_ACTANTIAL_REASON),
        },
    ]
}

/// Get all known template keys.
pub fn all_keys() -> Vec<&'static str> {
    vec![
        keys::DIALOGUE_RESPONSE_FORMAT,
        keys::DIALOGUE_CHALLENGE_SUGGESTION_FORMAT,
        keys::DIALOGUE_NARRATIVE_EVENT_FORMAT,
        keys::STAGING_SYSTEM_PROMPT,
        keys::STAGING_ROLE_INSTRUCTIONS,
        keys::STAGING_RESPONSE_FORMAT,
        keys::OUTCOME_SYSTEM_PROMPT,
        keys::OUTCOME_BRANCH_SYSTEM_PROMPT,
        keys::SUGGESTION_CHARACTER_NAME,
        keys::SUGGESTION_CHARACTER_DESCRIPTION,
        keys::SUGGESTION_CHARACTER_WANTS,
        keys::SUGGESTION_CHARACTER_FEARS,
        keys::SUGGESTION_CHARACTER_BACKSTORY,
        keys::SUGGESTION_LOCATION_NAME,
        keys::SUGGESTION_LOCATION_DESCRIPTION,
        keys::SUGGESTION_LOCATION_ATMOSPHERE,
        keys::SUGGESTION_LOCATION_FEATURES,
        keys::SUGGESTION_LOCATION_SECRETS,
        // Actantial suggestions
        keys::SUGGESTION_DEFLECTION_BEHAVIOR,
        keys::SUGGESTION_BEHAVIORAL_TELLS,
        keys::SUGGESTION_WANT_DESCRIPTION,
        keys::SUGGESTION_ACTANTIAL_REASON,
    ]
}
