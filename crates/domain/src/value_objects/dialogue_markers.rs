//! Dialogue marker parsing for expression changes
//!
//! Part of the three-tier emotional model (Tier 3: Expression).
//! This module parses inline markers like `*happy*` or `*sighs|thoughtful*`
//! from dialogue text to trigger sprite expression changes.
//!
//! # Marker Formats
//!
//! - `*expression*` - Change to expression (e.g., `*happy*`)
//! - `*action|expression*` - Perform action then change expression (e.g., `*sighs|sad*`)
//! - `*action*` - Perform action, keep current expression (e.g., `*nods*`)
//!
//! # Example
//!
//! ```
//! use wrldbldr_domain::{DialogueMarker, parse_dialogue_markers};
//!
//! let text = "*curious* \"You seek the Heartstone?\" *suspicious* \"But why?\"";
//! let markers = parse_dialogue_markers(text);
//!
//! assert_eq!(markers.len(), 2);
//! assert_eq!(markers[0].expression, Some("curious".to_string()));
//! assert_eq!(markers[1].expression, Some("suspicious".to_string()));
//! ```

use serde::{Deserialize, Serialize};

/// A parsed dialogue marker representing an expression change or action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogueMarker {
    /// The action to perform (displayed as stage direction)
    /// e.g., "sighs", "nods", "shakes head"
    pub action: Option<String>,

    /// The expression to change to
    /// e.g., "happy", "sad", "suspicious"
    pub expression: Option<String>,

    /// Character offset where this marker starts in the original text
    pub start_offset: usize,

    /// Character offset where this marker ends in the original text
    pub end_offset: usize,

    /// The raw marker text including asterisks
    pub raw: String,
}

impl DialogueMarker {
    /// Create a new marker with just an expression
    pub fn expression(
        expr: impl Into<String>,
        start: usize,
        end: usize,
        raw: impl Into<String>,
    ) -> Self {
        Self {
            action: None,
            expression: Some(expr.into()),
            start_offset: start,
            end_offset: end,
            raw: raw.into(),
        }
    }

    /// Create a new marker with just an action
    pub fn action(
        act: impl Into<String>,
        start: usize,
        end: usize,
        raw: impl Into<String>,
    ) -> Self {
        Self {
            action: Some(act.into()),
            expression: None,
            start_offset: start,
            end_offset: end,
            raw: raw.into(),
        }
    }

    /// Create a new marker with both action and expression
    pub fn action_and_expression(
        act: impl Into<String>,
        expr: impl Into<String>,
        start: usize,
        end: usize,
        raw: impl Into<String>,
    ) -> Self {
        Self {
            action: Some(act.into()),
            expression: Some(expr.into()),
            start_offset: start,
            end_offset: end,
            raw: raw.into(),
        }
    }

    /// Check if this marker has an expression change
    pub fn has_expression(&self) -> bool {
        self.expression.is_some()
    }

    /// Check if this marker has an action
    pub fn has_action(&self) -> bool {
        self.action.is_some()
    }

    /// Get the marker length in characters
    pub fn len(&self) -> usize {
        self.end_offset - self.start_offset
    }

    /// Check if marker is empty (shouldn't happen in valid parsing)
    pub fn is_empty(&self) -> bool {
        self.action.is_none() && self.expression.is_none()
    }
}

/// Result of parsing dialogue text for markers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDialogue {
    /// The original text with markers
    pub original: String,

    /// Text with markers removed (for display)
    pub clean_text: String,

    /// Extracted markers with their positions
    pub markers: Vec<DialogueMarker>,
}

impl ParsedDialogue {
    /// Check if there are any markers
    pub fn has_markers(&self) -> bool {
        !self.markers.is_empty()
    }

    /// Get markers that have expression changes
    pub fn expression_markers(&self) -> Vec<&DialogueMarker> {
        self.markers.iter().filter(|m| m.has_expression()).collect()
    }

    /// Get markers that have actions
    pub fn action_markers(&self) -> Vec<&DialogueMarker> {
        self.markers.iter().filter(|m| m.has_action()).collect()
    }
}

/// Parse dialogue text and extract all markers
///
/// Markers are in the format `*content*` where content can be:
/// - `expression` - Just an expression name
/// - `action|expression` - Action followed by expression
/// - `action` - Just an action (if not recognized as expression)
///
/// # Arguments
/// * `text` - The dialogue text containing markers
///
/// # Returns
/// A list of parsed markers with their positions
pub fn parse_dialogue_markers(text: &str) -> Vec<DialogueMarker> {
    let mut markers = Vec::new();
    let mut chars = text.char_indices().peekable();

    while let Some((start_idx, ch)) = chars.next() {
        if ch == '*' {
            // Look for closing asterisk
            let mut content = String::new();
            let mut found_close = false;

            for (_, inner_ch) in chars.by_ref() {
                if inner_ch == '*' {
                    found_close = true;
                    break;
                }
                // Don't allow newlines in markers
                if inner_ch == '\n' {
                    break;
                }
                content.push(inner_ch);
            }

            if found_close && !content.is_empty() {
                let end_idx = start_idx + 2 + content.len(); // *content*
                let raw = format!("*{}*", content);
                let marker = parse_marker_content(&content, start_idx, end_idx, &raw);
                markers.push(marker);
            }
        }
    }

    markers
}

