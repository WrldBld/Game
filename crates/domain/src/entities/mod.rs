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
mod region;
mod region_state;
mod scene;
mod skill;
mod spell;
mod staging;
mod story_event;
mod want;
mod world;

pub use challenge::{
    Challenge, ChallengeLocationAvailability, ChallengeOutcomes, ChallengePrerequisite,
    ChallengeRegionAvailability, ChallengeType, ChallengeUnlock, Difficulty, DifficultyDescriptor,
    Outcome, OutcomeTrigger, OutcomeType, TriggerCondition, TriggerType,
};
// Note: Character is now exported from aggregates
// StatBlock, StatModifier, StatValue are now exported from value_objects
pub use character_content::{
    AcquiredFeat, ActiveFeature, CharacterFeats, CharacterFeatures, CharacterIdentity,
    CharacterSpells, ClassLevel, KnownSpell, SpellSlotPool,
};
pub use class_feature::{BackgroundFeature, ClassFeature, FeatureUses, RacialTrait};
pub use content_types::{ContentSource, ContentType, SourceType};
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
// Note: Location is now exported from aggregates
pub use location::{ConnectionType, LocationConnection, LocationType};
pub use location_state::{LocationState, LocationStateSummary};
pub use lore::{Lore, LoreCategory, LoreChunk, LoreDiscoverySource, LoreKnowledge};
// Note: NarrativeEvent is now exported from aggregates
pub use narrative_event::{
    ChainedEvent, EventChainMembership, EventEffect, EventOutcome, FeaturedNpc, NarrativeTrigger,
    NarrativeTriggerType, OutcomeCondition, TriggerContext, TriggerEvaluation, TriggerLogic,
};
pub use observation::{NpcObservation, ObservationSummary, ObservationType};
// Note: PlayerCharacter is now exported from aggregates
pub use region::{MapBounds, Region, RegionConnection, RegionExit};
pub use region_state::{RegionState, RegionStateSummary};
// Note: Scene is now exported from aggregates
pub use scene::{SceneCharacter, SceneCharacterRole, SceneCondition, TimeContext};
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
// Note: World is now exported from aggregates
pub use world::{Act, MonomythStage, TimeAdvanceResult};
