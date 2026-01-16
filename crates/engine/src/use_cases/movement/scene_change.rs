use std::sync::Arc;

use wrldbldr_domain::{LocationId, Region, RegionId, StagedNpc};

use crate::infrastructure::ports::RepoError;
use crate::repositories::inventory::Inventory;
use crate::repositories::location::Location;

// =============================================================================
// Domain Types (for use case output)
// =============================================================================

/// Region data for scene changes (domain representation).
#[derive(Debug, Clone)]
pub struct RegionInfo {
    pub id: String,
    pub name: String,
    pub location_id: String,
    pub location_name: String,
    pub backdrop_asset: Option<String>,
    pub atmosphere: Option<String>,
    pub map_asset: Option<String>,
}

/// NPC presence info for scene display (domain representation).
#[derive(Debug, Clone)]
pub struct NpcPresenceInfo {
    pub character_id: String,
    pub name: String,
    pub sprite_asset: Option<String>,
    pub portrait_asset: Option<String>,
}

/// Navigation options from current region (domain representation).
#[derive(Debug, Clone)]
pub struct NavigationInfo {
    pub connected_regions: Vec<NavigationTargetInfo>,
    pub exits: Vec<NavigationExitInfo>,
}

/// A navigation target (region within same location) (domain representation).
#[derive(Debug, Clone)]
pub struct NavigationTargetInfo {
    pub region_id: String,
    pub name: String,
    pub is_locked: bool,
    pub lock_description: Option<String>,
}

/// An exit to another location (domain representation).
#[derive(Debug, Clone)]
pub struct NavigationExitInfo {
    pub location_id: String,
    pub location_name: String,
    pub arrival_region_id: String,
    pub description: Option<String>,
}

/// Item data for region display (domain representation).
#[derive(Debug, Clone)]
pub struct RegionItemInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub item_type: Option<String>,
}

/// Errors that can occur when building scene change data.
#[derive(Debug, thiserror::Error)]
pub enum SceneChangeError {
    #[error("Location {0} not found")]
    LocationNotFound(LocationId),
    #[error("Region {0} not found")]
    RegionNotFound(RegionId),
    #[error("Navigation exit to location {location_id} skipped: {reason}")]
    ExitSkipped {
        location_id: LocationId,
        reason: String,
    },
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}

pub struct SceneChangeBuilder {
    location: Arc<Location>,
    inventory: Arc<Inventory>,
}

impl SceneChangeBuilder {
    pub fn new(location: Arc<Location>, inventory: Arc<Inventory>) -> Self {
        Self {
            location,
            inventory,
        }
    }

    pub async fn build_scene_change(
        &self,
        region: &Region,
        npcs: Vec<StagedNpc>,
        include_hidden_npcs: bool,
    ) -> Result<SceneChangeData, SceneChangeError> {
        let location = self
            .location
            .get(region.location_id())
            .await?
            .ok_or(SceneChangeError::LocationNotFound(region.location_id()))?;

        let region_data = RegionInfo {
            id: region.id().to_string(),
            name: region.name().to_string(),
            location_id: region.location_id().to_string(),
            location_name: location.name().to_string(),
            backdrop_asset: region.backdrop_asset().map(|s| s.to_string()),
            atmosphere: region.atmosphere().map(|s| s.to_string()),
            map_asset: None,
        };

        let npcs_present: Vec<NpcPresenceInfo> = npcs
            .into_iter()
            .filter(|npc| include_hidden_npcs || npc.is_visible_to_players())
            .map(|npc| NpcPresenceInfo {
                character_id: npc.character_id.to_string(),
                name: npc.name.clone(),
                sprite_asset: npc.sprite_asset.clone(),
                portrait_asset: npc.portrait_asset.clone(),
            })
            .collect();

        let navigation = self.build_navigation_data(region.id()).await?;
        let region_items = self.build_region_items(region.id()).await;

        Ok(SceneChangeData {
            region: region_data,
            npcs_present,
            navigation,
            region_items,
        })
    }

    async fn build_navigation_data(
        &self,
        region_id: RegionId,
    ) -> Result<NavigationInfo, SceneChangeError> {
        let connections = self.location.get_connections(region_id).await?;

        let mut connected_regions = Vec::new();
        for connection in connections {
            let target_region = self
                .location
                .get_region(connection.to_region)
                .await?
                .ok_or(SceneChangeError::RegionNotFound(connection.to_region))?;

            connected_regions.push(NavigationTargetInfo {
                region_id: connection.to_region.to_string(),
                name: target_region.name().to_string(),
                is_locked: connection.is_locked,
                lock_description: connection.lock_description.map(|s| s.to_string()),
            });
        }

        let exits_result = self.location.get_exits(region_id).await?;

        // Fail hard if any exits were skipped due to data integrity issues
        if let Some(skipped) = exits_result.skipped.first() {
            return Err(SceneChangeError::ExitSkipped {
                location_id: skipped.to_location,
                reason: skipped.reason.clone(),
            });
        }

        let exits = exits_result
            .exits
            .into_iter()
            .map(|exit| NavigationExitInfo {
                location_id: exit.location_id.to_string(),
                location_name: exit.location_name,
                arrival_region_id: exit.arrival_region_id.to_string(),
                description: exit.description,
            })
            .collect();

        Ok(NavigationInfo {
            connected_regions,
            exits,
        })
    }

    async fn build_region_items(&self, region_id: RegionId) -> Vec<RegionItemInfo> {
        // Region items are optional/non-critical, keep graceful degradation here
        match self.inventory.list_in_region(region_id).await {
            Ok(items) => items
                .into_iter()
                .map(|item| RegionItemInfo {
                    id: item.id().to_string(),
                    name: item.name().to_string(),
                    description: item.description().map(|s| s.to_string()),
                    item_type: item.item_type().map(|s| s.to_string()),
                })
                .collect(),
            Err(e) => {
                tracing::warn!(error = %e, region_id = %region_id, "Failed to fetch region items");
                vec![]
            }
        }
    }
}

/// Scene change data returned by the use case (domain representation).
pub struct SceneChangeData {
    pub region: RegionInfo,
    pub npcs_present: Vec<NpcPresenceInfo>,
    pub navigation: NavigationInfo,
    pub region_items: Vec<RegionItemInfo>,
}
