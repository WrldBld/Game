# Mood & Expression System Redesign

**Created**: 2025-12-26  
**Status**: PLANNING  
**Priority**: High  
**Estimated Effort**: 3-4 days

---

## Executive Summary

This plan redesigns the mood system from a static DM-controlled value to a dynamic, dialogue-embedded expression system. Mood markers are inline with dialogue text, enabling expression changes during typewriter playback. Both NPCs (LLM-generated) and PCs (player-typed) use the same marker format.

### Key Features

- **Inline mood markers**: `*happy*` or `*excited|happy*` embedded in dialogue
- **Expression mapping**: Mood text maps to available character expressions
- **Pipe format**: `*mood|expression*` allows custom moods with known expressions
- **Action markers**: `*sighs*` `*laughs*` for transient physical actions
- **Typewriter integration**: Expressions change as dialogue plays out
- **Tool-based mood changes**: LLM can propose permanent mood state changes
- **PC support**: Players can add mood/action markers to their input
- **Mood history**: Track mood changes over conversation

---

## Marker Format Specification

### Syntax

```
*word*           → mood and expression are the same
*mood|expression* → custom mood displayed, uses expression sprite
*action text*    → transient action (not in expression vocabulary)
```

### Examples

| Marker | Mood Displayed | Expression Used | Type |
|--------|---------------|-----------------|------|
| `*happy*` | happy | happy | mood |
| `*excited|happy*` | excited | happy | mood with mapping |
| `*nervous|afraid*` | nervous | afraid | mood with mapping |
| `*devastated|sad*` | devastated | sad | mood with mapping |
| `*sighs*` | — | — | action |
| `*slams fist on table*` | — | — | action |

### Distinction: Mood vs Action

- **Mood**: Single word or piped format, matches expression vocabulary
- **Action**: Multi-word or not in vocabulary, displayed inline, no expression change

### Dialogue Example

```
*curious* "You've come a long way, traveler." *narrows eyes* *suspicious* "But I wonder... what brings you to these forgotten ruins?"
```

**Playback**:
1. Expression → `curious`, mood badge → "curious"
2. Text: "You've come a long way, traveler."
3. Action displayed inline: *narrows eyes*
4. Expression → `suspicious`, mood badge → "suspicious"  
5. Text: "But I wonder... what brings you to these forgotten ruins?"

---

## Data Model

### Character Expression Configuration

```rust
// crates/domain/src/entities/character.rs

/// Expression configuration for a character
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExpressionConfig {
    /// Available expression names with sprites (e.g., ["neutral", "happy", "sad"])
    pub expressions: Vec<String>,
    /// Custom actions this character uses (e.g., ["sighs", "laughs nervously"])
    pub actions: Vec<String>,
    /// Default expression when no mood specified
    pub default_expression: String, // Default: "neutral"
}

pub struct Character {
    // ... existing fields ...
    
    /// Expression configuration for this character
    pub expression_config: ExpressionConfig,
}
```

### Player Character Expression Support

```rust
// crates/domain/src/entities/player_character.rs

pub struct PlayerCharacter {
    // ... existing fields ...
    
    /// Expression configuration (same as NPC)
    pub expression_config: ExpressionConfig,
}
```

### Parsed Dialogue Types

```rust
// crates/domain/src/value_objects/dialogue_markers.rs (NEW)

/// A single marker parsed from dialogue
#[derive(Debug, Clone, PartialEq)]
pub enum DialogueMarker {
    /// Mood marker with optional expression mapping
    Mood {
        /// The mood text to display (e.g., "excited")
        mood: String,
        /// The expression to use for sprite (e.g., "happy")
        /// If None, try mood as expression with fallback
        expression: Option<String>,
    },
    /// Transient action marker
    Action(String),
}

/// A segment of dialogue with its associated marker
#[derive(Debug, Clone)]
pub struct DialogueSegment {
    /// Text content (marker stripped)
    pub text: String,
    /// Marker that precedes this text (if any)
    pub marker: Option<DialogueMarker>,
    /// Character position where this segment starts in original text
    pub start_pos: usize,
}

/// Fully parsed dialogue
#[derive(Debug, Clone)]
pub struct ParsedDialogue {
    /// Ordered segments
    pub segments: Vec<DialogueSegment>,
    /// Plain text with all markers stripped (for logging/accessibility)
    pub plain_text: String,
    /// Original text with markers (for storage)
    pub raw_text: String,
}

impl ParsedDialogue {
    /// Parse dialogue text with markers
    pub fn parse(text: &str) -> Self { ... }
    
    /// Get expression changes with their character positions
    pub fn expression_changes(&self) -> Vec<(usize, String, Option<String>)> {
        // Returns (char_position, mood, expression)
    }
}
```

