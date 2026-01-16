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

use crate::entities::WantVisibility;
use crate::{CharacterId, GoalId, ItemId, PlayerCharacterId, WantId};

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
/// Uses typed IDs (CharacterId, PlayerCharacterId) for type safety.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActantialTarget {
    /// An NPC (Character)
    Npc(CharacterId),
    /// A Player Character
    Pc(PlayerCharacterId),
}

impl ActantialTarget {
    /// Create from a CharacterId (NPC)
    pub fn npc(id: CharacterId) -> Self {
        ActantialTarget::Npc(id)
    }

    /// Create from a PlayerCharacterId
    pub fn pc(id: PlayerCharacterId) -> Self {
        ActantialTarget::Pc(id)
    }

    /// Get the CharacterId if this is an NPC target
    pub fn as_character_id(&self) -> Option<CharacterId> {
        match self {
            ActantialTarget::Npc(id) => Some(*id),
            ActantialTarget::Pc(_) => None,
        }
    }

    /// Get the PlayerCharacterId if this is a PC target
    pub fn as_player_character_id(&self) -> Option<PlayerCharacterId> {
        match self {
            ActantialTarget::Npc(_) => None,
            ActantialTarget::Pc(id) => Some(*id),
        }
    }

    /// Get the ID as a string for generic operations (e.g., serialization)
    pub fn id_string(&self) -> String {
        match self {
            ActantialTarget::Npc(id) => id.to_string(),
            ActantialTarget::Pc(id) => id.to_string(),
        }
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
    Character { id: CharacterId, name: String },
    /// Want targets an Item
    Item { id: ItemId, name: String },
    /// Want targets a Goal (abstract desire)
    Goal {
        id: GoalId,
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

    /// Get the CharacterId if this is a Character target
    pub fn as_character_id(&self) -> Option<CharacterId> {
        match self {
            WantTarget::Character { id, .. } => Some(*id),
            _ => None,
        }
    }

    /// Get the ItemId if this is an Item target
    pub fn as_item_id(&self) -> Option<ItemId> {
        match self {
            WantTarget::Item { id, .. } => Some(*id),
            _ => None,
        }
    }

    /// Get the GoalId if this is a Goal target
    pub fn as_goal_id(&self) -> Option<GoalId> {
        match self {
            WantTarget::Goal { id, .. } => Some(*id),
            _ => None,
        }
    }

    /// Get the ID as a string for generic operations (e.g., serialization)
    pub fn id_string(&self) -> String {
        match self {
            WantTarget::Character { id, .. } => id.to_string(),
            WantTarget::Item { id, .. } => id.to_string(),
            WantTarget::Goal { id, .. } => id.to_string(),
        }
    }

    /// Format for LLM context
    pub fn to_context_string(&self) -> String {
        match self {
            WantTarget::Character { name, .. } => format!("the character {}", name),
            WantTarget::Item { name, .. } => format!("the item {}", name),
            WantTarget::Goal {
                name, description, ..
            } => {
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
    target: ActantialTarget,
    /// The actor's name for display
    name: String,
    /// Why the character views them this way
    reason: String,
}

impl ActantialActor {
    pub fn new(
        target: ActantialTarget,
        name: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            target,
            name: name.into(),
            reason: reason.into(),
        }
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Get the target (NPC or PC)
    pub fn target(&self) -> &ActantialTarget {
        &self.target
    }

    /// Get the actor's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the reason the character views them this way
    pub fn reason(&self) -> &str {
        &self.reason
    }

    // -------------------------------------------------------------------------
    // Formatting
    // -------------------------------------------------------------------------

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
    want_id: WantId,
    /// Description of what the character wants
    description: String,
    /// Intensity (0.0 = mild interest, 1.0 = obsession)
    intensity: f32,
    /// Priority (1 = primary want)
    priority: u32,
    /// How much the player knows
    visibility: WantVisibility,
    /// The resolved target (if any)
    target: Option<WantTarget>,
    /// How to behave when probed about this want
    deflection_behavior: Option<String>,
    /// Behavioral tells that hint at this want
    tells: Vec<String>,
    /// Characters seen as helping achieve this want
    helpers: Vec<ActantialActor>,
    /// Characters seen as opposing this want
    opponents: Vec<ActantialActor>,
    /// Who/what initiated or motivated this want
    sender: Option<ActantialActor>,
    /// Who benefits from this want being fulfilled
    receiver: Option<ActantialActor>,
}

impl WantContext {
    /// Create a new WantContext with minimal data
    pub fn new(
        want_id: WantId,
        description: impl Into<String>,
        intensity: f32,
        priority: u32,
    ) -> Self {
        Self {
            want_id,
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

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Get the want's ID
    pub fn want_id(&self) -> WantId {
        self.want_id
    }

    /// Get the description of what the character wants
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the intensity (0.0 = mild interest, 1.0 = obsession)
    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    /// Get the priority (1 = primary want)
    pub fn priority(&self) -> u32 {
        self.priority
    }

    /// Get the visibility level
    pub fn visibility(&self) -> WantVisibility {
        self.visibility
    }

    /// Get the resolved target (if any)
    pub fn target(&self) -> Option<&WantTarget> {
        self.target.as_ref()
    }

    /// Get the deflection behavior (if any)
    pub fn deflection_behavior(&self) -> Option<&str> {
        self.deflection_behavior.as_deref()
    }

    /// Get the behavioral tells
    pub fn tells(&self) -> &[String] {
        &self.tells
    }

    /// Get the helpers
    pub fn helpers(&self) -> &[ActantialActor] {
        &self.helpers
    }

    /// Get the opponents
    pub fn opponents(&self) -> &[ActantialActor] {
        &self.opponents
    }

    /// Get the sender (who/what initiated this want)
    pub fn sender(&self) -> Option<&ActantialActor> {
        self.sender.as_ref()
    }

    /// Get the receiver (who benefits from fulfillment)
    pub fn receiver(&self) -> Option<&ActantialActor> {
        self.receiver.as_ref()
    }

    // -------------------------------------------------------------------------
    // Builder methods
    // -------------------------------------------------------------------------

    /// Set the visibility
    pub fn with_visibility(mut self, visibility: WantVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Set the target
    pub fn with_target(mut self, target: WantTarget) -> Self {
        self.target = Some(target);
        self
    }

    /// Set the deflection behavior
    pub fn with_deflection_behavior(mut self, behavior: impl Into<String>) -> Self {
        self.deflection_behavior = Some(behavior.into());
        self
    }

    /// Set the behavioral tells
    pub fn with_tells(mut self, tells: Vec<String>) -> Self {
        self.tells = tells;
        self
    }

    /// Set the helpers
    pub fn with_helpers(mut self, helpers: Vec<ActantialActor>) -> Self {
        self.helpers = helpers;
        self
    }

    /// Set the opponents
    pub fn with_opponents(mut self, opponents: Vec<ActantialActor>) -> Self {
        self.opponents = opponents;
        self
    }

    /// Set the sender
    pub fn with_sender(mut self, sender: ActantialActor) -> Self {
        self.sender = Some(sender);
        self
    }

    /// Set the receiver
    pub fn with_receiver(mut self, receiver: ActantialActor) -> Self {
        self.receiver = Some(receiver);
        self
    }

    // -------------------------------------------------------------------------
    // Query methods
    // -------------------------------------------------------------------------

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
        let target_str = self
            .target
            .as_ref()
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
            let opponent_strs: Vec<_> = self
                .opponents
                .iter()
                .map(|a| a.to_context_string())
                .collect();
            parts.push(format!("  Sees as opponents: {}", opponent_strs.join(", ")));
        }
        if let Some(sender) = &self.sender {
            parts.push(format!(
                "  Sender/motivator: {}",
                sender.to_context_string()
            ));
        }
        if let Some(receiver) = &self.receiver {
            parts.push(format!("  Beneficiary: {}", receiver.to_context_string()));
        }

        // Secret guidance (only included for hidden wants when requested)
        if include_secret_guidance && self.is_hidden() {
            parts.push(format!(
                "  [SECRET - DEFLECTION]: {}",
                self.effective_deflection()
            ));
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
    allies: Vec<(ActantialTarget, String, Vec<String>)>,
    /// Characters seen as enemies (target, name, reasons from all wants)
    enemies: Vec<(ActantialTarget, String, Vec<String>)>,
}

impl SocialViewSummary {
    pub fn new() -> Self {
        Self::default()
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Get the allies list
    pub fn allies(&self) -> &[(ActantialTarget, String, Vec<String>)] {
        &self.allies
    }

    /// Get the enemies list
    pub fn enemies(&self) -> &[(ActantialTarget, String, Vec<String>)] {
        &self.enemies
    }

    // -------------------------------------------------------------------------
    // Builder methods
    // -------------------------------------------------------------------------

    /// Add an ally with a reason (builder pattern)
    pub fn with_ally(mut self, target: ActantialTarget, name: String, reason: String) -> Self {
        if let Some(existing) = self.allies.iter_mut().find(|(t, _, _)| t == &target) {
            existing.2.push(reason);
        } else {
            self.allies.push((target, name, vec![reason]));
        }
        self
    }

    /// Add an enemy with a reason (builder pattern)
    pub fn with_enemy(mut self, target: ActantialTarget, name: String, reason: String) -> Self {
        if let Some(existing) = self.enemies.iter_mut().find(|(t, _, _)| t == &target) {
            existing.2.push(reason);
        } else {
            self.enemies.push((target, name, vec![reason]));
        }
        self
    }

    // -------------------------------------------------------------------------
    // Query methods
    // -------------------------------------------------------------------------

    /// Check if a target is considered an ally
    pub fn is_ally(&self, target: &ActantialTarget) -> bool {
        self.allies.iter().any(|(t, _, _)| t == target)
    }

    /// Check if a target is considered an enemy
    pub fn is_enemy(&self, target: &ActantialTarget) -> bool {
        self.enemies.iter().any(|(t, _, _)| t == target)
    }

    // -------------------------------------------------------------------------
    // Formatting
    // -------------------------------------------------------------------------

    /// Format for LLM context
    pub fn to_llm_string(&self) -> String {
        let mut parts = Vec::new();

        if !self.allies.is_empty() {
            let ally_strs: Vec<_> = self
                .allies
                .iter()
                .map(|(_, name, reasons)| format!("{} ({})", name, reasons.join("; ")))
                .collect();
            parts.push(format!("Allies: {}", ally_strs.join(", ")));
        }

        if !self.enemies.is_empty() {
            let enemy_strs: Vec<_> = self
                .enemies
                .iter()
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
    character_id: CharacterId,
    /// The character's name
    character_name: String,
    /// All wants with their full context
    wants: Vec<WantContext>,
    /// Aggregated social views
    social_views: SocialViewSummary,
}

impl ActantialContext {
    pub fn new(character_id: CharacterId, character_name: impl Into<String>) -> Self {
        Self {
            character_id,
            character_name: character_name.into(),
            wants: Vec::new(),
            social_views: SocialViewSummary::new(),
        }
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Get the character's ID
    pub fn character_id(&self) -> CharacterId {
        self.character_id
    }

    /// Get the character's name
    pub fn character_name(&self) -> &str {
        &self.character_name
    }

    /// Get all wants
    pub fn wants(&self) -> &[WantContext] {
        &self.wants
    }

    /// Get the social views summary
    pub fn social_views(&self) -> &SocialViewSummary {
        &self.social_views
    }

    // -------------------------------------------------------------------------
    // Builder methods
    // -------------------------------------------------------------------------

    /// Set the wants
    pub fn with_wants(mut self, wants: Vec<WantContext>) -> Self {
        self.wants = wants;
        self
    }

    /// Set the social views
    pub fn with_social_views(mut self, social_views: SocialViewSummary) -> Self {
        self.social_views = social_views;
        self
    }

    // -------------------------------------------------------------------------
    // Query methods
    // -------------------------------------------------------------------------

    /// Get the primary want (priority 1)
    pub fn primary_want(&self) -> Option<&WantContext> {
        self.wants.iter().find(|w| w.priority() == 1)
    }

    /// Get known wants (visible to player)
    pub fn known_wants(&self) -> Vec<&WantContext> {
        self.wants
            .iter()
            .filter(|w| w.visibility().is_known())
            .collect()
    }

    /// Get hidden wants
    pub fn hidden_wants(&self) -> Vec<&WantContext> {
        self.wants.iter().filter(|w| w.is_hidden()).collect()
    }

    // -------------------------------------------------------------------------
    // Formatting
    // -------------------------------------------------------------------------

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
            format!(
                "{} has no defined motivations or social views.",
                self.character_name
            )
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
    primary_motivation: Option<String>,
    /// List of known motivations (what player knows)
    known_motivations: Vec<String>,
    /// Secret motivations with behavioral guidance
    secret_motivations: Vec<SecretMotivationContext>,
    /// Social alignment summary
    social_summary: String,
}

/// Compact representation of a secret motivation for LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMotivationContext {
    /// What the character secretly wants
    want: String,
    /// How to deflect when probed
    deflection: String,
    /// Subtle behavioral tells
    tells: Vec<String>,
}

impl SecretMotivationContext {
    /// Create a new secret motivation context
    pub fn new(want: impl Into<String>, deflection: impl Into<String>, tells: Vec<String>) -> Self {
        Self {
            want: want.into(),
            deflection: deflection.into(),
            tells,
        }
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Get what the character secretly wants
    pub fn want(&self) -> &str {
        &self.want
    }

    /// Get the deflection behavior
    pub fn deflection(&self) -> &str {
        &self.deflection
    }

    /// Get the behavioral tells
    pub fn tells(&self) -> &[String] {
        &self.tells
    }
}

impl ActantialLLMContext {
    /// Create a new ActantialLLMContext
    pub fn new(
        primary_motivation: Option<String>,
        known_motivations: Vec<String>,
        secret_motivations: Vec<SecretMotivationContext>,
        social_summary: impl Into<String>,
    ) -> Self {
        Self {
            primary_motivation,
            known_motivations,
            secret_motivations,
            social_summary: social_summary.into(),
        }
    }

    /// Build from full ActantialContext
    pub fn from_context(ctx: &ActantialContext) -> Self {
        let primary_motivation = ctx.primary_want().map(|w| {
            let target = w
                .target()
                .map(|t| format!(" (targeting {})", t.name()))
                .unwrap_or_default();
            format!("{}{}", w.description(), target)
        });

        let known_motivations: Vec<String> = ctx
            .known_wants()
            .iter()
            .map(|w| w.description().to_string())
            .collect();

        let secret_motivations: Vec<SecretMotivationContext> = ctx
            .hidden_wants()
            .iter()
            .map(|w| {
                SecretMotivationContext::new(
                    w.description(),
                    w.effective_deflection(),
                    w.tells().to_vec(),
                )
            })
            .collect();

        let social_summary = ctx.social_views().to_llm_string();

        Self {
            primary_motivation,
            known_motivations,
            secret_motivations,
            social_summary,
        }
    }

    // -------------------------------------------------------------------------
    // Accessors
    // -------------------------------------------------------------------------

    /// Get the primary motivation summary
    pub fn primary_motivation(&self) -> Option<&str> {
        self.primary_motivation.as_deref()
    }

    /// Get the known motivations
    pub fn known_motivations(&self) -> &[String] {
        &self.known_motivations
    }

    /// Get the secret motivations
    pub fn secret_motivations(&self) -> &[SecretMotivationContext] {
        &self.secret_motivations
    }

    /// Get the social summary
    pub fn social_summary(&self) -> &str {
        &self.social_summary
    }

    // -------------------------------------------------------------------------
    // Formatting
    // -------------------------------------------------------------------------

    /// Format as compact LLM string
    pub fn to_compact_string(&self) -> String {
        let mut lines = Vec::new();

        if let Some(primary) = &self.primary_motivation {
            lines.push(format!("Primary motivation: {}", primary));
        }

        if !self.known_motivations.is_empty() {
            lines.push(format!(
                "Known goals: {}",
                self.known_motivations.join(", ")
            ));
        }

        for secret in &self.secret_motivations {
            lines.push(format!(
                "[SECRET] Wants: {} | Deflect: {} | Tells: {}",
                secret.want(),
                secret.deflection(),
                if secret.tells().is_empty() {
                    "none".to_string()
                } else {
                    secret.tells().join("; ")
                }
            ));
        }

        lines.push(self.social_summary.clone());

        lines.join("\n")
    }
}