/// Parse the full dialogue and return both clean text and markers
///
/// This removes marker text from the dialogue while preserving marker positions
/// relative to the original text.
pub fn parse_dialogue(text: &str) -> ParsedDialogue {
    let markers = parse_dialogue_markers(text);

    // Build clean text by removing markers
    let mut clean = String::with_capacity(text.len());
    let mut last_end = 0;

    for marker in &markers {
        // Add text between markers
        if marker.start_offset > last_end {
            clean.push_str(&text[last_end..marker.start_offset]);
        }
        last_end = marker.end_offset;
    }

    // Add remaining text after last marker
    if last_end < text.len() {
        clean.push_str(&text[last_end..]);
    }

    // Trim extra whitespace that might result from marker removal
    let clean_text = clean.split_whitespace().collect::<Vec<_>>().join(" ");

    ParsedDialogue {
        original: text.to_string(),
        clean_text,
        markers,
    }
}

/// Parse the content of a marker (text between asterisks)
fn parse_marker_content(content: &str, start: usize, end: usize, raw: &str) -> DialogueMarker {
    let content = content.trim();

    // Check for action|expression format
    if let Some(pipe_idx) = content.find('|') {
        let action = content[..pipe_idx].trim();
        let expression = content[pipe_idx + 1..].trim();

        if !action.is_empty() && !expression.is_empty() {
            return DialogueMarker::action_and_expression(action, expression, start, end, raw);
        } else if !action.is_empty() {
            return DialogueMarker::action(action, start, end, raw);
        } else if !expression.is_empty() {
            return DialogueMarker::expression(expression, start, end, raw);
        }
    }

    // Single word - could be expression or action
    // We'll treat it as expression by default; the renderer can check
    // against ExpressionConfig to determine if it's an action
    DialogueMarker::expression(content, start, end, raw)
}

/// Validate markers against a character's expression config
///
/// Returns a list of warnings for unknown expressions/actions
pub fn validate_markers(
    markers: &[DialogueMarker],
    expressions: &[String],
    actions: &[String],
) -> Vec<String> {
    let mut warnings = Vec::new();

    for marker in markers {
        if let Some(ref expr) = marker.expression {
            let found = expressions.iter().any(|e| e.eq_ignore_ascii_case(expr));
            if !found {
                warnings.push(format!("Unknown expression: '{}'", expr));
            }
        }

        if let Some(ref act) = marker.action {
            let found = actions.iter().any(|a| a.eq_ignore_ascii_case(act));
            if !found {
                warnings.push(format!("Unknown action: '{}'", act));
            }
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_expression() {
        let markers = parse_dialogue_markers("*happy* Hello there!");
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].expression, Some("happy".to_string()));
        assert!(markers[0].action.is_none());
        assert_eq!(markers[0].start_offset, 0);
    }

    #[test]
    fn test_parse_multiple_expressions() {
        let text = "*curious* \"You seek the Heartstone?\" *suspicious* \"But why?\"";
        let markers = parse_dialogue_markers(text);

        assert_eq!(markers.len(), 2);
        assert_eq!(markers[0].expression, Some("curious".to_string()));
        assert_eq!(markers[1].expression, Some("suspicious".to_string()));
    }

    #[test]
    fn test_parse_action_and_expression() {
        let markers = parse_dialogue_markers("*sighs|sad* I suppose so...");

        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].action, Some("sighs".to_string()));
        assert_eq!(markers[0].expression, Some("sad".to_string()));
    }

    #[test]
    fn test_parse_dialogue_clean_text() {
        let text = "*happy* Hello there! *curious* How are you?";
        let parsed = parse_dialogue(text);

        assert_eq!(parsed.clean_text, "Hello there! How are you?");
        assert_eq!(parsed.markers.len(), 2);
    }

    #[test]
    fn test_unclosed_marker_ignored() {
        let markers = parse_dialogue_markers("*incomplete marker without close");
        assert!(markers.is_empty());
    }

    #[test]
    fn test_empty_marker_ignored() {
        let markers = parse_dialogue_markers("** empty markers ** are ignored");
        assert!(markers.is_empty());
    }

    #[test]
    fn test_marker_with_spaces() {
        let markers = parse_dialogue_markers("*shakes head* No way!");

        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].expression, Some("shakes head".to_string()));
    }

    #[test]
    fn test_validate_markers() {
        let markers = vec![
            DialogueMarker::expression("happy", 0, 7, "*happy*"),
            DialogueMarker::expression("unknown_expr", 10, 25, "*unknown_expr*"),
            DialogueMarker::action("sighs", 30, 38, "*sighs*"),
        ];

        let expressions = vec!["happy".to_string(), "sad".to_string()];
        let actions = vec!["sighs".to_string()];

        let warnings = validate_markers(&markers, &expressions, &actions);

        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("unknown_expr"));
    }

    #[test]
    fn test_pipe_only_expression() {
        let markers = parse_dialogue_markers("*|happy*");
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].expression, Some("happy".to_string()));
        assert!(markers[0].action.is_none());
    }

    #[test]
    fn test_pipe_only_action() {
        let markers = parse_dialogue_markers("*sighs|*");
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].action, Some("sighs".to_string()));
        assert!(markers[0].expression.is_none());
    }

    #[test]
    fn test_newline_breaks_marker() {
        let markers = parse_dialogue_markers("*incomplete\nmarker*");
        assert!(markers.is_empty());
    }

    #[test]
    fn test_expression_markers_filter() {
        let parsed = parse_dialogue("*nods* *happy* Hello *sighs|sad*");
        let expr_markers = parsed.expression_markers();

        // All three have expressions (nods treated as expression, happy is expression, sad is expression)
        assert_eq!(expr_markers.len(), 3);
    }

    #[test]
    fn test_marker_len() {
        let marker = DialogueMarker::expression("happy", 0, 7, "*happy*");
        assert_eq!(marker.len(), 7);
    }
}