### Mood History

```rust
// crates/domain/src/value_objects/mood_history.rs (NEW)

/// A recorded mood change in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodHistoryEntry {
    /// Character who expressed this mood
    pub character_id: String,
    /// Character name for display
    pub character_name: String,
    /// The mood expressed
    pub mood: String,
    /// Expression used (may differ from mood)
    pub expression: Option<String>,
    /// When this occurred in game time
    pub game_time: Option<String>,
    /// When this occurred in real time
    pub timestamp: DateTime<Utc>,
    /// Context (first few words of dialogue)
    pub dialogue_preview: String,
}

/// Mood history for a conversation/session
#[derive(Debug, Clone, Default)]
pub struct MoodHistory {
    pub entries: Vec<MoodHistoryEntry>,
}
```

---

## Protocol Changes

### CharacterData Enhancement

```rust
// crates/protocol/src/messages.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterData {
    pub id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
    pub position: CharacterPosition,
    pub is_speaking: bool,
    
    // UPDATED: Current expression state
    pub current_expression: Option<String>,  // Expression for sprite
    pub current_mood: Option<String>,        // Mood text to display
    
    // NEW: Available expressions for this character
    pub available_expressions: Vec<String>,
    
    // NEW: Known actions for this character  
    pub available_actions: Vec<String>,
}
```

### New Messages

```rust
// Client → Server
ClientMessage::UpdateCharacterExpressions {
    character_id: String,
    expressions: Vec<String>,
    actions: Vec<String>,
    default_expression: String,
}

// Server → Client (broadcast on mood history update)
ServerMessage::MoodHistoryEntry {
    session_id: String,
    entry: MoodHistoryEntryData,
}

// Existing NpcMoodChanged updated to include expression
ServerMessage::NpcMoodChanged {
    npc_id: String,
    npc_name: String,
    pc_id: String,
    mood: String,
    expression: Option<String>,  // NEW
    relationship: String,
    reason: Option<String>,
}
```

### DialogueResponse (Unchanged)

The `text` field already contains the raw dialogue. Client parses markers from text.

```rust
ServerMessage::DialogueResponse {
    speaker_id: String,
    speaker_name: String,
    text: String,  // Contains markers: "*happy* Hello there!"
    choices: Vec<DialogueChoice>,
}
```

---

## LLM Integration

### Prompt Template Updates

Update `DIALOGUE_RESPONSE_FORMAT` in `prompt_templates.rs`:

```rust
pub const DIALOGUE_RESPONSE_FORMAT: &str = r#"
RESPONSE FORMAT:
You must respond in the following format:

<reasoning>
Your internal thoughts about how to respond. Consider:
- What does your character know about the situation?
- How does your character feel about this moment?
- What are your character's immediate goals in this conversation?
- Are any game mechanics or tool calls dramatically appropriate?
This section is hidden from the player but shown to the Game Master for review.
</reasoning>

<dialogue>
Your character's spoken response with mood and action markers.

MOOD MARKERS indicate emotional state. Place BEFORE the text they apply to.
Format: *mood* or *mood|expression* to map a custom mood to an available expression.

YOUR AVAILABLE EXPRESSIONS: {available_expressions}
- Use *expression* when mood matches an available expression
- Use *mood|expression* when using a custom mood word: *nervous|afraid*

ACTION MARKERS show brief physical actions inline: *sighs* *laughs* *slams fist*
YOUR KNOWN ACTIONS: {available_actions}

You may use multiple markers to show emotional shifts during speech.
Example: *curious* "Interesting..." *suspicious* "But why come to me?" *narrows eyes*
</dialogue>

<suggested_beats>
Optional narrative suggestions for the Game Master, one per line.
</suggested_beats>

AVAILABLE TOOLS:
You may propose tool calls to affect game state. Available tools:
- give_item: Give an item to the player
- reveal_info: Reveal plot-relevant information
- change_relationship: Modify relationship with player
- trigger_event: Trigger a game event
- change_mood: Permanently change your disposition toward the player
  Parameters: { "new_mood": "friendly|neutral|suspicious|hostile|...", "reason": "..." }
  Use this when an interaction significantly changes how you feel about the player.

Only propose tool calls when dramatically appropriate.
"#;
```

