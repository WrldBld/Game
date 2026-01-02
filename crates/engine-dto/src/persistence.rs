//! Persistence DTOs for Neo4j JSON serialization
//!
//! These DTOs are used by Neo4j adapters for storing complex domain types as JSON.
//! Moving them to engine-dto removes the adapters→app dependency cycle.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use wrldbldr_domain::entities::{
    ChallengeOutcomes, Difficulty, DifficultyDescriptor, FieldType, ItemListType, Outcome,
    OutcomeTrigger, SectionLayout, SelectOption, SheetField, SheetSection, TriggerCondition,
    TriggerType,
};
use wrldbldr_domain::value_objects::RuleSystemVariant;
use wrldbldr_domain::{ChallengeId, SceneId, WorldId};

// ============================================================================
// Challenge Persistence DTOs
// ============================================================================

/// Difficulty persistence format
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DifficultyRequestDto {
    Dc {
        value: u32,
    },
    Percentage {
        value: u32,
    },
    Descriptor {
        value: String,
    },
    Opposed,
    Custom {
        value: String,
    },
    #[serde(other)]
    Unknown,
}

impl From<DifficultyRequestDto> for Difficulty {
    fn from(req: DifficultyRequestDto) -> Self {
        match req {
            DifficultyRequestDto::Dc { value } => Difficulty::DC(value),
            DifficultyRequestDto::Percentage { value } => Difficulty::Percentage(value),
            DifficultyRequestDto::Descriptor { value } => {
                let descriptor = match value.to_lowercase().as_str() {
                    "trivial" => DifficultyDescriptor::Trivial,
                    "easy" => DifficultyDescriptor::Easy,
                    "routine" => DifficultyDescriptor::Routine,
                    "moderate" => DifficultyDescriptor::Moderate,
                    "challenging" => DifficultyDescriptor::Challenging,
                    "hard" => DifficultyDescriptor::Hard,
                    "very_hard" | "veryhard" => DifficultyDescriptor::VeryHard,
                    "extreme" => DifficultyDescriptor::Extreme,
                    "impossible" => DifficultyDescriptor::Impossible,
                    "risky" => DifficultyDescriptor::Risky,
                    "desperate" => DifficultyDescriptor::Desperate,
                    _ => DifficultyDescriptor::Moderate,
                };
                Difficulty::Descriptor(descriptor)
            }
            DifficultyRequestDto::Opposed => Difficulty::Opposed,
            DifficultyRequestDto::Custom { value } => Difficulty::Custom(value),
            DifficultyRequestDto::Unknown => Difficulty::Descriptor(DifficultyDescriptor::Moderate),
        }
    }
}

impl From<Difficulty> for DifficultyRequestDto {
    fn from(d: Difficulty) -> Self {
        match d {
            Difficulty::DC(v) => DifficultyRequestDto::Dc { value: v },
            Difficulty::Percentage(v) => DifficultyRequestDto::Percentage { value: v },
            Difficulty::Descriptor(d) => DifficultyRequestDto::Descriptor {
                value: format!("{:?}", d).to_lowercase(),
            },
            Difficulty::Opposed => DifficultyRequestDto::Opposed,
            Difficulty::Custom(s) => DifficultyRequestDto::Custom { value: s },
        }
    }
}

/// Outcomes persistence format
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct OutcomesRequestDto {
    pub success: OutcomeRequestDto,
    pub failure: OutcomeRequestDto,
    #[serde(default)]
    pub partial: Option<OutcomeRequestDto>,
    #[serde(default)]
    pub critical_success: Option<OutcomeRequestDto>,
    #[serde(default)]
    pub critical_failure: Option<OutcomeRequestDto>,
}

impl From<OutcomesRequestDto> for ChallengeOutcomes {
    fn from(req: OutcomesRequestDto) -> Self {
        Self {
            success: req.success.into(),
            failure: req.failure.into(),
            partial: req.partial.map(Into::into),
            critical_success: req.critical_success.map(Into::into),
            critical_failure: req.critical_failure.map(Into::into),
        }
    }
}

