//! Actantial context value objects for LLM consumption
//!
//! These types represent the resolved, aggregated actantial model data
//! that gets passed to the LLM for character roleplay context.
//!
//! # Design Notes
//!
//! - `ActantialTarget` distinguishes between NPC and PC targets for actantial views
//! - `WantTarget` is the resolved target of a want (Character, Item, or Goal)
//! - `WantContext` is a fully resolved want with all its associated data
//! - `ActantialContext` is the complete context for a character
//!
//! # Usage
//!
//! The `ActantialContextService` aggregates data from multiple repositories
//! to build these resolved contexts for LLM consumption.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::WantVisibility;

// =============================================================================
// Actor Type (simple enum for LLM context)
// =============================================================================

/// Simple type discriminator for actors (NPC vs PC)
///
/// Used in LLM context serialization where we only need the type label,
/// not the full ID. For internal operations with IDs, use `ActantialTarget`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorType {
    /// Non-player character
    Npc,
    /// Player character
    Pc,
}

impl std::fmt::Display for ActorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorType::Npc => write!(f, "NPC"),
            ActorType::Pc => write!(f, "PC"),
        }
    }
}

// =============================================================================
// Actantial Target (NPC or PC with ID)
// =============================================================================

/// Target of an actantial view - can be an NPC or a Player Character
///
/// NPCs can view both other NPCs and PCs as helpers/opponents/etc.
/// This enum allows the system to track these relationships uniformly.
///
/// Uses Uuid internally for serde compatibility; convert to/from typed IDs
/// at the service layer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActantialTarget {
    /// An NPC (Character)
    Npc(Uuid),
    /// A Player Character
    Pc(Uuid),
}

impl ActantialTarget {
    /// Create from a CharacterId (NPC)
    pub fn npc(id: impl Into<Uuid>) -> Self {
        ActantialTarget::Npc(id.into())
    }

    /// Create from a PlayerCharacterId
    pub fn pc(id: impl Into<Uuid>) -> Self {
        ActantialTarget::Pc(id.into())
    }

    /// Get the ID as a Uuid
    pub fn id(&self) -> Uuid {
        match self {
            ActantialTarget::Npc(id) => *id,
            ActantialTarget::Pc(id) => *id,
        }
    }

    /// Get the ID as a string for generic operations
    pub fn id_string(&self) -> String {
        self.id().to_string()
    }

    /// Get the actor type (without ID)
    pub fn actor_type(&self) -> ActorType {
        match self {
            ActantialTarget::Npc(_) => ActorType::Npc,
            ActantialTarget::Pc(_) => ActorType::Pc,
        }
    }

    /// Get the type label for display
    pub fn type_label(&self) -> &'static str {
        match self {
            ActantialTarget::Npc(_) => "NPC",
            ActantialTarget::Pc(_) => "PC",
        }
    }

    /// Check if this is an NPC target
    pub fn is_npc(&self) -> bool {
        matches!(self, ActantialTarget::Npc(_))
    }

    /// Check if this is a PC target
    pub fn is_pc(&self) -> bool {
        matches!(self, ActantialTarget::Pc(_))
    }
}

// =============================================================================
// Want Target (resolved)
// =============================================================================

/// A resolved want target with its associated data
///
/// When a Want has a TARGETS edge, this represents the fully resolved target
/// with its name and ID for display/context purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WantTarget {
    /// Want targets a Character (NPC)
    Character {
        id: Uuid,
        name: String,
    },
    /// Want targets an Item
    Item {
        id: Uuid,
        name: String,
    },
    /// Want targets a Goal (abstract desire)
    Goal {
        id: Uuid,
        name: String,
        description: Option<String>,
    },
}