### Context Injection

In `prompt_builder.rs`, inject available expressions:

```rust
// When building character context
let expressions_str = character.expression_config.expressions.join(", ");
let actions_str = character.expression_config.actions.join(", ");

// Replace placeholders in template
prompt = prompt
    .replace("{available_expressions}", &expressions_str)
    .replace("{available_actions}", &actions_str);
```

### Change Mood Tool Call

Add to tool definitions:

```rust
ToolDefinition {
    name: "change_mood",
    description: "Permanently change your emotional disposition toward the player",
    parameters: json!({
        "type": "object",
        "properties": {
            "new_mood": {
                "type": "string",
                "description": "New mood state (e.g., friendly, suspicious, hostile)"
            },
            "reason": {
                "type": "string", 
                "description": "Why your mood changed"
            }
        },
        "required": ["new_mood", "reason"]
    }),
}
```

---

## UI Components

### DialogueState Updates

```rust
// crates/player-ui/src/presentation/state/dialogue_state.rs

pub struct DialogueState {
    // Existing
    pub speaker_name: Signal<String>,
    pub full_text: Signal<String>,
    pub displayed_text: Signal<String>,
    pub is_typing: Signal<bool>,
    pub choices: Signal<Vec<DialogueChoice>>,
    pub speaker_id: Signal<Option<String>>,
    pub is_llm_processing: Signal<bool>,
    
    // NEW: Expression state
    pub current_mood: Signal<Option<String>>,
    pub current_expression: Signal<Option<String>>,
    pub current_action: Signal<Option<String>>,
    
    // NEW: Parsed segments for typewriter
    parsed_segments: Signal<Vec<DialogueSegment>>,
    
    // NEW: Available expressions for current speaker
    pub speaker_expressions: Signal<Vec<String>>,
}
```

### Typewriter with Expression Changes

```rust
pub fn use_typewriter_effect_with_expressions(
    dialogue_state: &mut DialogueState,
) {
    use_effect(move || {
        let full_text = dialogue_state.full_text.read().clone();
        let expressions = dialogue_state.speaker_expressions.read().clone();
        let default_expr = expressions.first().cloned().unwrap_or("neutral".to_string());
        
        // Parse dialogue
        let parsed = ParsedDialogue::parse(&full_text);
        let expression_changes = parsed.expression_changes();
        
        // Set initial state
        dialogue_state.displayed_text.set(String::new());
        dialogue_state.is_typing.set(true);
        dialogue_state.current_mood.set(None);
        dialogue_state.current_expression.set(Some(default_expr.clone()));
        dialogue_state.current_action.set(None);
        
        spawn(async move {
            let plain_text = parsed.plain_text.clone();
            let mut change_idx = 0;
            
            for (i, ch) in plain_text.char_indices() {
                // Check for expression changes at this position
                while change_idx < expression_changes.len() 
                    && expression_changes[change_idx].0 <= i 
                {
                    let (_, ref mood, ref expr) = expression_changes[change_idx];
                    
                    // Determine if this is a mood or action
                    if is_action(mood, &expressions) {
                        dialogue_state.current_action.set(Some(mood.clone()));
                        // Clear action after brief display
                        spawn(async move {
                            sleep(Duration::from_millis(500)).await;
                            dialogue_state.current_action.set(None);
                        });
                    } else {
                        dialogue_state.current_mood.set(Some(mood.clone()));
                        let resolved = expr.clone()
                            .or_else(|| resolve_expression(mood, &expressions))
                            .unwrap_or(default_expr.clone());
                        dialogue_state.current_expression.set(Some(resolved));
                    }
                    change_idx += 1;
                }
                
                // Append character
                dialogue_state.displayed_text.write().push(ch);
                
                // Delay based on punctuation
                let delay = match ch {
                    '.' | '!' | '?' => 150,
                    ',' | ';' | ':' => 80,
                    _ => 30,
                };
                sleep(Duration::from_millis(delay)).await;
            }
            
            dialogue_state.is_typing.set(false);
        });
    });
}
```