impl From<ChallengeOutcomes> for OutcomesRequestDto {
    fn from(o: ChallengeOutcomes) -> Self {
        Self {
            success: o.success.into(),
            failure: o.failure.into(),
            partial: o.partial.map(Into::into),
            critical_success: o.critical_success.map(Into::into),
            critical_failure: o.critical_failure.map(Into::into),
        }
    }
}

/// Single outcome persistence format
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct OutcomeRequestDto {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub triggers: Vec<OutcomeTriggerRequestDto>,
}

impl From<OutcomeRequestDto> for Outcome {
    fn from(req: OutcomeRequestDto) -> Self {
        Self {
            description: req.description,
            triggers: req.triggers.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<Outcome> for OutcomeRequestDto {
    fn from(o: Outcome) -> Self {
        Self {
            description: o.description,
            triggers: o.triggers.into_iter().map(Into::into).collect(),
        }
    }
}

/// Outcome trigger persistence format
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutcomeTriggerRequestDto {
    RevealInformation {
        info: String,
        persist: bool,
    },
    EnableChallenge {
        challenge_id: String,
    },
    DisableChallenge {
        challenge_id: String,
    },
    ModifyCharacterStat {
        stat: String,
        modifier: i32,
    },
    TriggerScene {
        scene_id: String,
    },
    GiveItem {
        item_name: String,
        item_description: Option<String>,
    },
    Custom {
        description: String,
    },
    #[serde(other)]
    Unknown,
}

impl From<OutcomeTriggerRequestDto> for OutcomeTrigger {
    fn from(req: OutcomeTriggerRequestDto) -> Self {
        match req {
            OutcomeTriggerRequestDto::RevealInformation { info, persist } => {
                OutcomeTrigger::RevealInformation { info, persist }
            }
            OutcomeTriggerRequestDto::EnableChallenge { challenge_id } => {
                let id = Uuid::parse_str(&challenge_id)
                    .map(ChallengeId::from_uuid)
                    .unwrap_or_else(|_| ChallengeId::new());
                OutcomeTrigger::EnableChallenge { challenge_id: id }
            }
            OutcomeTriggerRequestDto::DisableChallenge { challenge_id } => {
                let id = Uuid::parse_str(&challenge_id)
                    .map(ChallengeId::from_uuid)
                    .unwrap_or_else(|_| ChallengeId::new());
                OutcomeTrigger::DisableChallenge { challenge_id: id }
            }
            OutcomeTriggerRequestDto::ModifyCharacterStat { stat, modifier } => {
                OutcomeTrigger::ModifyCharacterStat { stat, modifier }
            }
            OutcomeTriggerRequestDto::TriggerScene { scene_id } => {
                let id = Uuid::parse_str(&scene_id)
                    .map(SceneId::from_uuid)
                    .unwrap_or_else(|_| SceneId::new());
                OutcomeTrigger::TriggerScene { scene_id: id }
            }
            OutcomeTriggerRequestDto::GiveItem {
                item_name,
                item_description,
            } => OutcomeTrigger::GiveItem {
                item_name,
                item_description,
            },
            OutcomeTriggerRequestDto::Custom { description } => {
                OutcomeTrigger::Custom { description }
            }
            OutcomeTriggerRequestDto::Unknown => OutcomeTrigger::Custom {
                description: "Unknown trigger type".to_string(),
            },
        }
    }
}

impl From<OutcomeTrigger> for OutcomeTriggerRequestDto {
    fn from(t: OutcomeTrigger) -> Self {
        match t {
            OutcomeTrigger::RevealInformation { info, persist } => {
                OutcomeTriggerRequestDto::RevealInformation { info, persist }
            }
            OutcomeTrigger::EnableChallenge { challenge_id } => {
                OutcomeTriggerRequestDto::EnableChallenge {
                    challenge_id: challenge_id.to_string(),
                }
            }
            OutcomeTrigger::DisableChallenge { challenge_id } => {
                OutcomeTriggerRequestDto::DisableChallenge {
                    challenge_id: challenge_id.to_string(),
                }
            }
            OutcomeTrigger::ModifyCharacterStat { stat, modifier } => {
                OutcomeTriggerRequestDto::ModifyCharacterStat { stat, modifier }
            }
            OutcomeTrigger::TriggerScene { scene_id } => OutcomeTriggerRequestDto::TriggerScene {
                scene_id: scene_id.to_string(),
            },
            OutcomeTrigger::GiveItem {
                item_name,
                item_description,
            } => OutcomeTriggerRequestDto::GiveItem {
                item_name,
                item_description,
            },
            OutcomeTrigger::Custom { description } => {
                OutcomeTriggerRequestDto::Custom { description }
            }
        }
    }
}

/// Trigger condition persistence format
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TriggerConditionRequestDto {
    pub condition_type: TriggerTypeRequestDto,
    pub description: String,
    #[serde(default)]
    pub required: bool,
}

impl From<TriggerConditionRequestDto> for TriggerCondition {
    fn from(req: TriggerConditionRequestDto) -> Self {
        let mut tc = TriggerCondition::new(req.condition_type.into(), req.description);
        if req.required {
            tc = tc.required();
        }
        tc
    }
}

impl From<TriggerCondition> for TriggerConditionRequestDto {
    fn from(tc: TriggerCondition) -> Self {
        Self {
            condition_type: tc.condition_type.into(),
            description: tc.description,
            required: tc.required,
        }
    }
}

/// Trigger type persistence format
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerTypeRequestDto {
    ObjectInteraction {
        keywords: Vec<String>,
    },
    EnterArea {
        keywords: Vec<String>,
    },
    DialogueTopic {
        keywords: Vec<String>,
    },
    ChallengeComplete {
        challenge_id: String,
        requires_success: Option<bool>,
    },
    TimeBased {
        turns: u32,
    },
    NpcPresent {
        keywords: Vec<String>,
    },
    Custom {
        description: String,
    },
    #[serde(other)]
    Unknown,
}

impl From<TriggerTypeRequestDto> for TriggerType {
    fn from(req: TriggerTypeRequestDto) -> Self {
        match req {
            TriggerTypeRequestDto::ObjectInteraction { keywords } => {
                TriggerType::ObjectInteraction { keywords }
            }
            TriggerTypeRequestDto::EnterArea { keywords } => TriggerType::EnterArea {
                area_keywords: keywords,
            },
            TriggerTypeRequestDto::DialogueTopic { keywords } => TriggerType::DialogueTopic {
                topic_keywords: keywords,
            },
            TriggerTypeRequestDto::ChallengeComplete {
                challenge_id,
                requires_success,
            } => {
                let id = Uuid::parse_str(&challenge_id)
                    .map(ChallengeId::from_uuid)
                    .unwrap_or_else(|_| ChallengeId::new());
                TriggerType::ChallengeComplete {
                    challenge_id: id,
                    requires_success,
                }
            }
            TriggerTypeRequestDto::TimeBased { turns } => TriggerType::TimeBased { turns },
            TriggerTypeRequestDto::NpcPresent { keywords } => TriggerType::NpcPresent {
                npc_keywords: keywords,
            },
            TriggerTypeRequestDto::Custom { description } => TriggerType::Custom { description },
            TriggerTypeRequestDto::Unknown => TriggerType::Custom {
                description: "Unknown trigger type".to_string(),
            },
        }
    }
}

impl From<TriggerType> for TriggerTypeRequestDto {
    fn from(t: TriggerType) -> Self {
        match t {
            TriggerType::ObjectInteraction { keywords } => {
                TriggerTypeRequestDto::ObjectInteraction { keywords }
            }
            TriggerType::EnterArea { area_keywords } => TriggerTypeRequestDto::EnterArea {
                keywords: area_keywords,
            },
            TriggerType::DialogueTopic { topic_keywords } => TriggerTypeRequestDto::DialogueTopic {
                keywords: topic_keywords,
            },
            TriggerType::ChallengeComplete {
                challenge_id,
                requires_success,
            } => TriggerTypeRequestDto::ChallengeComplete {
                challenge_id: challenge_id.to_string(),
                requires_success,
            },
            TriggerType::TimeBased { turns } => TriggerTypeRequestDto::TimeBased { turns },
            TriggerType::NpcPresent { npc_keywords } => TriggerTypeRequestDto::NpcPresent {
                keywords: npc_keywords,
            },
            TriggerType::Custom { description } => TriggerTypeRequestDto::Custom { description },
        }
    }
}

// ============================================================================
// Sheet Template Persistence DTOs
// ============================================================================

/// Storage format for character sheet templates in Neo4j
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetTemplateStorageDto {
    pub id: String,
    pub world_id: String,
    pub name: String,
    pub description: String,
    pub variant: serde_json::Value,
    pub sections: Vec<SheetSectionDto>,
    pub is_default: bool,
}

impl SheetTemplateStorageDto {
    pub fn variant_to_json(variant: &RuleSystemVariant) -> serde_json::Value {
        match variant {
            RuleSystemVariant::Dnd5e => serde_json::Value::String("Dnd5e".to_string()),
            RuleSystemVariant::Pathfinder2e => {
                serde_json::Value::String("Pathfinder2e".to_string())
            }
            RuleSystemVariant::GenericD20 => serde_json::Value::String("GenericD20".to_string()),
            RuleSystemVariant::CallOfCthulhu7e => {
                serde_json::Value::String("CallOfCthulhu7e".to_string())
            }
            RuleSystemVariant::RuneQuest => serde_json::Value::String("RuneQuest".to_string()),
            RuleSystemVariant::GenericD100 => serde_json::Value::String("GenericD100".to_string()),
            RuleSystemVariant::KidsOnBikes => serde_json::Value::String("KidsOnBikes".to_string()),
            RuleSystemVariant::FateCore => serde_json::Value::String("FateCore".to_string()),
            RuleSystemVariant::PoweredByApocalypse => {
                serde_json::Value::String("PoweredByApocalypse".to_string())
            }
            RuleSystemVariant::BladesInTheDark => {
                serde_json::Value::String("BladesInTheDark".to_string())
            }
            RuleSystemVariant::Custom(name) => serde_json::json!({ "Custom": name }),
        }
    }

    pub fn variant_from_json(value: serde_json::Value) -> RuleSystemVariant {
        match value {
            serde_json::Value::String(s) => match s.as_str() {
                "Dnd5e" => RuleSystemVariant::Dnd5e,
                "Pathfinder2e" => RuleSystemVariant::Pathfinder2e,
                "GenericD20" => RuleSystemVariant::GenericD20,
                "CallOfCthulhu7e" => RuleSystemVariant::CallOfCthulhu7e,
                "RuneQuest" => RuleSystemVariant::RuneQuest,
                "GenericD100" => RuleSystemVariant::GenericD100,
                "KidsOnBikes" => RuleSystemVariant::KidsOnBikes,
                "FateCore" => RuleSystemVariant::FateCore,
                "PoweredByApocalypse" => RuleSystemVariant::PoweredByApocalypse,
                "BladesInTheDark" => RuleSystemVariant::BladesInTheDark,
                other => RuleSystemVariant::Custom(other.to_string()),
            },
            serde_json::Value::Object(map) => {
                if let Some(serde_json::Value::String(name)) = map.get("Custom") {
                    RuleSystemVariant::Custom(name.clone())
                } else {
                    RuleSystemVariant::Custom(serde_json::Value::Object(map).to_string())
                }
            }
            other => RuleSystemVariant::Custom(other.to_string()),
        }
    }
}

impl From<&wrldbldr_domain::entities::CharacterSheetTemplate> for SheetTemplateStorageDto {
    fn from(value: &wrldbldr_domain::entities::CharacterSheetTemplate) -> Self {
        Self {
            id: value.id.0.clone(),
            world_id: value.world_id.to_string(),
            name: value.name.clone(),
            description: value.description.clone(),
            variant: Self::variant_to_json(&value.variant),
            sections: value.sections.clone().into_iter().map(Into::into).collect(),
            is_default: value.is_default,
        }
    }
}

impl TryFrom<SheetTemplateStorageDto> for wrldbldr_domain::entities::CharacterSheetTemplate {
    type Error = String;

    fn try_from(value: SheetTemplateStorageDto) -> Result<Self, Self::Error> {
        let world_uuid = Uuid::parse_str(&value.world_id)
            .map_err(|e| format!("Invalid world_id UUID: {}", e))?;
        let world_id = WorldId::from_uuid(world_uuid);

        Ok(Self {
            id: wrldbldr_domain::entities::SheetTemplateId::from_string(value.id),
            world_id,
            name: value.name,
            description: value.description,
            variant: SheetTemplateStorageDto::variant_from_json(value.variant),
            sections: value.sections.into_iter().map(Into::into).collect(),
            is_default: value.is_default,
        })
    }
}

/// Sheet section persistence format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetSectionDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<SheetFieldDto>,
    pub layout: SectionLayoutDto,
    pub collapsible: bool,
    pub collapsed_by_default: bool,
    pub order: u32,
}

impl From<SheetSection> for SheetSectionDto {
    fn from(value: SheetSection) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            fields: value.fields.into_iter().map(Into::into).collect(),
            layout: value.layout.into(),
            collapsible: value.collapsible,
            collapsed_by_default: value.collapsed_by_default,
            order: value.order,
        }
    }
}

