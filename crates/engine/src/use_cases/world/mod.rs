//! World management use cases.
//!
//! Handles world export and import for backup/sharing.

use std::sync::Arc;
use wrldbldr_domain::WorldId;

use crate::repositories::World;
use crate::repositories::character::Character;
use crate::repositories::inventory::Inventory;
use crate::repositories::location::Location;
use crate::use_cases::narrative_operations::Narrative;
use crate::infrastructure::ports::RepoError;

/// Container for world use cases.
pub struct WorldUseCases {
    pub export: Arc<ExportWorld>,
    pub import: Arc<ImportWorld>,
}

impl WorldUseCases {
    pub fn new(export: Arc<ExportWorld>, import: Arc<ImportWorld>) -> Self {
        Self { export, import }
    }
}

/// Exported world data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorldExport {
    /// The world metadata
    pub world: wrldbldr_domain::World,
    /// All locations in the world
    pub locations: Vec<wrldbldr_domain::Location>,
    /// All regions in the world
    pub regions: Vec<wrldbldr_domain::Region>,
    /// All NPCs in the world
    pub characters: Vec<wrldbldr_domain::Character>,
    /// All items in the world
    pub items: Vec<wrldbldr_domain::Item>,
    /// All narrative events
    pub narrative_events: Vec<wrldbldr_domain::NarrativeEvent>,
    /// Export format version
    pub format_version: u32,
}

/// Export world use case.
///
/// Exports a world and all its contents to a portable format.
pub struct ExportWorld {
    world: Arc<World>,
    location: Arc<Location>,
    character: Arc<Character>,
    inventory: Arc<Inventory>,
    narrative: Arc<Narrative>,
}

impl ExportWorld {
    pub fn new(
        world: Arc<World>,
        location: Arc<Location>,
        character: Arc<Character>,
        inventory: Arc<Inventory>,
        narrative: Arc<Narrative>,
    ) -> Self {
        Self {
            world,
            location,
            character,
            inventory,
            narrative,
        }
    }

    /// Export a world to a portable format.
    ///
    /// # Arguments
    /// * `world_id` - The world to export
    ///
    /// # Returns
    /// * `Ok(WorldExport)` - The exported world data
    /// * `Err(WorldError)` - Export failed
    pub async fn execute(&self, world_id: WorldId) -> Result<WorldExport, WorldError> {
        // Get the world
        let world = self
            .world
            .get(world_id)
            .await?
            .ok_or(WorldError::NotFound)?;

        // Get all locations
        let locations = self.location.list_in_world(world_id).await?;

        // Get all regions
        let mut regions = Vec::new();
        for loc in &locations {
            let loc_regions = self.location.list_regions_in_location(loc.id()).await?;
            regions.extend(loc_regions);
        }

        // Get all characters
        let characters = self.character.list_in_world(world_id).await?;

        // Get all items
        let items = self.inventory.list_in_world(world_id).await?;

        // Get all narrative events
        let narrative_events = self.narrative.list_events(world_id).await?;

        Ok(WorldExport {
            world,
            locations,
            regions,
            characters,
            items,
            narrative_events,
            format_version: 1,
        })
    }
}

/// Import world use case.
///
/// Imports a world from an exported format.
pub struct ImportWorld {
    world: Arc<World>,
    location: Arc<Location>,
    character: Arc<Character>,
    inventory: Arc<Inventory>,
    narrative: Arc<Narrative>,
}

impl ImportWorld {
    pub fn new(
        world: Arc<World>,
        location: Arc<Location>,
        character: Arc<Character>,
        inventory: Arc<Inventory>,
        narrative: Arc<Narrative>,
    ) -> Self {
        Self {
            world,
            location,
            character,
            inventory,
            narrative,
        }
    }

    /// Import a world from exported data.
    ///
    /// # Arguments
    /// * `data` - The exported world data
    ///
    /// # Returns
    /// * `Ok(WorldId)` - The ID of the imported world
    /// * `Err(WorldError)` - Import failed
    pub async fn execute(&self, data: WorldExport) -> Result<WorldId, WorldError> {
        // Validate format version
        if data.format_version > 1 {
            return Err(WorldError::ImportFailed(format!(
                "Unsupported format version: {}",
                data.format_version
            )));
        }

        // Create the world
        let world_id = data.world.id;
        self.world.save(&data.world).await?;

        // Create locations
        for location in &data.locations {
            self.location.save_location(location).await?;
        }

        // Create regions
        for region in &data.regions {
            self.location.save_region(region).await?;
        }

        // Create characters
        for character in &data.characters {
            self.character.save(character).await?;
        }

        // Create items
        for item in &data.items {
            self.inventory.save(item).await?;
        }

        // Create narrative events
        for event in &data.narrative_events {
            self.narrative.save_event(event).await?;
        }

        Ok(world_id)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("World not found")]
    NotFound,
    #[error("Export failed: {0}")]
    ExportFailed(String),
    #[error("Import failed: {0}")]
    ImportFailed(String),
    #[error("Repository error: {0}")]
    Repo(#[from] RepoError),
}