### Character Sprite Updates

```rust
// crates/player-ui/src/presentation/components/visual_novel/character_sprite.rs

#[component]
pub fn CharacterSprite(props: CharacterSpriteProps) -> Element {
    let dialogue_state = use_dialogue_state();
    
    // Get current expression for this character
    let current_expression = if Some(&props.character.id) == dialogue_state.speaker_id.read().as_ref() {
        dialogue_state.current_expression.read().clone()
    } else {
        props.character.current_expression.clone()
    };
    
    let current_mood = if Some(&props.character.id) == dialogue_state.speaker_id.read().as_ref() {
        dialogue_state.current_mood.read().clone()
    } else {
        props.character.current_mood.clone()
    };
    
    // Build sprite URL with expression fallback
    let sprite_url = get_expression_sprite_url(
        props.character.sprite_asset.as_deref(),
        current_expression.as_deref(),
        &props.character.available_expressions,
    );
    
    rsx! {
        div {
            class: "character-sprite-container {position_class}",
            
            // Sprite image
            if let Some(url) = sprite_url {
                img {
                    src: "{url}",
                    class: "character-sprite {speaking_class}",
                }
            } else {
                // Placeholder silhouette
                CharacterPlaceholder { name: props.character.name.clone() }
            }
            
            // Mood badge (corner of sprite)
            if let Some(mood) = &current_mood {
                div {
                    class: "mood-badge",
                    "*{mood}*"
                }
            }
        }
    }
}

fn get_expression_sprite_url(
    base: Option<&str>,
    expression: Option<&str>,
    available: &[String],
) -> Option<String> {
    let base = base?;
    
    match expression {
        Some(expr) if available.contains(&expr.to_string()) => {
            // Try expression-specific: /assets/marcus_happy.png
            let base_without_ext = base.trim_end_matches(".png");
            Some(format!("{}_{}.png", base_without_ext, expr))
        }
        _ => {
            // Fallback to base sprite
            Some(base.to_string())
        }
    }
}
```

### Dialogue Box with Mood Display

```rust
// crates/player-ui/src/presentation/components/visual_novel/dialogue_box.rs

#[component]
pub fn DialogueBox(props: DialogueBoxProps) -> Element {
    let dialogue_state = use_dialogue_state();
    let current_mood = dialogue_state.current_mood.read().clone();
    let current_action = dialogue_state.current_action.read().clone();
    
    rsx! {
        div {
            class: "vn-dialogue-box",
            
            // Speaker nameplate with mood
            if !props.speaker_name.is_empty() {
                div {
                    class: "vn-speaker-header",
                    span { class: "vn-character-name", "{props.speaker_name}" }
                    if let Some(mood) = &current_mood {
                        span { class: "vn-mood-tag", "*{mood}*" }
                    }
                }
            }
            
            // Dialogue text
            div {
                class: "vn-dialogue-text",
                
                // Show action inline if active
                if let Some(action) = &current_action {
                    span { class: "vn-action-marker", "*{action}* " }
                }
                
                "{dialogue_state.displayed_text}"
                
                // Blinking cursor during typing
                if *dialogue_state.is_typing.read() {
                    span { class: "vn-cursor", "▌" }
                }
            }
            
            // Choices (after typing complete)
            // ...
        }
    }
}
```

### Player Input with Marker Support