impl WantTarget {
    /// Get the target type as a string
    pub fn target_type(&self) -> &'static str {
        match self {
            WantTarget::Character { .. } => "Character",
            WantTarget::Item { .. } => "Item",
            WantTarget::Goal { .. } => "Goal",
        }
    }

    /// Get the target name
    pub fn name(&self) -> &str {
        match self {
            WantTarget::Character { name, .. } => name,
            WantTarget::Item { name, .. } => name,
            WantTarget::Goal { name, .. } => name,
        }
    }

    /// Get the target ID
    pub fn id(&self) -> Uuid {
        match self {
            WantTarget::Character { id, .. } => *id,
            WantTarget::Item { id, .. } => *id,
            WantTarget::Goal { id, .. } => *id,
        }
    }

    /// Format for LLM context
    pub fn to_context_string(&self) -> String {
        match self {
            WantTarget::Character { name, .. } => format!("the character {}", name),
            WantTarget::Item { name, .. } => format!("the item {}", name),
            WantTarget::Goal { name, description, .. } => {
                if let Some(desc) = description {
                    format!("{} ({})", name, desc)
                } else {
                    name.clone()
                }
            }
        }
    }
}

// =============================================================================
// Actantial Actor
// =============================================================================

/// An actor in the actantial model (helper, opponent, sender, or receiver)
///
/// This represents someone that the character views in a specific role
/// relative to one of their wants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActantialActor {
    /// The target (NPC or PC)
    pub target: ActantialTarget,
    /// The actor's name for display
    pub name: String,
    /// Why the character views them this way
    pub reason: String,
}

impl ActantialActor {
    pub fn new(target: ActantialTarget, name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            target,
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Format for LLM context
    pub fn to_context_string(&self) -> String {
        format!("{} ({})", self.name, self.reason)
    }
}

// =============================================================================
// Want Context (fully resolved)
// =============================================================================

/// A want with its full context (resolved target, actors, behavioral guidance)
///
/// This is the complete package of information about a single want,
/// ready for LLM consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WantContext {
    /// The want's ID
    pub want_id: Uuid,
    /// Description of what the character wants
    pub description: String,
    /// Intensity (0.0 = mild interest, 1.0 = obsession)
    pub intensity: f32,
    /// Priority (1 = primary want)
    pub priority: u32,
    /// How much the player knows
    pub visibility: WantVisibility,
    /// The resolved target (if any)
    pub target: Option<WantTarget>,
    /// How to behave when probed about this want
    pub deflection_behavior: Option<String>,
    /// Behavioral tells that hint at this want
    pub tells: Vec<String>,
    /// Characters seen as helping achieve this want
    pub helpers: Vec<ActantialActor>,
    /// Characters seen as opposing this want
    pub opponents: Vec<ActantialActor>,
    /// Who/what initiated or motivated this want
    pub sender: Option<ActantialActor>,
    /// Who benefits from this want being fulfilled
    pub receiver: Option<ActantialActor>,
}

impl WantContext {
    /// Create a new WantContext with minimal data
    pub fn new(want_id: impl Into<Uuid>, description: impl Into<String>, intensity: f32, priority: u32) -> Self {
        Self {
            want_id: want_id.into(),
            description: description.into(),
            intensity,
            priority,
            visibility: WantVisibility::Hidden,
            target: None,
            deflection_behavior: None,
            tells: Vec::new(),
            helpers: Vec::new(),
            opponents: Vec::new(),
            sender: None,
            receiver: None,
        }
    }

    /// Check if this is a hidden want
    pub fn is_hidden(&self) -> bool {
        matches!(self.visibility, WantVisibility::Hidden)
    }

