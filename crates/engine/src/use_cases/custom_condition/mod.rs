//! Custom condition evaluation via LLM.
//!
//! Evaluates custom conditions and triggers that require LLM interpretation.
//! Used for `SceneCondition::Custom` and `NarrativeTriggerType::Custom`.
//!
//! See `docs/designs/LLM_RESILIENCE_AND_CUSTOM_EVALUATION.md` for design details.

use std::sync::Arc;

use crate::infrastructure::ports::{ChatMessage, LlmError, LlmPort, LlmRequest};

/// Result of evaluating a custom condition.
#[derive(Debug, Clone)]
pub struct ConditionEvaluationResult {
    /// Whether the condition is met
    pub result: bool,
    /// LLM's confidence in the result (0.0-1.0)
    pub confidence: f32,
    /// LLM's reasoning (for DM review/debugging)
    pub reasoning: String,
}

/// Context for evaluating a custom condition.
///
/// Provides game state information to help the LLM evaluate the condition.
#[derive(Debug, Clone, Default)]
pub struct EvaluationContext {
    /// Current time of day in the game
    pub time_of_day: Option<String>,
    /// Description of the current location/region
    pub location_description: Option<String>,
    /// NPCs present in the current scene
    pub npcs_present: Vec<String>,
    /// Items the player character has
    pub inventory: Vec<String>,
    /// Characters the player knows
    pub known_characters: Vec<String>,
    /// Flags that are currently set
    pub flags: Vec<String>,
    /// Recent narrative events or story context
    pub recent_events: Vec<String>,
    /// Any additional context the DM provided
    pub additional_context: Option<String>,
}