impl From<SheetSectionDto> for SheetSection {
    fn from(value: SheetSectionDto) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            fields: value.fields.into_iter().map(Into::into).collect(),
            layout: value.layout.into(),
            collapsible: value.collapsible,
            collapsed_by_default: value.collapsed_by_default,
            order: value.order,
        }
    }
}

/// Sheet field persistence format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetFieldDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub field_type: FieldTypeDto,
    pub required: bool,
    pub read_only: bool,
    pub order: u32,
}

impl From<SheetField> for SheetFieldDto {
    fn from(value: SheetField) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            field_type: value.field_type.into(),
            required: value.required,
            read_only: value.read_only,
            order: value.order,
        }
    }
}

impl From<SheetFieldDto> for SheetField {
    fn from(value: SheetFieldDto) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            field_type: value.field_type.into(),
            required: value.required,
            read_only: value.read_only,
            order: value.order,
        }
    }
}

/// Section layout persistence format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SectionLayoutDto {
    Vertical,
    Grid {
        columns: u8,
    },
    Flow,
    TwoColumn,
    #[serde(other)]
    Unknown,
}

impl From<SectionLayout> for SectionLayoutDto {
    fn from(value: SectionLayout) -> Self {
        match value {
            SectionLayout::Vertical => Self::Vertical,
            SectionLayout::Grid { columns } => Self::Grid { columns },
            SectionLayout::Flow => Self::Flow,
            SectionLayout::TwoColumn => Self::TwoColumn,
        }
    }
}