    /// Get intensity description for LLM
    pub fn intensity_description(&self) -> &'static str {
        if self.intensity > 0.8 {
            "obsessive"
        } else if self.intensity > 0.6 {
            "strong"
        } else if self.intensity > 0.4 {
            "moderate"
        } else if self.intensity > 0.2 {
            "mild"
        } else {
            "passing"
        }
    }

    /// Get effective deflection behavior (uses default if not set)
    pub fn effective_deflection(&self) -> String {
        self.deflection_behavior.clone().unwrap_or_else(|| {
            if self.intensity > 0.8 {
                "Become visibly uncomfortable; firmly redirect conversation".to_string()
            } else if self.intensity > 0.5 {
                "Give a vague, non-committal response".to_string()
            } else {
                "Smoothly change the subject".to_string()
            }
        })
    }

    /// Format for LLM context (full detail for hidden wants, summary for known)
    pub fn to_llm_string(&self, include_secret_guidance: bool) -> String {
        let mut parts = Vec::new();

        // Core want description
        let target_str = self.target.as_ref()
            .map(|t| format!(" targeting {}", t.to_context_string()))
            .unwrap_or_default();
        
        parts.push(format!(
            "{} want (priority {}): {}{}",
            self.intensity_description(),
            self.priority,
            self.description,
            target_str
        ));

        // Actantial actors
        if !self.helpers.is_empty() {
            let helper_strs: Vec<_> = self.helpers.iter().map(|a| a.to_context_string()).collect();
            parts.push(format!("  Sees as helpers: {}", helper_strs.join(", ")));
        }
        if !self.opponents.is_empty() {
            let opponent_strs: Vec<_> = self.opponents.iter().map(|a| a.to_context_string()).collect();
            parts.push(format!("  Sees as opponents: {}", opponent_strs.join(", ")));
        }
        if let Some(sender) = &self.sender {
            parts.push(format!("  Sender/motivator: {}", sender.to_context_string()));
        }
        if let Some(receiver) = &self.receiver {
            parts.push(format!("  Beneficiary: {}", receiver.to_context_string()));
        }

        // Secret guidance (only included for hidden wants when requested)
        if include_secret_guidance && self.is_hidden() {
            parts.push(format!("  [SECRET - DEFLECTION]: {}", self.effective_deflection()));
            if !self.tells.is_empty() {
                parts.push(format!("  [SECRET - TELLS]: {}", self.tells.join("; ")));
            }
        }

        parts.join("\n")
    }
}

// =============================================================================
// Social View Summary
// =============================================================================

/// Summary of a character's social views aggregated across all wants
///
/// This provides a high-level view of who the character considers
/// allies vs enemies, collapsing the per-want details.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SocialViewSummary {
    /// Characters seen as allies (target, name, reasons from all wants)
    pub allies: Vec<(ActantialTarget, String, Vec<String>)>,
    /// Characters seen as enemies (target, name, reasons from all wants)
    pub enemies: Vec<(ActantialTarget, String, Vec<String>)>,
}

impl SocialViewSummary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an ally with a reason
    pub fn add_ally(&mut self, target: ActantialTarget, name: String, reason: String) {
        if let Some(existing) = self.allies.iter_mut().find(|(t, _, _)| t == &target) {
            existing.2.push(reason);
        } else {
            self.allies.push((target, name, vec![reason]));
        }
    }

    /// Add an enemy with a reason
    pub fn add_enemy(&mut self, target: ActantialTarget, name: String, reason: String) {
        if let Some(existing) = self.enemies.iter_mut().find(|(t, _, _)| t == &target) {
            existing.2.push(reason);
        } else {
            self.enemies.push((target, name, vec![reason]));
        }
    }

    /// Check if a target is considered an ally
    pub fn is_ally(&self, target: &ActantialTarget) -> bool {
        self.allies.iter().any(|(t, _, _)| t == target)
    }

    /// Check if a target is considered an enemy
    pub fn is_enemy(&self, target: &ActantialTarget) -> bool {
        self.enemies.iter().any(|(t, _, _)| t == target)
    }

    /// Format for LLM context
    pub fn to_llm_string(&self) -> String {
        let mut parts = Vec::new();

        if !self.allies.is_empty() {
            let ally_strs: Vec<_> = self.allies.iter()
                .map(|(_, name, reasons)| format!("{} ({})", name, reasons.join("; ")))
                .collect();
            parts.push(format!("Allies: {}", ally_strs.join(", ")));
        }

        if !self.enemies.is_empty() {
            let enemy_strs: Vec<_> = self.enemies.iter()
                .map(|(_, name, reasons)| format!("{} ({})", name, reasons.join("; ")))
                .collect();
            parts.push(format!("Enemies: {}", enemy_strs.join(", ")));
        }

        if parts.is_empty() {
            "No strong social alignments".to_string()
        } else {
            parts.join("\n")
        }
    }
}

