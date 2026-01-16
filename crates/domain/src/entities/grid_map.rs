//! Grid map for tactical combat

use serde::{Deserialize, Serialize};
use wrldbldr_domain::{GridMapId, WorldId};

use crate::value_objects::AssetPath;

/// A tactical grid map for combat
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridMap {
    id: GridMapId,
    world_id: WorldId,
    name: String,
    width: u32,
    height: u32,
    /// Path to the tilesheet asset
    tilesheet_asset: AssetPath,
    /// Tile size in pixels (for rendering)
    tile_size: u32,
    /// The grid of tiles
    tiles: Vec<Vec<Tile>>,
}

impl GridMap {
    pub fn new(
        world_id: WorldId,
        name: impl Into<String>,
        width: u32,
        height: u32,
        tilesheet_asset: AssetPath,
    ) -> Self {
        let tiles = (0..height)
            .map(|_| (0..width).map(|_| Tile::default()).collect())
            .collect();

        Self {
            id: GridMapId::new(),
            world_id,
            name: name.into(),
            width,
            height,
            tilesheet_asset,
            tile_size: 32,
            tiles,
        }
    }

    /// Reconstruct from stored data
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        id: GridMapId,
        world_id: WorldId,
        name: String,
        width: u32,
        height: u32,
        tilesheet_asset: AssetPath,
        tile_size: u32,
        tiles: Vec<Vec<Tile>>,
    ) -> Self {
        Self {
            id,
            world_id,
            name,
            width,
            height,
            tilesheet_asset,
            tile_size,
            tiles,
        }
    }

    // Read-only accessors

    pub fn id(&self) -> GridMapId {
        self.id
    }

    pub fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn tilesheet_asset(&self) -> &AssetPath {
        &self.tilesheet_asset
    }

    pub fn tile_size(&self) -> u32 {
        self.tile_size
    }

    pub fn tiles(&self) -> &[Vec<Tile>] {
        &self.tiles
    }

    // Builder-style methods

    pub fn with_tile_size(mut self, tile_size: u32) -> Self {
        self.tile_size = tile_size;
        self
    }

    pub fn with_tiles(mut self, tiles: Vec<Vec<Tile>>) -> Self {
        self.tiles = tiles;
        self
    }

    // Tile access methods

    pub fn get_tile(&self, x: u32, y: u32) -> Option<&Tile> {
        self.tiles.get(y as usize)?.get(x as usize)
    }

    pub fn get_tile_mut(&mut self, x: u32, y: u32) -> Option<&mut Tile> {
        self.tiles.get_mut(y as usize)?.get_mut(x as usize)
    }

    pub fn set_tile(&mut self, x: u32, y: u32, tile: Tile) {
        if let Some(row) = self.tiles.get_mut(y as usize) {
            if let Some(cell) = row.get_mut(x as usize) {
                *cell = tile;
            }
        }
    }

    /// Calculate movement cost between adjacent tiles considering elevation
    pub fn movement_cost(&self, from: (u32, u32), to: (u32, u32)) -> Option<u32> {
        let from_tile = self.get_tile(from.0, from.1)?;
        let to_tile = self.get_tile(to.0, to.1)?;

        if !to_tile.passable {
            return None;
        }

        let elevation_diff = (to_tile.elevation - from_tile.elevation).abs();
        let base_cost = to_tile.terrain_type.movement_cost();

        // Climbing costs extra
        Some(base_cost.saturating_add(elevation_diff as u32))
    }
}

/// A single tile on the grid map
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tile {
    pub terrain_type: TerrainType,
    /// Elevation level (supports height differences)
    pub elevation: i32,
    /// Index into the tilesheet
    pub tile_index: u32,
    /// Whether units can move through this tile
    pub passable: bool,
    /// Cover value for combat (0 = none, 1 = light, 2 = heavy)
    pub cover_value: u8,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            terrain_type: TerrainType::Ground,
            elevation: 0,
            tile_index: 0,
            passable: true,
            cover_value: 0,
        }
    }
}

impl Tile {
    pub fn new(terrain_type: TerrainType, tile_index: u32) -> Self {
        Self {
            terrain_type,
            elevation: 0,
            tile_index,
            passable: terrain_type.default_passable(),
            cover_value: terrain_type.default_cover(),
        }
    }

    /// Reconstruct from stored data
    pub fn from_parts(
        terrain_type: TerrainType,
        elevation: i32,
        tile_index: u32,
        passable: bool,
        cover_value: u8,
    ) -> Self {
        Self {
            terrain_type,
            elevation,
            tile_index,
            passable,
            cover_value,
        }
    }

    // Builder-style methods

    pub fn with_elevation(mut self, elevation: i32) -> Self {
        self.elevation = elevation;
        self
    }

    pub fn with_tile_index(mut self, tile_index: u32) -> Self {
        self.tile_index = tile_index;
        self
    }

    pub fn blocking(mut self) -> Self {
        self.passable = false;
        self
    }

    pub fn with_passable(mut self, passable: bool) -> Self {
        self.passable = passable;
        self
    }

    pub fn with_cover(mut self, cover: u8) -> Self {
        self.cover_value = cover;
        self
    }
}

/// Types of terrain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TerrainType {
    #[default]
    Ground,
    Water,
    Wall,
    Difficult,
    Hazard,
    Pit,
}

impl TerrainType {
    pub fn movement_cost(&self) -> u32 {
        match self {
            Self::Ground => 1,
            Self::Water => 2,
            Self::Wall => u32::MAX,
            Self::Difficult => 2,
            Self::Hazard => 1,
            Self::Pit => u32::MAX,
        }
    }

    pub fn default_passable(&self) -> bool {
        !matches!(self, Self::Wall | Self::Pit)
    }

    pub fn default_cover(&self) -> u8 {
        match self {
            Self::Wall => 2,
            _ => 0,
        }
    }
}