impl From<SectionLayoutDto> for SectionLayout {
    fn from(value: SectionLayoutDto) -> Self {
        match value {
            SectionLayoutDto::Vertical => Self::Vertical,
            SectionLayoutDto::Grid { columns } => Self::Grid { columns },
            SectionLayoutDto::Flow => Self::Flow,
            SectionLayoutDto::TwoColumn => Self::TwoColumn,
            SectionLayoutDto::Unknown => Self::Vertical,
        }
    }
}

/// Select option persistence format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOptionDto {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

impl From<SelectOption> for SelectOptionDto {
    fn from(value: SelectOption) -> Self {
        Self {
            value: value.value,
            label: value.label,
            description: value.description,
        }
    }
}

impl From<SelectOptionDto> for SelectOption {
    fn from(value: SelectOptionDto) -> Self {
        Self {
            value: value.value,
            label: value.label,
            description: value.description,
        }
    }
}

/// Item list type persistence format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemListTypeDto {
    Inventory,
    Features,
    Spells,
    Notes,
    #[serde(other)]
    Unknown,
}

impl From<ItemListType> for ItemListTypeDto {
    fn from(value: ItemListType) -> Self {
        match value {
            ItemListType::Inventory => Self::Inventory,
            ItemListType::Features => Self::Features,
            ItemListType::Spells => Self::Spells,
            ItemListType::Notes => Self::Notes,
        }
    }
}

