//! Workflow configuration and analysis types for asset generation.

use serde::{Deserialize, Serialize};

/// Slots for different asset types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkflowSlot {
    /// Character portrait (256x256)
    CharacterPortrait,
    /// Character sprite (512x512)
    CharacterSprite,
    /// Character expression sheet (768x768 grid)
    CharacterExpressionSheet,
    /// Location backdrop (1920x1080)
    LocationBackdrop,
    /// Location tilesheet (512x512)
    LocationTilesheet,
    /// Location time variant (day/night/weather)
    LocationTimeVariant,
    /// Item icon (64x64)
    ItemIcon,
    /// Item set (multiple items in grid)
    ItemSet,
    /// Map region backdrop (1280x720)
    MapRegion,
    /// Unknown workflow slot (for forward compatibility)
    #[serde(other)]
    Unknown,
}

impl WorkflowSlot {
    /// Get default dimensions for this slot.
    pub fn default_dimensions(&self) -> (u32, u32) {
        match self {
            Self::CharacterPortrait => (256, 256),
            Self::CharacterSprite => (512, 512),
            Self::CharacterExpressionSheet => (768, 768),
            Self::LocationBackdrop => (1920, 1080),
            Self::LocationTilesheet => (512, 512),
            Self::LocationTimeVariant => (1920, 1080),
            Self::ItemIcon => (64, 64),
            Self::ItemSet => (256, 256),
            Self::MapRegion => (1280, 720),
            Self::Unknown => (256, 256),
        }
    }

    /// Get display name for this slot.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CharacterPortrait => "Character Portrait",
            Self::CharacterSprite => "Character Sprite",
            Self::CharacterExpressionSheet => "Expression Sheet",
            Self::LocationBackdrop => "Location Backdrop",
            Self::LocationTilesheet => "Tilesheet",
            Self::LocationTimeVariant => "Time Variant",
            Self::ItemIcon => "Item Icon",
            Self::ItemSet => "Item Set",
            Self::MapRegion => "Map Region",
            Self::Unknown => "Unknown",
        }
    }

    /// Get the category for grouping in UI.
    pub fn category(&self) -> &'static str {
        match self {
            Self::CharacterPortrait | Self::CharacterSprite | Self::CharacterExpressionSheet => {
                "Character Assets"
            }
            Self::LocationBackdrop | Self::LocationTilesheet | Self::LocationTimeVariant => {
                "Location Assets"
            }
            Self::ItemIcon | Self::ItemSet => "Item Assets",
            Self::MapRegion => "Map Assets",
            Self::Unknown => "Other",
        }
    }

    /// Get all slots.
    pub fn all() -> &'static [WorkflowSlot] {
        &[
            Self::CharacterPortrait,
            Self::CharacterSprite,
            Self::CharacterExpressionSheet,
            Self::LocationBackdrop,
            Self::LocationTilesheet,
            Self::LocationTimeVariant,
            Self::ItemIcon,
            Self::ItemSet,
            Self::MapRegion,
        ]
    }

    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CharacterPortrait => "character_portrait",
            Self::CharacterSprite => "character_sprite",
            Self::CharacterExpressionSheet => "character_expression_sheet",
            Self::LocationBackdrop => "location_backdrop",
            Self::LocationTilesheet => "location_tilesheet",
            Self::LocationTimeVariant => "location_time_variant",
            Self::ItemIcon => "item_icon",
            Self::ItemSet => "item_set",
            Self::MapRegion => "map_region",
            Self::Unknown => "unknown",
        }
    }
}

impl std::str::FromStr for WorkflowSlot {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "character_portrait" => Self::CharacterPortrait,
            "character_sprite" => Self::CharacterSprite,
            "character_expression_sheet" => Self::CharacterExpressionSheet,
            "location_backdrop" => Self::LocationBackdrop,
            "location_tilesheet" => Self::LocationTilesheet,
            "location_time_variant" => Self::LocationTimeVariant,
            "item_icon" => Self::ItemIcon,
            "item_set" => Self::ItemSet,
            "map_region" => Self::MapRegion,
            _ => Self::Unknown,
        })
    }
}