// =============================================================================
// Actantial Context (complete character context)
// =============================================================================

/// Complete actantial context for a character
///
/// This is the top-level structure that gets passed to the LLM
/// to inform roleplay of an NPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActantialContext {
    /// The character's ID
    pub character_id: Uuid,
    /// The character's name
    pub character_name: String,
    /// All wants with their full context
    pub wants: Vec<WantContext>,
    /// Aggregated social views
    pub social_views: SocialViewSummary,
}

impl ActantialContext {
    pub fn new(character_id: impl Into<Uuid>, character_name: impl Into<String>) -> Self {
        Self {
            character_id: character_id.into(),
            character_name: character_name.into(),
            wants: Vec::new(),
            social_views: SocialViewSummary::new(),
        }
    }

    /// Get the primary want (priority 1)
    pub fn primary_want(&self) -> Option<&WantContext> {
        self.wants.iter().find(|w| w.priority == 1)
    }

    /// Get known wants (visible to player)
    pub fn known_wants(&self) -> Vec<&WantContext> {
        self.wants.iter().filter(|w| w.visibility.is_known()).collect()
    }

    /// Get hidden wants
    pub fn hidden_wants(&self) -> Vec<&WantContext> {
        self.wants.iter().filter(|w| w.is_hidden()).collect()
    }

    /// Format for LLM context
    ///
    /// If `include_secrets` is true, includes hidden want guidance (tells, deflection).
    /// The LLM always sees the hidden wants, but the guidance helps it roleplay them.
    pub fn to_llm_string(&self, include_secrets: bool) -> String {
        let mut sections = Vec::new();

        // Wants section
        if !self.wants.is_empty() {
            let mut wants_section = format!("=== {}'s Motivations ===\n", self.character_name);
            for want in &self.wants {
                wants_section.push_str(&want.to_llm_string(include_secrets));
                wants_section.push('\n');
            }
            sections.push(wants_section);
        }

        // Social views section
        let social_str = self.social_views.to_llm_string();
        if social_str != "No strong social alignments" {
            sections.push(format!("=== Social Views ===\n{}", social_str));
        }

        if sections.is_empty() {
            format!("{} has no defined motivations or social views.", self.character_name)
        } else {
            sections.join("\n\n")
        }
    }
}

// =============================================================================
// LLM-Ready Context (minimal format)
// =============================================================================

/// Minimal actantial context for LLM token efficiency
///
/// This is a more compact representation when token budget is limited.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActantialLLMContext {
    /// Primary motivation summary
    pub primary_motivation: Option<String>,
    /// List of known motivations (what player knows)
    pub known_motivations: Vec<String>,
    /// Secret motivations with behavioral guidance
    pub secret_motivations: Vec<SecretMotivationContext>,
    /// Social alignment summary
    pub social_summary: String,
}

/// Compact representation of a secret motivation for LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMotivationContext {
    /// What the character secretly wants
    pub want: String,
    /// How to deflect when probed
    pub deflection: String,
    /// Subtle behavioral tells
    pub tells: Vec<String>,
}

impl ActantialLLMContext {
    /// Build from full ActantialContext
    pub fn from_context(ctx: &ActantialContext) -> Self {
        let primary_motivation = ctx.primary_want().map(|w| {
            let target = w.target.as_ref()
                .map(|t| format!(" (targeting {})", t.name()))
                .unwrap_or_default();
            format!("{}{}", w.description, target)
        });

        let known_motivations: Vec<String> = ctx.known_wants()
            .iter()
            .map(|w| w.description.clone())
            .collect();

        let secret_motivations: Vec<SecretMotivationContext> = ctx.hidden_wants()
            .iter()
            .map(|w| SecretMotivationContext {
                want: w.description.clone(),
                deflection: w.effective_deflection(),
                tells: w.tells.clone(),
            })
            .collect();

        let social_summary = ctx.social_views.to_llm_string();

        Self {
            primary_motivation,
            known_motivations,
            secret_motivations,
            social_summary,
        }
    }

