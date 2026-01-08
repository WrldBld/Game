//! DM Control Port - Handles Dungeon Master operations
//!
//! This trait defines operations that only the DM can perform,
//! such as scene management, approval decisions, and NPC control.

use crate::outbound::GameConnectionPort;
use crate::session_types::{
    AdHocOutcomes, ApprovalDecision, ApprovedNpcInfo, ChallengeOutcomeDecision, DirectorialContext,
};

/// Port for Dungeon Master control operations
///
/// Handles all DM-specific actions including scene management,
/// approval workflows, challenge control, and NPC management.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
pub trait DmControlPort: Send + Sync {
    /// Request a scene change (DM only)
    fn request_scene_change(&self, scene_id: &str) -> anyhow::Result<()>;

    /// Send a directorial context update (DM only)
    fn send_directorial_update(&self, context: DirectorialContext) -> anyhow::Result<()>;

    /// Send an approval decision (DM only)
    fn send_approval_decision(
        &self,
        request_id: &str,
        decision: ApprovalDecision,
    ) -> anyhow::Result<()>;

    /// Send a challenge outcome decision (DM only)
    fn send_challenge_outcome_decision(
        &self,
        resolution_id: &str,
        decision: ChallengeOutcomeDecision,
    ) -> anyhow::Result<()>;

    /// Trigger a challenge (DM only)
    fn trigger_challenge(
        &self,
        challenge_id: &str,
        target_character_id: &str,
    ) -> anyhow::Result<()>;

    /// Create an ad-hoc challenge (DM only)
    fn create_adhoc_challenge(
        &self,
        challenge_name: &str,
        skill_name: &str,
        difficulty: &str,
        target_pc_id: &str,
        outcomes: AdHocOutcomes,
    ) -> anyhow::Result<()>;

    /// Set NPC disposition toward a PC (DM only)
    ///
    /// # Arguments
    /// * `npc_id` - The NPC's ID
    /// * `pc_id` - The PC's ID
    /// * `disposition` - The disposition value
    /// * `reason` - Optional reason for the disposition change
    fn set_npc_disposition(
        &self,
        npc_id: &str,
        pc_id: &str,
        disposition: &str,
        reason: Option<String>,
    ) -> anyhow::Result<()>;

    /// Set NPC relationship toward a PC (DM only)
    fn set_npc_relationship(
        &self,
        npc_id: &str,
        pc_id: &str,
        relationship: &str,
    ) -> anyhow::Result<()>;

    /// Request NPC dispositions for a PC (fetches current disposition data)
    fn get_npc_dispositions(&self, pc_id: &str) -> anyhow::Result<()>;

    /// Send a staging approval response (DM only)
    fn send_staging_approval(
        &self,
        request_id: &str,
        approved_npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
        source: &str,
    ) -> anyhow::Result<()>;

    /// Request regeneration of staging suggestions (DM only)
    fn request_staging_regenerate(&self, request_id: &str, guidance: &str) -> anyhow::Result<()>;

    /// Pre-stage a region before player arrival (DM only)
    fn pre_stage_region(
        &self,
        region_id: &str,
        npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
    ) -> anyhow::Result<()>;
}

// =============================================================================
// Blanket implementation: GameConnectionPort -> DmControlPort
// =============================================================================

/// Blanket implementation allowing any `GameConnectionPort` to be used as `DmControlPort`
impl<T: GameConnectionPort + ?Sized> DmControlPort for T {
    fn request_scene_change(&self, scene_id: &str) -> anyhow::Result<()> {
        GameConnectionPort::request_scene_change(self, scene_id)
    }

    fn send_directorial_update(&self, context: DirectorialContext) -> anyhow::Result<()> {
        GameConnectionPort::send_directorial_update(self, context)
    }

    fn send_approval_decision(
        &self,
        request_id: &str,
        decision: ApprovalDecision,
    ) -> anyhow::Result<()> {
        GameConnectionPort::send_approval_decision(self, request_id, decision)
    }

    fn send_challenge_outcome_decision(
        &self,
        resolution_id: &str,
        decision: ChallengeOutcomeDecision,
    ) -> anyhow::Result<()> {
        GameConnectionPort::send_challenge_outcome_decision(self, resolution_id, decision)
    }

    fn trigger_challenge(
        &self,
        challenge_id: &str,
        target_character_id: &str,
    ) -> anyhow::Result<()> {
        GameConnectionPort::trigger_challenge(self, challenge_id, target_character_id)
    }

    fn create_adhoc_challenge(
        &self,
        challenge_name: &str,
        skill_name: &str,
        difficulty: &str,
        target_pc_id: &str,
        outcomes: AdHocOutcomes,
    ) -> anyhow::Result<()> {
        GameConnectionPort::create_adhoc_challenge(
            self,
            challenge_name,
            skill_name,
            difficulty,
            target_pc_id,
            outcomes,
        )
    }

    fn set_npc_disposition(
        &self,
        npc_id: &str,
        pc_id: &str,
        disposition: &str,
        reason: Option<String>,
    ) -> anyhow::Result<()> {
        GameConnectionPort::set_npc_disposition(self, npc_id, pc_id, disposition, reason.as_deref())
    }

    fn set_npc_relationship(
        &self,
        npc_id: &str,
        pc_id: &str,
        relationship: &str,
    ) -> anyhow::Result<()> {
        GameConnectionPort::set_npc_relationship(self, npc_id, pc_id, relationship)
    }

    fn get_npc_dispositions(&self, pc_id: &str) -> anyhow::Result<()> {
        GameConnectionPort::get_npc_dispositions(self, pc_id)
    }

    fn send_staging_approval(
        &self,
        request_id: &str,
        approved_npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
        source: &str,
    ) -> anyhow::Result<()> {
        GameConnectionPort::send_staging_approval(
            self,
            request_id,
            approved_npcs,
            ttl_hours,
            source,
        )
    }

    fn request_staging_regenerate(&self, request_id: &str, guidance: &str) -> anyhow::Result<()> {
        GameConnectionPort::request_staging_regenerate(self, request_id, guidance)
    }

    fn pre_stage_region(
        &self,
        region_id: &str,
        npcs: Vec<ApprovedNpcInfo>,
        ttl_hours: i32,
    ) -> anyhow::Result<()> {
        GameConnectionPort::pre_stage_region(self, region_id, npcs, ttl_hours)
    }
}
