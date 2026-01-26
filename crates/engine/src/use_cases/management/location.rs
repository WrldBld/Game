//! Location and Region management operations.

use std::sync::Arc;

use wrldbldr_domain::value_objects::{Description, RegionName};
use wrldbldr_domain::{value_objects, LocationId, RegionId, WorldId};

use crate::infrastructure::ports::LocationRepo;
use crate::use_cases::validation::require_non_empty;

use super::ManagementError;

pub struct LocationManagement {
    location: Arc<dyn LocationRepo>,
}

impl LocationManagement {
    pub fn new(location: Arc<dyn LocationRepo>) -> Self {
        Self { location }
    }

    pub async fn list_locations(
        &self,
        world_id: WorldId,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<wrldbldr_domain::Location>, ManagementError> {
        Ok(self.location.list_locations_in_world(world_id, limit, offset).await?)
    }

    pub async fn get_location(
        &self,
        location_id: LocationId,
    ) -> Result<Option<wrldbldr_domain::Location>, ManagementError> {
        Ok(self.location.get_location(location_id).await?)
    }

    pub async fn create_location(
        &self,
        world_id: WorldId,
        name: String,
        description: Option<String>,
        setting: Option<String>,
        presence_cache_ttl_hours: Option<i32>,
        use_llm_presence: Option<bool>,
    ) -> Result<wrldbldr_domain::Location, ManagementError> {
        let location_name = value_objects::LocationName::new(&name)
            .map_err(ManagementError::Domain)?;
        let mut location = wrldbldr_domain::Location::new(
            world_id,
            location_name,
            wrldbldr_domain::LocationType::Unknown,
        );
        if let Some(description) = description {
            let desc = value_objects::Description::new(&description)
                .map_err(ManagementError::Domain)?;
            location = location.with_description(desc);
        }
        if let Some(setting) = setting {
            let atm = value_objects::Atmosphere::new(&setting)
                .map_err(ManagementError::Domain)?;
            location = location.with_atmosphere(atm);
        }
        if let Some(presence_cache_ttl_hours) = presence_cache_ttl_hours {
            location = location.with_presence_ttl(presence_cache_ttl_hours);
        }
        if let Some(use_llm_presence) = use_llm_presence {
            location = location.with_llm_presence(use_llm_presence);
        }

        self.location.save_location(&location).await?;
        Ok(location)
    }

    pub async fn update_location(
        &self,
        world_id: WorldId,
        location_id: LocationId,
        name: Option<String>,
        description: Option<String>,
        setting: Option<String>,
        presence_cache_ttl_hours: Option<i32>,
        use_llm_presence: Option<bool>,
    ) -> Result<wrldbldr_domain::Location, ManagementError> {
        let location =
            self.location
                .get_location(location_id)
                .await?
                .ok_or(ManagementError::NotFound {
                    entity_type: "Location",
                    id: location_id.to_string(),
                })?;

        // Validate entity belongs to requested world
        if location.world_id() != world_id {
            return Err(ManagementError::Unauthorized {
                message: "Location not in current world".to_string(),
            });
        }

        let mut location = location;

        if let Some(name) = name {
            require_non_empty(&name, "Location name")?;
            let location_name = value_objects::LocationName::new(&name)?;
            location.set_name(location_name);
        }
        if let Some(description) = description {
            let desc = value_objects::Description::new(&description)?;
            location.set_description(desc);
        }
        if let Some(setting) = setting {
            let atm = value_objects::Atmosphere::new(&setting)?;
            location.set_atmosphere(Some(atm));
        }
        if let Some(presence_cache_ttl_hours) = presence_cache_ttl_hours {
            location.set_presence_ttl(presence_cache_ttl_hours);
        }
        if let Some(use_llm_presence) = use_llm_presence {
            location.set_llm_presence(use_llm_presence);
        }

        self.location.save_location(&location).await?;
        Ok(location)
    }

    pub async fn delete_location(&self, location_id: LocationId) -> Result<(), ManagementError> {
        self.location.delete_location(location_id).await?;
        Ok(())
    }

    pub async fn list_regions(
        &self,
        location_id: LocationId,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<wrldbldr_domain::Region>, ManagementError> {
        Ok(self.location.list_regions_in_location(location_id, limit, offset).await?)
    }

    pub async fn get_region(
        &self,
        region_id: RegionId,
    ) -> Result<Option<wrldbldr_domain::Region>, ManagementError> {
        Ok(self.location.get_region(region_id).await?)
    }

    pub async fn create_region(
        &self,
        location_id: LocationId,
        name: String,
        description: Option<String>,
        is_spawn_point: Option<bool>,
    ) -> Result<wrldbldr_domain::Region, ManagementError> {
        let region_name =
            RegionName::new(&name).map_err(ManagementError::Domain)?;

        let mut region = wrldbldr_domain::Region::new(location_id, region_name);
        if let Some(description) = description {
            region = region.with_description(description);
        }
        if is_spawn_point.unwrap_or(false) {
            region = region.as_spawn_point();
        }

        self.location.save_region(&region).await?;
        Ok(region)
    }

    pub async fn update_region(
        &self,
        region_id: RegionId,
        name: Option<String>,
        description: Option<String>,
        is_spawn_point: Option<bool>,
    ) -> Result<wrldbldr_domain::Region, ManagementError> {
        let region =
            self.location
                .get_region(region_id)
                .await?
                .ok_or(ManagementError::NotFound {
                    entity_type: "Region",
                    id: region_id.to_string(),
                })?;

        // Regions are immutable - rebuild with updated values using from_parts
        let new_name = if let Some(name) = name {
            RegionName::new(&name).map_err(ManagementError::Domain)?
        } else {
            region.name().clone()
        };
        let new_description = if let Some(desc) = description {
            Description::new(&desc).map_err(|e| ManagementError::InvalidInput(e.to_string()))?
        } else {
            Description::new(region.description()).unwrap_or_default()
        };
        let new_is_spawn_point = is_spawn_point.unwrap_or_else(|| region.is_spawn_point());

        let region = wrldbldr_domain::Region::from_parts(
            region.id(),
            region.location_id(),
            new_name,
            new_description,
            region.backdrop_asset().cloned(),
            region.atmosphere().cloned(),
            region.map_bounds().cloned(),
            new_is_spawn_point,
            region.order(),
        );

        self.location.save_region(&region).await?;
        Ok(region)
    }

    pub async fn delete_region(&self, region_id: RegionId) -> Result<(), ManagementError> {
        self.location.delete_region(region_id).await?;
        Ok(())
    }

    pub async fn list_spawn_points(
        &self,
        world_id: WorldId,
    ) -> Result<Vec<wrldbldr_domain::Region>, ManagementError> {
        let mut spawn_points = Vec::new();
        let locations = self.location.list_locations_in_world(world_id, None, None).await?;
        for location in locations {
            let regions = self
                .location
                .list_regions_in_location(location.id(), None, None)
                .await?;
            spawn_points.extend(regions.into_iter().filter(|r| r.is_spawn_point()));
        }
        Ok(spawn_points)
    }

    pub async fn list_location_connections(
        &self,
        location_id: LocationId,
        limit: Option<u32>,
    ) -> Result<Vec<wrldbldr_domain::LocationConnection>, ManagementError> {
        Ok(self.location.get_location_exits(location_id, limit).await?)
    }

    pub async fn create_location_connection(
        &self,
        from_location: LocationId,
        to_location: LocationId,
        bidirectional: bool,
    ) -> Result<(), ManagementError> {
        let connection = wrldbldr_domain::LocationConnection {
            from_location,
            to_location,
            connection_type: wrldbldr_domain::ConnectionType::Other,
            description: None,
            bidirectional,
            travel_time: 0,
            is_locked: false,
            lock_description: None,
        };

        self.location.save_location_connection(&connection).await?;
        Ok(())
    }

    pub async fn delete_location_connection(
        &self,
        from_location: LocationId,
        to_location: LocationId,
    ) -> Result<(), ManagementError> {
        self.location
            .delete_location_connection(from_location, to_location)
            .await?;
        Ok(())
    }

    pub async fn list_region_connections(
        &self,
        region_id: RegionId,
        limit: Option<u32>,
    ) -> Result<Vec<wrldbldr_domain::RegionConnection>, ManagementError> {
        Ok(self.location.get_connections(region_id, limit).await?)
    }

    pub async fn create_region_connection(
        &self,
        from_region: RegionId,
        to_region: RegionId,
        description: Option<String>,
        bidirectional: Option<bool>,
        locked: Option<bool>,
        lock_description: Option<String>,
    ) -> Result<(), ManagementError> {
        if from_region == to_region {
            return Err(ManagementError::InvalidInput(
                "Cannot connect a region to itself".to_string(),
            ));
        }
        let is_locked = locked.unwrap_or(false);
        let conn_description = if let Some(desc) = description {
            Some(Description::new(&desc).map_err(|e| ManagementError::InvalidInput(e.to_string()))?)
        } else {
            None
        };
        let connection = wrldbldr_domain::RegionConnection {
            from_region,
            to_region,
            description: conn_description,
            bidirectional: bidirectional.unwrap_or(true),
            is_locked,
            lock_description: if is_locked {
                Some(lock_description.unwrap_or_else(|| "Locked".to_string()))
            } else {
                None
            },
        };

        self.location.save_connection(&connection).await?;
        Ok(())
    }

    pub async fn delete_region_connection(
        &self,
        from_region: RegionId,
        to_region: RegionId,
    ) -> Result<(), ManagementError> {
        self.location
            .delete_connection(from_region, to_region)
            .await?;
        Ok(())
    }

    pub async fn unlock_region_connection(
        &self,
        from_region: RegionId,
        to_region: RegionId,
    ) -> Result<(), ManagementError> {
        let connections = self.location.get_connections(from_region, None).await?;
        let existing = connections
            .into_iter()
            .find(|c| c.to_region == to_region)
            .ok_or(ManagementError::NotFound {
                entity_type: "RegionConnection",
                id: format!("{}→{}", from_region, to_region),
            })?;

        // Rebuild connection with updated lock state
        let updated = wrldbldr_domain::RegionConnection {
            from_region: existing.from_region,
            to_region: existing.to_region,
            description: existing.description.clone(),
            bidirectional: existing.bidirectional,
            is_locked: false,
            lock_description: None,
        };

        self.location.save_connection(&updated).await?;
        Ok(())
    }

    pub async fn list_region_exits(
        &self,
        region_id: RegionId,
        limit: Option<u32>,
    ) -> Result<Vec<wrldbldr_domain::RegionExit>, ManagementError> {
        Ok(self.location.get_region_exits(region_id, limit).await?)
    }

    pub async fn create_region_exit(
        &self,
        region_id: RegionId,
        location_id: LocationId,
        arrival_region_id: RegionId,
        description: Option<String>,
        bidirectional: Option<bool>,
    ) -> Result<(), ManagementError> {
        let is_bidirectional = bidirectional.unwrap_or(true);

        // Validate source region exists
        let source_region = self.location.get_region(region_id).await?.ok_or_else(|| {
            ManagementError::InvalidInput(format!("Source region {} does not exist", region_id))
        })?;

        // Validate target location exists
        let target_location = self
            .location
            .get_location(location_id)
            .await?
            .ok_or_else(|| {
                ManagementError::InvalidInput(format!(
                    "Target location {} does not exist",
                    location_id
                ))
            })?;

        // Validate arrival region exists and is in the target location
        let arrival_region = self
            .location
            .get_region(arrival_region_id)
            .await?
            .ok_or_else(|| {
                ManagementError::InvalidInput(format!(
                    "Arrival region {} does not exist",
                    arrival_region_id
                ))
            })?;

        if arrival_region.location_id() != location_id {
            return Err(ManagementError::InvalidInput(format!(
                "Arrival region {} is not in target location {} (it's in {})",
                arrival_region_id,
                target_location.name().as_str(),
                arrival_region.location_id()
            )));
        }

        // For bidirectional exits, validate the return path can be created
        if is_bidirectional {
            // The return path goes from arrival_region back to source_region's location
            // We need to ensure source_region's location exists (should always be true if source_region exists)
            let source_location = self
                .location
                .get_location(source_region.location_id())
                .await?
                .ok_or_else(|| {
                    ManagementError::InvalidInput(format!(
                        "Source region's location {} does not exist (data integrity issue)",
                        source_region.location_id()
                    ))
                })?;

            tracing::debug!(
                from_region = %region_id,
                to_location = %target_location.name().as_str(),
                arrival_region = %arrival_region.name(),
                return_location = %source_location.name().as_str(),
                "Creating bidirectional exit with validated return path"
            );
        }

        let exit_description = if let Some(desc) = description {
            Some(Description::new(&desc).map_err(|e| ManagementError::InvalidInput(e.to_string()))?)
        } else {
            None
        };
        let exit = wrldbldr_domain::RegionExit {
            from_region: region_id,
            to_location: location_id,
            arrival_region_id,
            description: exit_description,
            bidirectional: is_bidirectional,
        };
        self.location.save_region_exit(&exit).await?;
        Ok(())
    }

    pub async fn delete_region_exit(
        &self,
        region_id: RegionId,
        location_id: LocationId,
    ) -> Result<(), ManagementError> {
        self.location
            .delete_region_exit(region_id, location_id)
            .await?;
        Ok(())
    }
}
