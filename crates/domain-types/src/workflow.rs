//! Workflow configuration types for ComfyUI integration
//!
//! These types represent the configurable aspects of AI asset generation workflows.

use serde::{Deserialize, Serialize};

/// Slots for different asset types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
}

impl WorkflowSlot {
    /// Get default dimensions for this slot
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
        }
    }

    /// Get display name for this slot
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
        }
    }

    /// Get the category for grouping in UI
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
        }
    }

    /// Get all slots
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

    /// Convert to string
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
        }
    }
}

impl std::str::FromStr for WorkflowSlot {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "character_portrait" => Ok(Self::CharacterPortrait),
            "character_sprite" => Ok(Self::CharacterSprite),
            "character_expression_sheet" => Ok(Self::CharacterExpressionSheet),
            "location_backdrop" => Ok(Self::LocationBackdrop),
            "location_tilesheet" => Ok(Self::LocationTilesheet),
            "location_time_variant" => Ok(Self::LocationTimeVariant),
            "item_icon" => Ok(Self::ItemIcon),
            "item_set" => Ok(Self::ItemSet),
            "map_region" => Ok(Self::MapRegion),
            _ => Err(format!("Unknown workflow slot: {}", s)),
        }
    }
}

/// Type of prompt mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PromptMappingType {
    /// The main generation prompt
    Primary,
    /// The negative prompt (things to avoid)
    Negative,
}

/// Mapping of a text input to prompt injection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptMapping {
    /// The node ID in the workflow
    pub node_id: String,
    /// The input name on that node
    pub input_name: String,
    /// What type of prompt this receives
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

/// Default value for a workflow input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDefault {
    /// The node ID in the workflow
    pub node_id: String,
    /// The input name on that node
    pub input_name: String,
    /// The default value
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

    /// Get the path as "node_id.input_name"
    pub fn path(&self) -> String {
        format!("{}.{}", self.node_id, self.input_name)
    }
}

/// Detected input type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputType {
    /// Text/string input
    Text,
    /// Integer number
    Integer,
    /// Floating point number
    Float,
    /// Boolean/checkbox
    Boolean,
    /// Select from options
    Select(Vec<String>),
    /// Unknown type
    Unknown,
}

impl InputType {
    /// Detect the type from a JSON value
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

/// Parsed input from a workflow (for UI display)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowInput {
    /// The node ID this input belongs to
    pub node_id: String,
    /// The class type of the node (e.g., "KSampler", "CLIPTextEncode")
    pub node_type: String,
    /// The node's display title (from _meta)
    pub node_title: Option<String>,
    /// The input parameter name
    pub input_name: String,
    /// Detected type of the input
    pub input_type: InputType,
    /// Current value in the workflow
    pub current_value: serde_json::Value,
}

impl WorkflowInput {
    /// Get the path as "node_id.input_name"
    pub fn path(&self) -> String {
        format!("{}.{}", self.node_id, self.input_name)
    }

    /// Get display name for this input
    pub fn display_name(&self) -> String {
        if let Some(title) = &self.node_title {
            format!("{} → {}", title, self.input_name)
        } else {
            format!("{} → {}", self.node_type, self.input_name)
        }
    }
}

/// Result of analyzing a workflow JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowAnalysis {
    /// Total number of nodes in the workflow
    pub node_count: usize,
    /// All configurable (non-connection) inputs
    pub inputs: Vec<WorkflowInput>,
    /// Text inputs that could be prompt fields
    pub text_inputs: Vec<WorkflowInput>,
    /// Validation errors, if any
    pub errors: Vec<String>,
}

impl WorkflowAnalysis {
    /// Check if the workflow is valid for use
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty() && self.node_count > 0
    }
}
