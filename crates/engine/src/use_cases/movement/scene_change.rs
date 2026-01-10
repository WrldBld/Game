use std::sync::Arc;

use wrldbldr_domain::{Region, RegionId, StagedNpc};
use wrldbldr_protocol::{NavigationData, NpcPresenceData, RegionData, RegionItemData};

use crate::entities::{Inventory, Location};

pub struct SceneChangeBuilder {
    location: Arc<Location>,
    inventory: Arc<Inventory>,
}

impl SceneChangeBuilder {
    pub fn new(location: Arc<Location>, inventory: Arc<Inventory>) -> Self {
        Self { location, inventory }
    }

    pub async fn build_scene_change(
        &self,
        region: &Region,
        npcs: Vec<StagedNpc>,
        include_hidden_npcs: bool,
    ) -> SceneChangeData {
        let location_name = match self.location.get(region.location_id).await {
            Ok(Some(loc)) => loc.name,
            Ok(None) => {
                tracing::error!(
                    region_id = %region.id,
                    location_id = %region.location_id,
                    "Failed to load location for region: location not found"
                );
                "Unknown Location".to_string()
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    region_id = %region.id,
                    location_id = %region.location_id,
                    "Failed to load location for region"
                );
                "Unknown Location".to_string()
            }
        };

        let region_data = RegionData {
            id: region.id.to_string(),
            name: region.name.clone(),
            location_id: region.location_id.to_string(),
            location_name,
            backdrop_asset: region.backdrop_asset.clone(),
            atmosphere: region.atmosphere.clone(),
            map_asset: None,
        };

        let npcs_present: Vec<NpcPresenceData> = npcs
            .into_iter()
            .filter(|npc| include_hidden_npcs || npc.is_visible_to_players())
            .map(|npc| NpcPresenceData {
                character_id: npc.character_id.to_string(),
                name: npc.name,
                sprite_asset: npc.sprite_asset,
                portrait_asset: npc.portrait_asset,
            })
            .collect();

        let navigation = self.build_navigation_data(region.id).await;
        let region_items = self.build_region_items(region.id).await;

        SceneChangeData {
            region: region_data,
            npcs_present,
            navigation,
            region_items,
        }
    }

    async fn build_navigation_data(&self, region_id: RegionId) -> NavigationData {
        let connections = match self.location.get_connections(region_id).await {
            Ok(conns) => conns,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    region_id = %region_id,
                    "Failed to load region connections for navigation"
                );
                Vec::new()
            }
        };

        let mut connected_regions = Vec::new();
        for connection in connections {
            let region_name = match self.location.get_region(connection.to_region).await {
                Ok(Some(r)) => r.name,
                Ok(None) => {
                    tracing::error!(
                        from_region = %region_id,
                        to_region = %connection.to_region,
                        "Connected region not found"
                    );
                    "Unknown".to_string()
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        from_region = %region_id,
                        to_region = %connection.to_region,
                        "Failed to load connected region"
                    );
                    "Unknown".to_string()
                }
            };

            connected_regions.push(wrldbldr_protocol::NavigationTarget {
                region_id: connection.to_region.to_string(),
                name: region_name,
                is_locked: connection.is_locked,
                lock_description: connection.lock_description,
            });
        }

        let exits = match self.location.get_exits(region_id).await {
            Ok(result) => {
                // Log any skipped exits at error level (data integrity issues are already logged
                // in get_exits, but we also note them here for visibility)
                if !result.skipped.is_empty() {
                    tracing::warn!(
                        region_id = %region_id,
                        skipped_count = result.skipped.len(),
                        "Some navigation exits were skipped due to data integrity issues"
                    );
                }
                result
                    .exits
                    .into_iter()
                    .map(|exit| wrldbldr_protocol::NavigationExit {
                        location_id: exit.location_id.to_string(),
                        location_name: exit.location_name,
                        arrival_region_id: exit.arrival_region_id.to_string(),
                        description: exit.description,
                    })
                    .collect()
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    region_id = %region_id,
                    "Failed to load region exits for navigation"
                );
                Vec::new()
            }
        };

        NavigationData {
            connected_regions,
            exits,
        }
    }

    async fn build_region_items(&self, region_id: RegionId) -> Vec<RegionItemData> {
        match self.inventory.list_in_region(region_id).await {
            Ok(items) => items
                .into_iter()
                .map(|item| RegionItemData {
                    id: item.id.to_string(),
                    name: item.name,
                    description: item.description,
                    item_type: item.item_type,
                })
                .collect(),
            Err(e) => {
                tracing::warn!(error = %e, region_id = %region_id, "Failed to fetch region items");
                vec![]
            }
        }
    }
}

pub struct SceneChangeData {
    pub region: RegionData,
    pub npcs_present: Vec<NpcPresenceData>,
    pub navigation: NavigationData,
    pub region_items: Vec<RegionItemData>,
}