impl From<ItemListTypeDto> for ItemListType {
    fn from(value: ItemListTypeDto) -> Self {
        match value {
            ItemListTypeDto::Inventory => Self::Inventory,
            ItemListTypeDto::Features => Self::Features,
            ItemListTypeDto::Spells => Self::Spells,
            ItemListTypeDto::Notes => Self::Notes,
            ItemListTypeDto::Unknown => Self::Inventory,
        }
    }
}

/// Field type persistence format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FieldTypeDto {
    Number {
        min: Option<i32>,
        max: Option<i32>,
        default: Option<i32>,
    },
    Text {
        multiline: bool,
        max_length: Option<usize>,
    },
    Checkbox {
        default: bool,
    },
    Select {
        options: Vec<SelectOptionDto>,
    },
    SkillReference {
        categories: Option<Vec<String>>,
        show_attribute: bool,
    },
    Derived {
        formula: String,
        depends_on: Vec<String>,
    },
    Resource {
        max_field: Option<String>,
        default_max: Option<i32>,
    },
    ItemList {
        item_type: ItemListTypeDto,
        max_items: Option<usize>,
    },
    SkillList {
        show_modifier: bool,
        show_proficiency: bool,
    },
    #[serde(other)]
    Unknown,
}

impl From<FieldType> for FieldTypeDto {
    fn from(value: FieldType) -> Self {
        match value {
            FieldType::Number { min, max, default } => Self::Number { min, max, default },
            FieldType::Text {
                multiline,
                max_length,
            } => Self::Text {
                multiline,
                max_length,
            },
            FieldType::Checkbox { default } => Self::Checkbox { default },
            FieldType::Select { options } => Self::Select {
                options: options.into_iter().map(Into::into).collect(),
            },
            FieldType::SkillReference {
                categories,
                show_attribute,
            } => Self::SkillReference {
                categories,
                show_attribute,
            },
            FieldType::Derived {
                formula,
                depends_on,
            } => Self::Derived {
                formula,
                depends_on,
            },
            FieldType::Resource {
                max_field,
                default_max,
            } => Self::Resource {
                max_field,
                default_max,
            },
            FieldType::ItemList {
                item_type,
                max_items,
            } => Self::ItemList {
                item_type: item_type.into(),
                max_items,
            },
            FieldType::SkillList {
                show_modifier,
                show_proficiency,
            } => Self::SkillList {
                show_modifier,
                show_proficiency,
            },
        }
    }
}