impl EvaluationContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_time_of_day(mut self, time: impl Into<String>) -> Self {
        self.time_of_day = Some(time.into());
        self
    }

    pub fn with_location(mut self, description: impl Into<String>) -> Self {
        self.location_description = Some(description.into());
        self
    }

    pub fn with_npcs(mut self, npcs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.npcs_present = npcs.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_inventory(mut self, items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.inventory = items.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_known_characters(
        mut self,
        characters: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.known_characters = characters.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_flags(mut self, flags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.flags = flags.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_recent_events(
        mut self,
        events: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.recent_events = events.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_additional_context(mut self, context: impl Into<String>) -> Self {
        self.additional_context = Some(context.into());
        self
    }

    /// Format the context as a string for the LLM prompt.
    fn format_for_prompt(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref time) = self.time_of_day {
            parts.push(format!("Time of Day: {}", time));
        }

        if let Some(ref location) = self.location_description {
            parts.push(format!("Current Location: {}", location));
        }

        if !self.npcs_present.is_empty() {
            parts.push(format!("NPCs Present: {}", self.npcs_present.join(", ")));
        }

        if !self.inventory.is_empty() {
            parts.push(format!("Player Inventory: {}", self.inventory.join(", ")));
        }

        if !self.known_characters.is_empty() {
            parts.push(format!(
                "Known Characters: {}",
                self.known_characters.join(", ")
            ));
        }

        if !self.flags.is_empty() {
            parts.push(format!("Active Flags: {}", self.flags.join(", ")));
        }

        if !self.recent_events.is_empty() {
            parts.push(format!("Recent Events:\n- {}", self.recent_events.join("\n- ")));
        }

        if let Some(ref additional) = self.additional_context {
            parts.push(format!("Additional Context: {}", additional));
        }

        if parts.is_empty() {
            "No additional context available.".to_string()
        } else {
            parts.join("\n\n")
        }
    }
}

/// Evaluates custom conditions using the LLM.
///
/// This use case wraps the LLM port and provides a structured way to evaluate
/// natural language conditions against the current game state.
pub struct CustomConditionEvaluator {
    llm: Arc<dyn LlmPort>,
    /// Minimum confidence threshold to consider condition met (default: 0.7)
    confidence_threshold: f32,
}

impl CustomConditionEvaluator {
    pub fn new(llm: Arc<dyn LlmPort>) -> Self {
        Self {
            llm,
            confidence_threshold: 0.7,
        }
    }

    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Evaluate a custom condition description against the current game context.
    ///
    /// # Arguments
    /// * `condition_description` - Natural language description of the condition
    /// * `context` - Current game state context
    ///
    /// # Returns
    /// * `Ok(ConditionEvaluationResult)` - Evaluation result with confidence and reasoning
    /// * `Err(LlmError)` - If LLM call fails
    pub async fn evaluate(
        &self,
        condition_description: &str,
        context: &EvaluationContext,
    ) -> Result<ConditionEvaluationResult, LlmError> {
        let system_prompt = self.build_system_prompt();
        let user_message = self.build_user_message(condition_description, context);

        let request = LlmRequest::new(vec![ChatMessage::user(user_message)])
            .with_system_prompt(system_prompt)
            .with_temperature(0.1); // Low temperature for more deterministic evaluation

        tracing::debug!(
            condition = %condition_description,
            "Evaluating custom condition via LLM"
        );

        let response = self.llm.generate(request).await?;

        // Parse the response
        let result = self.parse_response(&response.content, condition_description)?;

        tracing::info!(
            condition = %condition_description,
            result = %result.result,
            confidence = %result.confidence,
            "Custom condition evaluated"
        );

        Ok(result)
    }

    /// Check if a condition is met based on evaluation result and confidence threshold.
    ///
    /// Returns true only if the LLM says the condition is met AND confidence >= threshold.
    pub fn is_condition_met(&self, result: &ConditionEvaluationResult) -> bool {
        result.result && result.confidence >= self.confidence_threshold
    }

    fn build_system_prompt(&self) -> String {
        format!(
            r#"You are a game master assistant evaluating whether a condition is currently met in a tabletop RPG game.

Your task is to analyze a condition description and the current game state, then determine if the condition is TRUE or FALSE.

IMPORTANT RULES:
1. Be conservative - if there's not enough information to determine the condition, lean toward FALSE
2. Consider only the information provided - don't assume facts not in the context
3. Provide a confidence score from 0.0 to 1.0:
   - 0.9-1.0: Absolutely certain based on explicit evidence
   - 0.7-0.9: Confident based on strong implications
   - 0.5-0.7: Somewhat confident but some ambiguity
   - Below 0.5: Uncertain or insufficient information
4. The condition must meet a confidence threshold of {} to be considered met

You MUST respond in EXACTLY this JSON format:
```json
{{
  "result": true or false,
  "confidence": 0.0 to 1.0,
  "reasoning": "Your explanation here"
}}
```

Do not include any text outside the JSON block."#,
            self.confidence_threshold
        )
    }

    fn build_user_message(&self, condition_description: &str, context: &EvaluationContext) -> String {
        format!(
            r#"## Condition to Evaluate
{}

## Current Game State
{}

Please evaluate whether this condition is currently met and respond with the JSON format specified."#,
            condition_description,
            context.format_for_prompt()
        )
    }

    fn parse_response(
        &self,
        response: &str,
        condition_description: &str,
    ) -> Result<ConditionEvaluationResult, LlmError> {
        // Try to extract JSON from the response
        let json_str = self.extract_json(response);

        // Parse the JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
            tracing::warn!(
                error = %e,
                response = %response,
                "Failed to parse LLM response as JSON"
            );
            LlmError::InvalidResponse(format!("Invalid JSON in response: {}", e))
        })?;

        // Extract fields
        let result = parsed["result"].as_bool().ok_or_else(|| {
            LlmError::InvalidResponse("Missing 'result' field in response".to_string())
        })?;

        let confidence = parsed["confidence"]
            .as_f64()
            .map(|f| f as f32)
            .unwrap_or(0.5);

        let reasoning = parsed["reasoning"]
            .as_str()
            .unwrap_or("No reasoning provided")
            .to_string();

        tracing::debug!(
            condition = %condition_description,
            result = %result,
            confidence = %confidence,
            reasoning = %reasoning,
            "Parsed custom condition evaluation"
        );

        Ok(ConditionEvaluationResult {
            result,
            confidence: confidence.clamp(0.0, 1.0),
            reasoning,
        })
    }

    /// Extract JSON from a response that might have markdown code blocks or extra text.
    fn extract_json(&self, response: &str) -> String {
        // Try to find JSON in markdown code block
        if let Some(start) = response.find("```json") {
            if let Some(end) = response[start + 7..].find("```") {
                return response[start + 7..start + 7 + end].trim().to_string();
            }
        }

        // Try to find JSON in plain code block
        if let Some(start) = response.find("```") {
            if let Some(end) = response[start + 3..].find("```") {
                let content = response[start + 3..start + 3 + end].trim();
                // Skip language identifier if present
                if let Some(newline_pos) = content.find('\n') {
                    let first_line = &content[..newline_pos];
                    if !first_line.starts_with('{') {
                        return content[newline_pos + 1..].trim().to_string();
                    }
                }
                return content.to_string();
            }
        }

        // Try to find raw JSON object
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                return response[start..=end].to_string();
            }
        }

        // Return as-is if no JSON found
        response.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::infrastructure::ports::{FinishReason, LlmResponse, ToolDefinition};

    /// Mock LLM that returns a configurable response
    struct MockLlm {
        response: String,
    }

    impl MockLlm {
        fn new(response: impl Into<String>) -> Self {
            Self {
                response: response.into(),
            }
        }
    }

    #[async_trait]
    impl LlmPort for MockLlm {
        async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
            Ok(LlmResponse {
                content: self.response.clone(),
                tool_calls: vec![],
                finish_reason: FinishReason::Stop,
                usage: None,
            })
        }

        async fn generate_with_tools(
            &self,
            request: LlmRequest,
            _tools: Vec<ToolDefinition>,
        ) -> Result<LlmResponse, LlmError> {
            self.generate(request).await
        }
    }

    #[tokio::test]
    async fn test_evaluate_condition_true() {
        let mock = Arc::new(MockLlm::new(
            r#"```json
{
  "result": true,
  "confidence": 0.9,
  "reasoning": "The player has the key item and knows the guard."
}
```"#,
        ));

        let evaluator = CustomConditionEvaluator::new(mock);
        let context = EvaluationContext::new()
            .with_inventory(["Ancient Key"])
            .with_known_characters(["Guard Captain"]);

        let result = evaluator
            .evaluate("The player has found the ancient key and befriended the guard", &context)
            .await
            .unwrap();

        assert!(result.result);
        assert!(result.confidence > 0.8);
        assert!(evaluator.is_condition_met(&result));
    }

    #[tokio::test]
    async fn test_evaluate_condition_false() {
        let mock = Arc::new(MockLlm::new(
            r#"{"result": false, "confidence": 0.85, "reasoning": "The player does not have the required item."}"#,
        ));

        let evaluator = CustomConditionEvaluator::new(mock);
        let context = EvaluationContext::new();

        let result = evaluator
            .evaluate("The player has completed the secret quest", &context)
            .await
            .unwrap();

        assert!(!result.result);
        assert!(!evaluator.is_condition_met(&result));
    }

    #[tokio::test]
    async fn test_low_confidence_fails_threshold() {
        let mock = Arc::new(MockLlm::new(
            r#"{"result": true, "confidence": 0.5, "reasoning": "Uncertain based on available information."}"#,
        ));

        let evaluator = CustomConditionEvaluator::new(mock);
        let result = evaluator
            .evaluate("The player is trusted by the guild", &EvaluationContext::new())
            .await
            .unwrap();

        assert!(result.result); // LLM said true
        assert!(!evaluator.is_condition_met(&result)); // But confidence too low
    }

    #[tokio::test]
    async fn test_custom_confidence_threshold() {
        let mock = Arc::new(MockLlm::new(
            r#"{"result": true, "confidence": 0.6, "reasoning": "Some evidence supports this."}"#,
        ));

        let evaluator = CustomConditionEvaluator::new(mock).with_confidence_threshold(0.5);
        let result = evaluator
            .evaluate("The player has earned respect", &EvaluationContext::new())
            .await
            .unwrap();

        assert!(evaluator.is_condition_met(&result)); // Passes lower threshold
    }

    #[test]
    fn test_extract_json_from_markdown() {
        let evaluator = CustomConditionEvaluator::new(Arc::new(MockLlm::new("")));

        let response = r#"Here is my evaluation:
```json
{"result": true, "confidence": 0.8, "reasoning": "Test"}
```
That's my answer."#;

        let json = evaluator.extract_json(response);
        assert!(json.starts_with('{'));
        assert!(json.contains("\"result\": true"));
    }

    #[test]
    fn test_extract_raw_json() {
        let evaluator = CustomConditionEvaluator::new(Arc::new(MockLlm::new("")));

        let response = r#"{"result": false, "confidence": 0.9, "reasoning": "No evidence"}"#;
        let json = evaluator.extract_json(response);
        assert_eq!(json, response);
    }

    #[test]
    fn test_context_format() {
        let context = EvaluationContext::new()
            .with_time_of_day("Evening")
            .with_location("A dimly lit tavern")
            .with_npcs(["Bartender", "Mysterious Stranger"])
            .with_flags(["quest_started", "met_informant"]);

        let formatted = context.format_for_prompt();
        assert!(formatted.contains("Evening"));
        assert!(formatted.contains("dimly lit tavern"));
        assert!(formatted.contains("Bartender"));
        assert!(formatted.contains("quest_started"));
    }
}