/// Type of prompt mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptMappingType {
    /// The main generation prompt.
    Primary,
    /// The negative prompt (things to avoid).
    Negative,
    /// Unknown mapping type (for forward compatibility).
    #[serde(other)]
    Unknown,
}

/// Mapping of a text input to prompt injection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptMapping {
    /// The node ID in the workflow.
    pub node_id: String,
    /// The input name on that node.
    pub input_name: String,
    /// What type of prompt this receives.
    pub mapping_type: PromptMappingType,
}

impl PromptMapping {
    pub fn primary(node_id: impl Into<String>, input_name: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            input_name: input_name.into(),
            mapping_type: PromptMappingType::Primary,
        }
    }

    pub fn negative(node_id: impl Into<String>, input_name: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            input_name: input_name.into(),
            mapping_type: PromptMappingType::Negative,
        }
    }
}

/// Default value for a workflow input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputDefault {
    /// The node ID in the workflow.
    pub node_id: String,
    /// The input name on that node.
    pub input_name: String,
    /// The default value.
    pub default_value: serde_json::Value,
}

impl InputDefault {
    pub fn new(
        node_id: impl Into<String>,
        input_name: impl Into<String>,
        value: serde_json::Value,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            input_name: input_name.into(),
            default_value: value,
        }
    }

    /// Get the path as "node_id.input_name".
    pub fn path(&self) -> String {
        format!("{}.{}", self.node_id, self.input_name)
    }
}

/// Detected input type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputType {
    /// Text/string input.
    Text,
    /// Integer number.
    Integer,
    /// Floating point number.
    Float,
    /// Boolean/checkbox.
    Boolean,
    /// Select from options.
    Select(Vec<String>),
    /// Unknown type (for forward compatibility).
    #[serde(other)]
    Unknown,
}

impl InputType {
    /// Detect the type from a JSON value.
    pub fn from_value(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::String(_) => Self::Text,
            serde_json::Value::Number(n) => {
                if n.is_i64() {
                    Self::Integer
                } else {
                    Self::Float
                }
            }
            serde_json::Value::Bool(_) => Self::Boolean,
            _ => Self::Unknown,
        }
    }
}

/// Parsed input from a workflow (for UI display).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    /// The node ID this input belongs to.
    pub node_id: String,
    /// The class type of the node (e.g., "KSampler", "CLIPTextEncode").
    pub node_type: String,
    /// The node's display title (from _meta).
    pub node_title: Option<String>,
    /// The input parameter name.
    pub input_name: String,
    /// Detected type of the input.
    pub input_type: InputType,
    /// Current value in the workflow.
    pub current_value: serde_json::Value,
}

impl WorkflowInput {
    /// Get the path as "node_id.input_name".
    pub fn path(&self) -> String {
        format!("{}.{}", self.node_id, self.input_name)
    }

    /// Get display name for this input.
    pub fn display_name(&self) -> String {
        if let Some(title) = &self.node_title {
            format!("{} → {}", title, self.input_name)
        } else {
            format!("{} → {}", self.node_type, self.input_name)
        }
    }
}

/// Result of analyzing a workflow JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAnalysis {
    /// Total number of nodes in the workflow.
    pub node_count: usize,
    /// All configurable (non-connection) inputs.
    pub inputs: Vec<WorkflowInput>,
    /// Text inputs that could be prompt fields.
    pub text_inputs: Vec<WorkflowInput>,
    /// Validation errors, if any.
    pub errors: Vec<String>,
}

impl WorkflowAnalysis {
    /// Check if the workflow is valid for use.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty() && self.node_count > 0
    }
}

// =============================================================================
// Pure workflow analysis functions
// =============================================================================