```rust
// crates/player-ui/src/presentation/components/visual_novel/choice_menu.rs

#[component]
pub fn PlayerInput(props: PlayerInputProps) -> Element {
    let mut input_text = use_signal(String::new);
    let mut validation_warning = use_signal::<Option<String>>(|| None);
    
    let on_submit = move |_| {
        let text = input_text.read().clone();
        if text.is_empty() { return; }
        
        // Validate markers
        let validation = validate_player_markers(
            &text,
            &props.available_expressions,
            &props.available_actions,
        );
        
        if let Some(warning) = validation.warning {
            validation_warning.set(Some(warning));
        }
        
        // Submit even with warnings (fallback will be used)
        props.on_submit.call(text);
        input_text.set(String::new());
        validation_warning.set(None);
    };
    
    rsx! {
        div {
            class: "player-input-container",
            
            // Warning message
            if let Some(warning) = validation_warning.read().as_ref() {
                div { class: "input-warning", "{warning}" }
            }
            
            // Input field with marker hint
            input {
                r#type: "text",
                value: "{input_text}",
                oninput: move |e| input_text.set(e.value()),
                onkeypress: move |e| if e.key() == Key::Enter { on_submit(()) },
                placeholder: "Type your response... (use *mood* for expressions)",
            }
            
            button {
                onclick: on_submit,
                "Send"
            }
        }
    }
}

fn validate_player_markers(
    text: &str,
    expressions: &[String],
    actions: &[String],
) -> MarkerValidation {
    let parsed = ParsedDialogue::parse(text);
    let mut warnings = Vec::new();
    
    for segment in &parsed.segments {
        if let Some(DialogueMarker::Mood { mood, expression }) = &segment.marker {
            let target_expr = expression.as_ref().unwrap_or(mood);
            if !expressions.contains(target_expr) && !expressions.is_empty() {
                warnings.push(format!(
                    "Expression '{}' not available, will use default",
                    target_expr
                ));
            }
        }
    }
    
    MarkerValidation {
        warning: if warnings.is_empty() { None } else { Some(warnings.join("; ")) },
    }
}
```

---

## DM Approval UI Updates

### Dialogue Approval with Marker Highlighting

```rust
// In DM approval panel

fn render_dialogue_preview(text: &str) -> Element {
    let parsed = ParsedDialogue::parse(text);
    
    rsx! {
        div {
            class: "dialogue-preview",
            for segment in parsed.segments {
                // Highlight markers
                if let Some(marker) = &segment.marker {
                    span {
                        class: "marker-highlight",
                        match marker {
                            DialogueMarker::Mood { mood, expression } => {
                                if let Some(expr) = expression {
                                    rsx! { "*{mood}|{expr}*" }
                                } else {
                                    rsx! { "*{mood}*" }
                                }
                            }
                            DialogueMarker::Action(action) => {
                                rsx! { "*{action}*" }
                            }
                        }
                    }
                }
                // Regular text
                span { "{segment.text}" }
            }
        }
    }
}
```

### Marker Editor

Allow DM to edit markers before approval:
- Click marker to edit/remove
- Insert new marker at cursor position
- Dropdown of available expressions

---

## Character Editor Updates

### Expression Configuration UI

Add to character creation/editing form:

```rust
#[component]
pub fn ExpressionConfigEditor(props: ExpressionConfigProps) -> Element {
    rsx! {
        div {
            class: "expression-config-section",
            
            h3 { "Expression Configuration" }
            
            // Available expressions (multi-select or tag input)
            div {
                class: "form-field",
                label { "Available Expressions" }
                TagInput {
                    values: props.expressions.clone(),
                    suggestions: vec!["neutral", "happy", "sad", "angry", "surprised", "thoughtful", "suspicious", "afraid"],
                    on_change: move |v| props.on_expressions_change.call(v),
                    placeholder: "Add expression...",
                }
                p { class: "help-text", "Expressions that have sprite assets (e.g., character_happy.png)" }
            }
            
            // Custom actions
            div {
                class: "form-field",
                label { "Custom Actions" }
                TagInput {
                    values: props.actions.clone(),
                    on_change: move |v| props.on_actions_change.call(v),
                    placeholder: "Add action...",
                }
                p { class: "help-text", "Physical actions this character uses (e.g., sighs, laughs nervously)" }
            }
            
            // Default expression
            div {
                class: "form-field",
                label { "Default Expression" }
                select {
                    value: "{props.default_expression}",
                    onchange: move |e| props.on_default_change.call(e.value()),
                    for expr in &props.expressions {
                        option { value: "{expr}", "{expr}" }
                    }
                }
            }
        }
    }
}
```

