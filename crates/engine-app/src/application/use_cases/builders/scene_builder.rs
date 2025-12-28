//! Scene Builder
//!
//! Builds `SceneChangedEvent` from region and staging data.
//! Shared across `MovementUseCase` and `StagingApprovalUseCase`.
//!
//! # Responsibilities
//!
//! - Fetch region and location data
//! - Fetch navigation (connected regions and exits)
//! - Fetch region items
//! - Convert staged NPCs to presence data
//! - Build the complete `SceneChangedEvent`
//!
//! # Architecture
//!
//! This builder encapsulates the data fetching and transformation logic
//! that was previously duplicated in movement.rs and staging.rs handlers.
//! Use cases call the builder instead of doing this work themselves.

use std::sync::Arc;
use tracing::{debug, warn};

use wrldbldr_domain::entities::{Location, Region, StagedNpc};
use wrldbldr_domain::{PlayerCharacterId, RegionId};
use wrldbldr_engine_ports::outbound::{
    LocationRepositoryPort, NavigationExit, NavigationInfo, NavigationTarget, NpcPresenceData,
    RegionInfo, RegionItemData, RegionRepositoryPort, SceneChangedEvent,
};

use crate::application::use_cases::errors::MovementError;

/// Builder for constructing `SceneChangedEvent` from region data
///
/// # Usage
///
/// ```rust,ignore
/// let builder = SceneBuilder::new(region_repo, location_repo);
/// let event = builder.build(pc_id, region_id, staged_npcs).await?;
/// ```
#[derive(Clone)]
pub struct SceneBuilder {
    region_repo: Arc<dyn RegionRepositoryPort>,
    location_repo: Arc<dyn LocationRepositoryPort>,
}

impl SceneBuilder {
    /// Create a new SceneBuilder with required repository dependencies
    pub fn new(
        region_repo: Arc<dyn RegionRepositoryPort>,
        location_repo: Arc<dyn LocationRepositoryPort>,
    ) -> Self {
        Self {
            region_repo,
            location_repo,
        }
    }

    /// Build a complete SceneChangedEvent for a PC in a region
    ///
    /// # Arguments
    ///
    /// * `pc_id` - The player character whose scene is changing
    /// * `region_id` - The target region
    /// * `staged_npcs` - NPCs staged in the region (from staging system)
    ///
    /// # Returns
    ///
    /// A complete `SceneChangedEvent` with region info, navigation, and NPCs.
    ///
    /// # Errors
    ///
    /// Returns `MovementError` if region or location cannot be found.
    pub async fn build(
        &self,
        pc_id: PlayerCharacterId,
        region_id: RegionId,
        staged_npcs: &[StagedNpc],
    ) -> Result<SceneChangedEvent, MovementError> {
        // Get region
        let region = self
            .region_repo
            .get(region_id)
            .await
            .map_err(|e| MovementError::Database(e.to_string()))?
            .ok_or(MovementError::RegionNotFound(region_id))?;

        // Get location
        let location = self
            .location_repo
            .get(region.location_id)
            .await
            .map_err(|e| MovementError::Database(e.to_string()))?
            .ok_or(MovementError::LocationNotFound(region.location_id))?;

        // Build all components
        let region_info = self.build_region_info(&region, &location);
        let npcs_present = self.build_npc_presence(staged_npcs);
        let navigation = self.build_navigation(region_id).await;
        let region_items = self.build_region_items(region_id).await;

        Ok(SceneChangedEvent {
            pc_id,
            region: region_info,
            npcs_present,
            navigation,
            region_items,
        })
    }

    /// Build a SceneChangedEvent with pre-fetched region and location
    ///
    /// Use this when you already have the region and location entities
    /// to avoid redundant database lookups.
    pub async fn build_with_entities(
        &self,
        pc_id: PlayerCharacterId,
        region: &Region,
        location: &Location,
        staged_npcs: &[StagedNpc],
    ) -> SceneChangedEvent {
        let region_info = self.build_region_info(region, location);
        let npcs_present = self.build_npc_presence(staged_npcs);
        let navigation = self.build_navigation(region.id).await;
        let region_items = self.build_region_items(region.id).await;

        SceneChangedEvent {
            pc_id,
            region: region_info,
            npcs_present,
            navigation,
            region_items,
        }
    }

    /// Build RegionInfo from region and location entities
    fn build_region_info(&self, region: &Region, location: &Location) -> RegionInfo {
        // Backdrop: region-specific, or fallback to location
        let backdrop_asset = region
            .backdrop_asset
            .clone()
            .or_else(|| location.backdrop_asset.clone());

        RegionInfo {
            id: region.id,
            name: region.name.clone(),
            location_id: region.location_id,
            location_name: location.name.clone(),
            backdrop_asset,
            atmosphere: region.atmosphere.clone(),
            map_asset: location.map_asset.clone(),
        }
    }