    /// Format as compact LLM string
    pub fn to_compact_string(&self) -> String {
        let mut lines = Vec::new();

        if let Some(primary) = &self.primary_motivation {
            lines.push(format!("Primary motivation: {}", primary));
        }

        if !self.known_motivations.is_empty() {
            lines.push(format!("Known goals: {}", self.known_motivations.join(", ")));
        }

        for secret in &self.secret_motivations {
            lines.push(format!(
                "[SECRET] Wants: {} | Deflect: {} | Tells: {}",
                secret.want,
                secret.deflection,
                if secret.tells.is_empty() { "none".to_string() } else { secret.tells.join("; ") }
            ));
        }

        lines.push(self.social_summary.clone());

        lines.join("\n")
    }
}

// =============================================================================
// Conversions to LLM Context Types
// =============================================================================

use super::llm_context::{
    ActantialActorEntry, MotivationEntry, MotivationsContext, SecretMotivationEntry,
    SocialRelationEntry, SocialStanceContext,
};

impl ActantialContext {
    /// Convert to LLM-ready MotivationsContext
    pub fn to_motivations_context(&self) -> MotivationsContext {
        let mut known = Vec::new();
        let mut suspected = Vec::new();
        let mut secret = Vec::new();

        for want in &self.wants {
            match want.visibility {
                WantVisibility::Known => {
                    known.push(want.to_motivation_entry());
                }
                WantVisibility::Suspected => {
                    suspected.push(want.to_motivation_entry());
                }
                WantVisibility::Hidden => {
                    secret.push(want.to_secret_motivation_entry());
                }
            }
        }

        MotivationsContext { known, suspected, secret }
    }

    /// Convert to LLM-ready SocialStanceContext
    pub fn to_social_stance_context(&self) -> SocialStanceContext {
        let allies = self.social_views.allies.iter()
            .map(|(target, name, reasons)| SocialRelationEntry {
                name: name.clone(),
                character_type: target.actor_type(),
                reasons: reasons.clone(),
            })
            .collect();

        let enemies = self.social_views.enemies.iter()
            .map(|(target, name, reasons)| SocialRelationEntry {
                name: name.clone(),
                character_type: target.actor_type(),
                reasons: reasons.clone(),
            })
            .collect();

        SocialStanceContext { allies, enemies }
    }
}

impl WantContext {
    /// Convert to a MotivationEntry (for Known/Suspected wants)
    fn to_motivation_entry(&self) -> MotivationEntry {
        MotivationEntry {
            description: self.description.clone(),
            priority: self.priority,
            intensity: self.intensity_description().to_string(),
            target: self.target.as_ref().map(|t| t.to_context_string()),
            helpers: self.helpers.iter().map(|a| a.to_actor_entry()).collect(),
            opponents: self.opponents.iter().map(|a| a.to_actor_entry()).collect(),
        }
    }

    /// Convert to a SecretMotivationEntry (for Hidden wants)
    fn to_secret_motivation_entry(&self) -> SecretMotivationEntry {
        SecretMotivationEntry {
            description: self.description.clone(),
            priority: self.priority,
            intensity: self.intensity_description().to_string(),
            target: self.target.as_ref().map(|t| t.to_context_string()),
            helpers: self.helpers.iter().map(|a| a.to_actor_entry()).collect(),
            opponents: self.opponents.iter().map(|a| a.to_actor_entry()).collect(),
            sender: self.sender.as_ref().map(|a| a.to_actor_entry()),
            receiver: self.receiver.as_ref().map(|a| a.to_actor_entry()),
            deflection_behavior: self.effective_deflection(),
            tells: self.tells.clone(),
        }
    }
}

impl ActantialActor {
    /// Convert to an ActantialActorEntry for LLM context
    fn to_actor_entry(&self) -> ActantialActorEntry {
        ActantialActorEntry {
            name: self.name.clone(),
            actor_type: self.target.actor_type(),
            reason: self.reason.clone(),
        }
    }
}