---

## Mood History Tracking

### Service Layer

```rust
// crates/engine-app/src/application/services/mood_history_service.rs (NEW)

pub trait MoodHistoryService: Send + Sync {
    async fn record_mood(
        &self,
        session_id: SessionId,
        character_id: CharacterId,
        character_name: &str,
        mood: &str,
        expression: Option<&str>,
        dialogue_preview: &str,
    ) -> Result<()>;
    
    async fn get_history(&self, session_id: SessionId) -> Result<MoodHistory>;
    
    async fn get_character_history(
        &self,
        session_id: SessionId,
        character_id: CharacterId,
    ) -> Result<MoodHistory>;
}
```

### Integration Point

Record mood history when:
1. Dialogue with mood marker is approved and broadcast
2. Player submits text with mood marker
3. Mood change tool call is approved

---

## Implementation Phases

### Phase 1: Core Parser & Types (3 hours)
- [ ] Create `dialogue_markers.rs` with `ParsedDialogue`, `DialogueMarker`, etc.
- [ ] Implement marker regex parsing
- [ ] Add `ExpressionConfig` to Character and PlayerCharacter entities
- [ ] Add `MoodHistory` types
- [ ] Unit tests for parser

**Files**:
- NEW: `crates/domain/src/value_objects/dialogue_markers.rs`
- NEW: `crates/domain/src/value_objects/mood_history.rs`
- MOD: `crates/domain/src/entities/character.rs`
- MOD: `crates/domain/src/entities/player_character.rs`
- MOD: `crates/domain/src/value_objects/mod.rs`

### Phase 2: Protocol & Persistence (2 hours)
- [ ] Update `CharacterData` with expression fields
- [ ] Add expression config to character repository
- [ ] Add mood history messages
- [ ] Update character routes to include expression config

**Files**:
- MOD: `crates/protocol/src/messages.rs`
- MOD: `crates/engine-adapters/src/infrastructure/persistence/character_repository.rs`
- MOD: `crates/engine-adapters/src/infrastructure/http/character_routes.rs`

### Phase 3: LLM Prompt Updates (1.5 hours)
- [ ] Update `DIALOGUE_RESPONSE_FORMAT` with marker instructions
- [ ] Inject available expressions into prompt
- [ ] Add `change_mood` tool definition
- [ ] Parse `change_mood` tool calls

**Files**:
- MOD: `crates/domain/src/value_objects/prompt_templates.rs`
- MOD: `crates/engine-app/src/application/services/llm/prompt_builder.rs`
- MOD: `crates/engine-app/src/application/services/llm/mod.rs`

### Phase 4: Typewriter Expression Integration (3 hours)
- [ ] Update `DialogueState` with expression signals
- [ ] Implement `use_typewriter_effect_with_expressions`
- [ ] Update `apply_dialogue` to parse markers
- [ ] Handle expression changes during playback

**Files**:
- MOD: `crates/player-ui/src/presentation/state/dialogue_state.rs`

### Phase 5: UI Component Updates (3 hours)
- [ ] Update `CharacterSprite` with expression sprite URL logic
- [ ] Add mood badge to sprite
- [ ] Update `DialogueBox` with mood tag and action display
- [ ] Add expression fallback CSS

**Files**:
- MOD: `crates/player-ui/src/presentation/components/visual_novel/character_sprite.rs`
- MOD: `crates/player-ui/src/presentation/components/visual_novel/dialogue_box.rs`
- MOD: `crates/player-ui/styles/input.css`

### Phase 6: Player Input Validation (1.5 hours)
- [ ] Add marker validation to player input
- [ ] Show validation warnings
- [ ] Pass available expressions to input component

**Files**:
- MOD: `crates/player-ui/src/presentation/components/visual_novel/choice_menu.rs`