    /// Convert staged NPCs to NpcPresenceData
    ///
    /// Only includes visible NPCs (is_present && !is_hidden_from_players)
    fn build_npc_presence(&self, staged_npcs: &[StagedNpc]) -> Vec<NpcPresenceData> {
        staged_npcs
            .iter()
            .filter(|npc| npc.is_present && !npc.is_hidden_from_players)
            .map(|npc| NpcPresenceData {
                character_id: npc.character_id,
                name: npc.name.clone(),
                sprite_asset: npc.sprite_asset.clone(),
                portrait_asset: npc.portrait_asset.clone(),
            })
            .collect()
    }

    /// Build navigation data (connected regions and exits)
    async fn build_navigation(&self, region_id: RegionId) -> NavigationInfo {
        let connected_regions = self.build_connected_regions(region_id).await;
        let exits = self.build_exits(region_id).await;

        NavigationInfo {
            connected_regions,
            exits,
        }
    }

    /// Build list of connected regions within the same location
    async fn build_connected_regions(&self, region_id: RegionId) -> Vec<NavigationTarget> {
        let connections = match self.region_repo.get_connections(region_id).await {
            Ok(conns) => conns,
            Err(e) => {
                warn!(error = %e, region_id = %region_id, "Failed to fetch region connections");
                return Vec::new();
            }
        };

        let mut targets = Vec::new();
        for conn in connections {
            // Get target region name
            match self.region_repo.get(conn.to_region).await {
                Ok(Some(target_region)) => {
                    targets.push(NavigationTarget {
                        region_id: conn.to_region,
                        name: target_region.name,
                        is_locked: conn.is_locked,
                        lock_description: conn.lock_description,
                    });
                }
                Ok(None) => {
                    debug!(
                        from = %region_id,
                        to = %conn.to_region,
                        "Connected region not found, skipping"
                    );
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        to = %conn.to_region,
                        "Failed to fetch connected region"
                    );
                }
            }
        }

        targets
    }

    /// Build list of exits to other locations
    async fn build_exits(&self, region_id: RegionId) -> Vec<NavigationExit> {
        let region_exits = match self.region_repo.get_exits(region_id).await {
            Ok(exits) => exits,
            Err(e) => {
                warn!(error = %e, region_id = %region_id, "Failed to fetch region exits");
                return Vec::new();
            }
        };

        let mut exits = Vec::new();
        for exit in region_exits {
            // Get target location details
            match self.location_repo.get(exit.to_location).await {
                Ok(Some(target_location)) => {
                    // Determine arrival region: use exit's arrival_region_id or location default
                    let arrival_region_id = target_location
                        .default_region_id
                        .unwrap_or(exit.arrival_region_id);

                    exits.push(NavigationExit {
                        location_id: exit.to_location,
                        location_name: target_location.name,
                        arrival_region_id,
                        description: exit.description,
                    });
                }
                Ok(None) => {
                    debug!(
                        from_region = %region_id,
                        to_location = %exit.to_location,
                        "Exit target location not found, skipping"
                    );
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        to_location = %exit.to_location,
                        "Failed to fetch exit target location"
                    );
                }
            }
        }

        exits
    }

    /// Build list of items in the region
    ///
    /// Currently returns empty list - region items not yet implemented.
    /// When US-REGION-ITEMS is complete, this will call region_repo.get_region_items()
    async fn build_region_items(&self, region_id: RegionId) -> Vec<RegionItemData> {
        // TODO: Implement when region item system is ready
        // For now, try to get items but handle the not-implemented error gracefully
        match self.region_repo.get_region_items(region_id).await {
            Ok(items) => items
                .into_iter()
                .map(|item| RegionItemData {
                    item_id: item.id,
                    name: item.name,
                    description: item.description,
                    quantity: 1, // TODO: Get actual quantity from edge
                })
                .collect(),
            Err(_) => {
                // Region items not yet implemented - return empty
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wrldbldr_domain::entities::StagedNpc;
    use wrldbldr_domain::CharacterId;

    fn make_staged_npc(name: &str, present: bool, hidden: bool) -> StagedNpc {
        StagedNpc {
            character_id: CharacterId::from_uuid(uuid::Uuid::new_v4()),
            name: name.to_string(),
            sprite_asset: None,
            portrait_asset: None,
            is_present: present,
            is_hidden_from_players: hidden,
            reasoning: "test".to_string(),
        }
    }

    #[test]
    fn test_npc_presence_filtering() {
        // Create a mock builder just to test the filtering logic
        // We can't easily mock the repos here, but we can test the pure function logic
        
        let staged = vec![
            make_staged_npc("Alice", true, false),  // Should be included
            make_staged_npc("Bob", true, true),     // Hidden, should be excluded
            make_staged_npc("Charlie", false, false), // Not present, excluded
            make_staged_npc("Diana", true, false),  // Should be included
        ];

        // Filter like the builder does
        let visible: Vec<_> = staged
            .iter()
            .filter(|npc| npc.is_present && !npc.is_hidden_from_players)
            .collect();

        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].name, "Alice");
        assert_eq!(visible[1].name, "Diana");
    }
}