/// Analyze a ComfyUI API format workflow JSON.
///
/// Extracts all configurable inputs (non-connection values) from the workflow.
/// This is a pure function with no side effects.
pub fn analyze_workflow(workflow_json: &serde_json::Value) -> WorkflowAnalysis {
    let mut inputs = Vec::new();
    let mut text_inputs = Vec::new();
    let mut errors = Vec::new();
    let mut node_count = 0;

    // The workflow should be an object with node IDs as keys.
    let nodes = match workflow_json.as_object() {
        Some(nodes) => nodes,
        None => {
            errors.push("Workflow JSON must be an object with node IDs as keys".to_string());
            return WorkflowAnalysis {
                node_count: 0,
                inputs,
                text_inputs,
                errors,
            };
        }
    };

    for (node_id, node) in nodes {
        node_count += 1;

        // Get the class_type (node type).
        let class_type = node
            .get("class_type")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        // Get the node title from _meta if available.
        let node_title = node
            .get("_meta")
            .and_then(|m| m.get("title"))
            .and_then(|t| t.as_str())
            .map(String::from);

        // Get inputs.
        let node_inputs = match node.get("inputs").and_then(|v| v.as_object()) {
            Some(inputs) => inputs,
            None => continue, // Node has no inputs.
        };

        for (input_name, value) in node_inputs {
            // Skip connection inputs (arrays like ["node_id", output_index]).
            if value.is_array() {
                continue;
            }

            let input_type = InputType::from_value(value);

            let workflow_input = WorkflowInput {
                node_id: node_id.clone(),
                node_type: class_type.clone(),
                node_title: node_title.clone(),
                input_name: input_name.clone(),
                input_type: input_type.clone(),
                current_value: value.clone(),
            };

            // Track text inputs separately (potential prompt fields).
            if input_type == InputType::Text {
                text_inputs.push(workflow_input.clone());
            }

            inputs.push(workflow_input);
        }
    }

    // Sort inputs by node_id then input_name for consistent ordering.
    inputs.sort_by(|a, b| {
        a.node_id
            .cmp(&b.node_id)
            .then(a.input_name.cmp(&b.input_name))
    });

    text_inputs.sort_by(|a, b| {
        a.node_id
            .cmp(&b.node_id)
            .then(a.input_name.cmp(&b.input_name))
    });

    WorkflowAnalysis {
        node_count,
        inputs,
        text_inputs,
        errors,
    }
}

/// Validate a workflow JSON is in ComfyUI API format.
///
/// Returns Ok(()) if valid, or Err with a descriptive message if invalid.
/// This is a pure function with no side effects.
pub fn validate_workflow(workflow_json: &serde_json::Value) -> Result<(), String> {
    let nodes = workflow_json
        .as_object()
        .ok_or_else(|| "Workflow must be a JSON object".to_string())?;

    if nodes.is_empty() {
        return Err("Workflow has no nodes".to_string());
    }

    // Check that at least some nodes have the expected structure.
    let mut valid_nodes = 0;
    for (_node_id, node) in nodes {
        if !node.is_object() {
            continue;
        }

        // Check for class_type (required in API format).
        if node.get("class_type").is_some() {
            valid_nodes += 1;
        }
    }

    if valid_nodes == 0 {
        return Err(
            "No valid ComfyUI nodes found. Make sure you're using the API format (Save API Format from ComfyUI)".to_string()
        );
    }

    Ok(())
}

/// Find nodes in a workflow by their class_type.
///
/// Returns a vector of (node_id, node_value) tuples for all matching nodes.
/// This is a pure function with no side effects.
pub fn find_nodes_by_type(
    workflow: &serde_json::Value,
    class_type: &str,
) -> Vec<(String, serde_json::Value)> {
    let mut found = Vec::new();

    if let Some(nodes) = workflow.as_object() {
        for (node_id, node) in nodes {
            if let Some(ct) = node.get("class_type").and_then(|v| v.as_str()) {
                if ct == class_type {
                    found.push((node_id.clone(), node.clone()));
                }
            }
        }
    }

    found
}