### Phase 7: Character Editor Updates (2 hours)
- [ ] Add `ExpressionConfigEditor` component
- [ ] Wire into character creation/editing form
- [ ] Add to PC creation flow

**Files**:
- NEW: `crates/player-ui/src/presentation/components/creator/expression_config_editor.rs`
- MOD: `crates/player-ui/src/presentation/components/creator/character_form.rs`
- MOD: `crates/player-ui/src/presentation/views/pc_creation.rs`

### Phase 8: DM Approval Updates (2 hours)
- [ ] Highlight markers in approval preview
- [ ] Add marker editing capability
- [ ] Handle `change_mood` tool approval
- [ ] Show expression preview

**Files**:
- MOD: `crates/player-ui/src/presentation/components/dm_panel/dm_approval_panel.rs`

### Phase 9: Mood History Service (2 hours)
- [ ] Create `MoodHistoryService`
- [ ] Integrate with dialogue broadcast
- [ ] Add history view to DM panel

**Files**:
- NEW: `crates/engine-app/src/application/services/mood_history_service.rs`
- MOD: `crates/engine-app/src/application/services/mod.rs`
- MOD: `crates/engine-adapters/src/infrastructure/websocket.rs`

### Phase 10: Testing & Polish (2 hours)
- [ ] End-to-end testing with LLM
- [ ] Test PC mood markers
- [ ] Test expression fallback
- [ ] Fix styling issues
- [ ] Documentation updates

---

## Verification Commands

```bash
# After each phase
nix-shell /home/otto/repos/WrldBldr/Game/shell.nix --run "cd /home/otto/repos/WrldBldr/Game && cargo check --workspace"
nix-shell /home/otto/repos/WrldBldr/Game/shell.nix --run "cd /home/otto/repos/WrldBldr/Game && cargo xtask arch-check"

# Run parser tests
nix-shell /home/otto/repos/WrldBldr/Game/shell.nix --run "cd /home/otto/repos/WrldBldr/Game && cargo test -p wrldbldr-domain dialogue_markers"
```

---

## Success Criteria

- [ ] LLM generates dialogue with mood markers
- [ ] Typewriter effect changes expressions at marker positions
- [ ] Character sprites switch to expression-specific assets
- [ ] Mood badge appears on sprite and in dialogue box
- [ ] Actions display inline during typewriter
- [ ] Players can add mood markers to their input
- [ ] Markers are validated with fallback
- [ ] DM can edit markers in approval
- [ ] `change_mood` tool call updates permanent mood state
- [ ] Mood history is tracked per conversation
- [ ] Expression config is editable per character

---

## CSS Additions

```css
/* Mood badge on character sprite */
.mood-badge {
    position: absolute;
    top: 0.5rem;
    right: 0.5rem;
    background: rgba(0, 0, 0, 0.7);
    color: var(--accent-gold);
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    font-size: 0.75rem;
    font-style: italic;
}

/* Mood tag in speaker header */
.vn-mood-tag {
    color: var(--accent-gold);
    font-style: italic;
    margin-left: 0.5rem;
    font-size: 0.9em;
}

/* Action marker in dialogue */
.vn-action-marker {
    color: var(--muted-text);
    font-style: italic;
}

/* Marker highlight in DM approval */
.marker-highlight {
    background: rgba(212, 175, 55, 0.2);
    color: var(--accent-gold);
    padding: 0 0.25rem;
    border-radius: 0.125rem;
}

/* Input warning */
.input-warning {
    color: var(--warning-orange);
    font-size: 0.8rem;
    margin-bottom: 0.5rem;
}
```

---

## Rollback Plan

If issues arise:
1. Markers can be stripped from text (fallback to plain dialogue)
2. Expression config can default to empty (no expression changes)
3. Old mood system still works for permanent state

---

## Future Enhancements

- **Sound effects**: Action markers could trigger sounds
- **Animations**: Expression transitions with CSS animations
- **Voice synthesis**: Mood influences voice tone
- **AI expression detection**: Auto-suggest markers from plain text
- **Expression sheet support**: Grid-based sprites instead of individual files
