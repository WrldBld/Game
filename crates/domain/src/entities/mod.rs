//! Domain entities - Core business objects with identity

mod challenge;
mod character;
mod character_content;
mod class_feature;
mod content_types;
mod event_chain;
mod feat;
mod gallery_asset;
mod game_flag;
mod generation_batch;
mod goal;
mod grid_map;
mod interaction;
mod item;
mod location;
mod location_state;
mod lore;
mod narrative_event;
mod observation;
mod player_character;
mod region;
mod region_state;
mod scene;
mod skill;
mod spell;
mod staging;
mod story_event;
mod want;
mod workflow_config;
mod world;

pub use challenge::{
    Challenge, ChallengeLocationAvailability, ChallengeOutcomes, ChallengePrerequisite,
    ChallengeRegionAvailability, ChallengeType, ChallengeUnlock, Difficulty, DifficultyDescriptor,
    Outcome, OutcomeTrigger, OutcomeType, TriggerCondition, TriggerType,
};
pub use character::{Character, StatBlock, StatModifier, StatValue};
pub use character_content::{
    AcquiredFeat, ActiveFeature, CharacterFeats, CharacterFeatures, CharacterIdentity,
    CharacterSpells, ClassLevel, KnownSpell, SpellSlotPool,
};
pub use class_feature::{BackgroundFeature, ClassFeature, FeatureUses, RacialTrait};
pub use content_types::{ContentFilter, ContentItem, ContentSource, ContentType, SourceType};
pub use event_chain::{ChainStatus, EventChain};
pub use feat::{AbilityUses, Feat, FeatBenefit, Prerequisite, RechargeType, UsesFormula};
pub use gallery_asset::{AssetType, EntityType, GalleryAsset, GenerationMetadata};
pub use game_flag::{FlagScope, GameFlag};
pub use generation_batch::{BatchStatus, GenerationBatch, GenerationRequest};
pub use goal::Goal;
pub use grid_map::GridMap;
pub use interaction::{
    InteractionCondition, InteractionRequirement, InteractionTarget, InteractionTargetType,
    InteractionTemplate, InteractionType,
};
pub use item::{AcquisitionMethod, FrequencyLevel, InventoryItem, Item};
pub use location::{Location, LocationConnection, LocationType};
pub use location_state::{LocationState, LocationStateSummary};
pub use lore::{Lore, LoreCategory, LoreChunk, LoreDiscoverySource, LoreKnowledge};
pub use narrative_event::{
    ChainedEvent, EventChainMembership, EventEffect, EventOutcome, FeaturedNpc, NarrativeEvent,
    NarrativeTrigger, NarrativeTriggerType, OutcomeCondition, TriggerContext, TriggerEvaluation,
    TriggerLogic,
};
pub use observation::{NpcObservation, ObservationSummary, ObservationType};
pub use player_character::PlayerCharacter;
pub use region::{MapBounds, Region, RegionConnection, RegionExit};
pub use region_state::{RegionState, RegionStateSummary};
pub use scene::{Scene, SceneCharacter, SceneCharacterRole, SceneCondition, TimeContext};
pub use skill::{default_skills_for_variant, Skill, SkillCategory};
pub use spell::{
    CastingTime, CastingTimeUnit, DurationUnit, MaterialComponent, Spell, SpellComponents,
    SpellDuration, SpellLevel, SpellRange,
};
pub use staging::{
    ResolvedStateInfo, ResolvedVisualState, StagedNpc, Staging, StagingSource, VisualStateSource,
};
pub use story_event::InfoImportance as StoryEventInfoImportance;
pub use story_event::{
    ChallengeEventOutcome, CombatEventType, CombatOutcome, DmMarkerType, InfoType,
    InvolvedCharacter, ItemSource, MarkerImportance, StoryEvent, StoryEventType,
};
pub use want::{ActantialRole, ActantialView, CharacterWant, Want, WantTargetType, WantVisibility};
pub use workflow_config::{
    InputDefault, InputType, PromptMapping, PromptMappingType, WorkflowAnalysis,
    WorkflowConfiguration, WorkflowInput, WorkflowSlot,
};
pub use world::{Act, MonomythStage, TimeAdvanceResult, World};