impl From<FieldTypeDto> for FieldType {
    fn from(value: FieldTypeDto) -> Self {
        match value {
            FieldTypeDto::Number { min, max, default } => Self::Number { min, max, default },
            FieldTypeDto::Text {
                multiline,
                max_length,
            } => Self::Text {
                multiline,
                max_length,
            },
            FieldTypeDto::Checkbox { default } => Self::Checkbox { default },
            FieldTypeDto::Select { options } => Self::Select {
                options: options.into_iter().map(Into::into).collect(),
            },
            FieldTypeDto::SkillReference {
                categories,
                show_attribute,
            } => Self::SkillReference {
                categories,
                show_attribute,
            },
            FieldTypeDto::Derived {
                formula,
                depends_on,
            } => Self::Derived {
                formula,
                depends_on,
            },
            FieldTypeDto::Resource {
                max_field,
                default_max,
            } => Self::Resource {
                max_field,
                default_max,
            },
            FieldTypeDto::ItemList {
                item_type,
                max_items,
            } => Self::ItemList {
                item_type: item_type.into(),
                max_items,
            },
            FieldTypeDto::SkillList {
                show_modifier,
                show_proficiency,
            } => Self::SkillList {
                show_modifier,
                show_proficiency,
            },
            // Unknown field types default to a text field
            FieldTypeDto::Unknown => Self::Text {
                multiline: false,
                max_length: None,
            },
        }
    }
}

// ============================================================================
// ============================================================================
// Workflow Persistence DTOs - Re-exported from protocol (single source of truth)
// ============================================================================
//
// These types are defined in wrldbldr_protocol::dto and re-exported here for
// convenience. The From implementations are also defined in protocol.

pub use wrldbldr_protocol::dto::{InputDefaultDto, PromptMappingDto, PromptMappingTypeDto};