/// Auto-detect prompt mappings from common node types.
///
/// Looks for CLIPTextEncode nodes and attempts to determine which are
/// positive vs negative prompts based on their titles.
/// This is a pure function with no side effects.
pub fn auto_detect_prompt_mappings(workflow: &serde_json::Value) -> Vec<PromptMapping> {
    let mut mappings = Vec::new();

    // Look for CLIPTextEncode nodes (most common for prompts).
    let clip_nodes = find_nodes_by_type(workflow, "CLIPTextEncode");

    for (node_id, node) in clip_nodes {
        // Check the node title to guess if it's positive or negative.
        let title = node
            .get("_meta")
            .and_then(|m| m.get("title"))
            .and_then(|t| t.as_str())
            .unwrap_or("");

        let is_negative =
            title.to_lowercase().contains("negative") || title.to_lowercase().contains("neg");

        let mapping_type = if is_negative {
            PromptMappingType::Negative
        } else {
            // Default to primary for the first positive prompt found.
            if mappings
                .iter()
                .any(|m: &PromptMapping| m.mapping_type == PromptMappingType::Primary)
            {
                continue; // Skip additional positive prompts.
            }
            PromptMappingType::Primary
        };

        mappings.push(PromptMapping {
            node_id: node_id.clone(),
            input_name: "text".to_string(),
            mapping_type,
        });
    }

    mappings
}

/// Workflow configuration for a specific asset generation slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfiguration {
    pub id: String,
    /// The slot this workflow is configured for.
    pub slot: WorkflowSlot,
    /// User-friendly name for this workflow.
    pub name: String,
    /// The raw ComfyUI API workflow JSON.
    pub workflow_json: serde_json::Value,
    /// Which text inputs should receive the generation prompt.
    pub prompt_mappings: Vec<PromptMapping>,
    /// Default values for workflow inputs.
    pub input_defaults: Vec<InputDefault>,
    /// Input paths that should always use defaults (never shown in UI).
    pub locked_inputs: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl WorkflowConfiguration {
    /// Create a new workflow configuration.
    pub fn new(
        slot: WorkflowSlot,
        name: impl Into<String>,
        workflow_json: serde_json::Value,
        now: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            slot,
            name: name.into(),
            workflow_json,
            prompt_mappings: Vec::new(),
            input_defaults: Vec::new(),
            locked_inputs: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Add a prompt mapping.
    pub fn with_prompt_mapping(mut self, mapping: PromptMapping) -> Self {
        self.prompt_mappings.push(mapping);
        self
    }

    /// Add a default value for an input.
    pub fn with_default(mut self, default: InputDefault) -> Self {
        self.input_defaults.push(default);
        self
    }

    /// Lock an input (always use default, hide from UI).
    pub fn with_locked_input(mut self, input_path: impl Into<String>) -> Self {
        self.locked_inputs.push(input_path.into());
        self
    }

    /// Get the default value for a specific input, if set.
    pub fn get_default(&self, node_id: &str, input_name: &str) -> Option<&serde_json::Value> {
        self.input_defaults
            .iter()
            .find(|d| d.node_id == node_id && d.input_name == input_name)
            .map(|d| &d.default_value)
    }

    /// Check if an input is locked.
    pub fn is_locked(&self, node_id: &str, input_name: &str) -> bool {
        let path = format!("{}.{}", node_id, input_name);
        self.locked_inputs.contains(&path)
    }

    /// Get the primary prompt mapping.
    pub fn primary_prompt_mapping(&self) -> Option<&PromptMapping> {
        self.prompt_mappings
            .iter()
            .find(|m| m.mapping_type == PromptMappingType::Primary)
    }

    /// Get the negative prompt mapping.
    pub fn negative_prompt_mapping(&self) -> Option<&PromptMapping> {
        self.prompt_mappings
            .iter()
            .find(|m| m.mapping_type == PromptMappingType::Negative)
    }

    /// Update the workflow JSON.
    pub fn update_workflow(&mut self, workflow_json: serde_json::Value, now: String) {
        self.workflow_json = workflow_json;
        self.updated_at = now;
    }

    /// Update prompt mappings.
    pub fn set_prompt_mappings(&mut self, mappings: Vec<PromptMapping>, now: String) {
        self.prompt_mappings = mappings;
        self.updated_at = now;
    }

    /// Update input defaults.
    pub fn set_input_defaults(&mut self, defaults: Vec<InputDefault>, now: String) {
        self.input_defaults = defaults;
        self.updated_at = now;
    }

    /// Update locked inputs.
    pub fn set_locked_inputs(&mut self, locked: Vec<String>, now: String) {
        self.locked_inputs = locked;
        self.updated_at = now;
    }
}
