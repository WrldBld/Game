//! Neo4j location repository implementation.
//!
//! Handles both Location and Region CRUD operations, plus connections.

use crate::infrastructure::neo4j::Neo4jGraph;
use async_trait::async_trait;
use neo4rs::{query, Row};
use uuid::Uuid;
use wrldbldr_domain::*;

use super::helpers::{parse_typed_id, NodeExt, RowExt};
use crate::infrastructure::ports::{LocationRepo, RepoError};

/// Repository for Location and Region operations.
pub struct Neo4jLocationRepo {
    graph: Neo4jGraph,
}

impl Neo4jLocationRepo {
    pub fn new(graph: Neo4jGraph) -> Self {
        Self { graph }
    }

    fn row_to_location(&self, row: Row) -> Result<Location, RepoError> {
        let node: neo4rs::Node = row.get("l").map_err(|e| RepoError::database("query", e))?;

        let id: LocationId =
            parse_typed_id(&node, "id").map_err(|e| RepoError::database("query", e))?;
        let world_id: WorldId =
            parse_typed_id(&node, "world_id").map_err(|e| RepoError::database("query", e))?;
        let name: String = node
            .get("name")
            .map_err(|e| RepoError::database("query", e))?;
        let description: String = node
            .get("description")
            .map_err(|e| RepoError::database("query", e))?;
        let location_type_str: String = node
            .get("location_type")
            .map_err(|e| RepoError::database("query", e))?;

        let backdrop_asset = node.get_optional_string("backdrop_asset");
        let map_asset = node.get_optional_string("map_asset");
        let atmosphere = node.get_optional_string("atmosphere");
        let presence_cache_ttl_hours = node.get_i64_or("presence_cache_ttl_hours", 3);
        let use_llm_presence = node.get_bool_or("use_llm_presence", true);

        // Parse optional default_region_id
        let default_region_id: Option<RegionId> = node
            .get_optional_string("default_region_id")
            .and_then(|s| uuid::Uuid::parse_str(&s).ok())
            .map(RegionId::from_uuid);

        let location_type = match location_type_str.as_str() {
            "Interior" => LocationType::Interior,
            "Exterior" => LocationType::Exterior,
            "Abstract" => LocationType::Abstract,
            _ => LocationType::Interior,
        };

        // Parse parent_map_bounds from JSON
        let parent_map_bounds = node
            .get_optional_string("parent_map_bounds")
            .filter(|s| !s.is_empty())
            .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
            .and_then(|v| {
                MapBounds::new(
                    v.get("x")?.as_u64()? as u32,
                    v.get("y")?.as_u64()? as u32,
                    v.get("width")?.as_u64()? as u32,
                    v.get("height")?.as_u64()? as u32,
                )
            });

        // Build location using aggregate constructor and builder pattern
        let location_name =
            value_objects::LocationName::new(&name).map_err(|e| RepoError::database("parse", e))?;
        let desc = value_objects::Description::new(&description)
            .map_err(|e| RepoError::database("parse", e))?;

        let mut location = Location::new(world_id, location_name, location_type)
            .with_id(id)
            .with_description(desc)
            .with_presence_ttl(presence_cache_ttl_hours as i32)
            .with_llm_presence(use_llm_presence);

        if let Some(asset) = backdrop_asset {
            location = location.with_backdrop(asset);
        }
        if let Some(asset) = map_asset {
            location = location.with_map(asset);
        }
        if let Some(atm) = atmosphere {
            location = location.with_atmosphere(atm);
        }
        if let Some(bounds) = parent_map_bounds {
            location = location.with_parent_map_bounds(bounds);
        }
        if let Some(region_id) = default_region_id {
            location = location.with_default_region(region_id);
        }

        Ok(location)
    }

    fn row_to_region(&self, row: &Row) -> Result<Region, RepoError> {
        let node: neo4rs::Node = row.get("r").map_err(|e| RepoError::database("query", e))?;

        let id: RegionId =
            parse_typed_id(&node, "id").map_err(|e| RepoError::database("query", e))?;
        let location_id: LocationId =
            parse_typed_id(&node, "location_id").map_err(|e| RepoError::database("query", e))?;
        let name: String = node
            .get("name")
            .map_err(|e| RepoError::database("query", e))?;
        let description = node.get_string_or("description", "");
        let backdrop_asset = node.get_optional_string("backdrop_asset");
        let atmosphere = node.get_optional_string("atmosphere");
        let is_spawn_point = node.get_bool_or("is_spawn_point", false);
        let order = node.get_i64_or("order", 0) as u32;

        // Parse map_bounds from JSON
        let map_bounds = node
            .get_optional_string("map_bounds")
            .filter(|s| !s.is_empty())
            .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
            .and_then(|v| {
                MapBounds::new(
                    v.get("x")?.as_u64()? as u32,
                    v.get("y")?.as_u64()? as u32,
                    v.get("width")?.as_u64()? as u32,
                    v.get("height")?.as_u64()? as u32,
                )
            });

        Ok(Region::from_parts(
            id,
            location_id,
            name,
            description,
            backdrop_asset,
            atmosphere,
            map_bounds,
            is_spawn_point,
            order,
        ))
    }

    fn row_to_region_connection(&self, row: Row) -> Result<RegionConnection, RepoError> {
        let from_id_str: String = row
            .get("from_id")
            .map_err(|e| RepoError::database("query", e))?;
        let to_id_str: String = row
            .get("to_id")
            .map_err(|e| RepoError::database("query", e))?;
        let description = row.get_optional_string("description");
        let bidirectional: bool = row.get("bidirectional").unwrap_or(true);
        let is_locked: bool = row.get("is_locked").unwrap_or(false);
        let lock_description = row.get_optional_string("lock_description");

        let from_id =
            uuid::Uuid::parse_str(&from_id_str).map_err(|e| RepoError::database("query", e))?;
        let to_id =
            uuid::Uuid::parse_str(&to_id_str).map_err(|e| RepoError::database("query", e))?;

        Ok(RegionConnection::from_parts(
            RegionId::from_uuid(from_id),
            RegionId::from_uuid(to_id),
            description,
            bidirectional,
            is_locked,
            lock_description,
        ))
    }

    fn row_to_location_connection(&self, row: Row) -> Result<LocationConnection, RepoError> {
        let from_id_str: String = row
            .get("from_id")
            .map_err(|e| RepoError::database("query", e))?;
        let to_id_str: String = row
            .get("to_id")
            .map_err(|e| RepoError::database("query", e))?;
        let connection_type_str: String = row
            .get("connection_type")
            .map_err(|e| RepoError::database("query", e))?;
        let bidirectional: bool = row.get("bidirectional").unwrap_or(true);
        let travel_time: i64 = row.get("travel_time").unwrap_or(0);
        let is_locked: bool = row.get("is_locked").unwrap_or(false);
        let description = row.get_optional_string("description");
        let lock_description = row.get_optional_string("lock_description");

        let from_id =
            uuid::Uuid::parse_str(&from_id_str).map_err(|e| RepoError::database("query", e))?;
        let to_id =
            uuid::Uuid::parse_str(&to_id_str).map_err(|e| RepoError::database("query", e))?;

        Ok(LocationConnection::from_parts(
            LocationId::from_uuid(from_id),
            LocationId::from_uuid(to_id),
            ConnectionType::parse(&connection_type_str),
            description,
            bidirectional,
            travel_time as u32,
            is_locked,
            lock_description,
        ))
    }
}

#[async_trait]
impl LocationRepo for Neo4jLocationRepo {
    // =========================================================================
    // Location CRUD
    // =========================================================================

    async fn get_location(&self, id: LocationId) -> Result<Option<Location>, RepoError> {
        let q = query("MATCH (l:Location {id: $id}) RETURN l").param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            Ok(Some(self.row_to_location(row)?))
        } else {
            Ok(None)
        }
    }

    async fn save_location(&self, location: &Location) -> Result<(), RepoError> {
        let map_bounds_json = location
            .parent_map_bounds()
            .map(|b| {
                serde_json::json!({
                    "x": b.x(),
                    "y": b.y(),
                    "width": b.width(),
                    "height": b.height()
                })
                .to_string()
            })
            .unwrap_or_default();

        let q = query(
            "MERGE (l:Location {id: $id})
            SET l.world_id = $world_id,
                l.name = $name,
                l.description = $description,
                l.location_type = $location_type,
                l.backdrop_asset = $backdrop_asset,
                l.map_asset = $map_asset,
                l.parent_map_bounds = $parent_map_bounds,
                l.default_region_id = $default_region_id,
                l.atmosphere = $atmosphere,
                l.presence_cache_ttl_hours = $presence_cache_ttl_hours,
                l.use_llm_presence = $use_llm_presence
            WITH l
            MATCH (w:World {id: $world_id})
            MERGE (w)-[:CONTAINS_LOCATION]->(l)
            RETURN l.id as id",
        )
        .param("id", location.id().to_string())
        .param("world_id", location.world_id().to_string())
        .param("name", location.name().as_str().to_string())
        .param("description", location.description().as_str().to_string())
        .param("location_type", format!("{:?}", location.location_type()))
        .param(
            "backdrop_asset",
            location.backdrop_asset().unwrap_or_default().to_string(),
        )
        .param(
            "map_asset",
            location.map_asset().unwrap_or_default().to_string(),
        )
        .param("parent_map_bounds", map_bounds_json)
        .param(
            "default_region_id",
            location
                .default_region_id()
                .map(|id| id.to_string())
                .unwrap_or_default(),
        )
        .param(
            "atmosphere",
            location.atmosphere().unwrap_or_default().to_string(),
        )
        .param(
            "presence_cache_ttl_hours",
            location.presence_cache_ttl_hours() as i64,
        )
        .param("use_llm_presence", location.use_llm_presence());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Saved location: {}", location.name().as_str());
        Ok(())
    }

    async fn list_locations_in_world(&self, world_id: WorldId) -> Result<Vec<Location>, RepoError> {
        let q = query(
            "MATCH (w:World {id: $world_id})-[:CONTAINS_LOCATION]->(l:Location)
            RETURN l
            ORDER BY l.name",
        )
        .param("world_id", world_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut locations = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            locations.push(self.row_to_location(row)?);
        }

        Ok(locations)
    }

    async fn delete_location(&self, id: LocationId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (l:Location {id: $id})
            DETACH DELETE l",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Deleted location: {}", id);
        Ok(())
    }

    // =========================================================================
    // Region CRUD
    // =========================================================================

    async fn get_region(&self, id: RegionId) -> Result<Option<Region>, RepoError> {
        let q = query("MATCH (r:Region {id: $id}) RETURN r").param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            Ok(Some(self.row_to_region(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn save_region(&self, region: &Region) -> Result<(), RepoError> {
        let map_bounds_json = region
            .map_bounds()
            .map(|b| {
                serde_json::json!({
                    "x": b.x(),
                    "y": b.y(),
                    "width": b.width(),
                    "height": b.height()
                })
                .to_string()
            })
            .unwrap_or_default();

        let q = query(
            "MERGE (r:Region {id: $id})
            SET r.location_id = $location_id,
                r.name = $name,
                r.description = $description,
                r.backdrop_asset = $backdrop_asset,
                r.atmosphere = $atmosphere,
                r.map_bounds = $map_bounds,
                r.is_spawn_point = $is_spawn_point,
                r.order = $order
            WITH r
            MATCH (l:Location {id: $location_id})
            MERGE (l)-[:HAS_REGION]->(r)
            RETURN r.id as id",
        )
        .param("id", region.id().to_string())
        .param("location_id", region.location_id().to_string())
        .param("name", region.name().to_string())
        .param("description", region.description().to_string())
        .param(
            "backdrop_asset",
            region.backdrop_asset().unwrap_or_default().to_string(),
        )
        .param(
            "atmosphere",
            region.atmosphere().unwrap_or_default().to_string(),
        )
        .param("map_bounds", map_bounds_json)
        .param("is_spawn_point", region.is_spawn_point())
        .param("order", region.order() as i64);

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Saved region: {}", region.name());
        Ok(())
    }

    async fn list_regions_in_location(
        &self,
        location_id: LocationId,
    ) -> Result<Vec<Region>, RepoError> {
        let q = query(
            "MATCH (l:Location {id: $location_id})-[:HAS_REGION]->(r:Region)
            RETURN r
            ORDER BY r.order",
        )
        .param("location_id", location_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut regions = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            regions.push(self.row_to_region(&row)?);
        }

        Ok(regions)
    }

    async fn delete_region(&self, id: RegionId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (r:Region {id: $id})
            DETACH DELETE r",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Deleted region: {}", id);
        Ok(())
    }

    // =========================================================================
    // Region Connections
    // =========================================================================

    async fn get_connections(
        &self,
        region_id: RegionId,
    ) -> Result<Vec<RegionConnection>, RepoError> {
        let q = query(
            "MATCH (from:Region {id: $id})-[rel:CONNECTED_TO_REGION]->(to:Region)
            RETURN from.id as from_id, to.id as to_id,
                   rel.description as description,
                   rel.bidirectional as bidirectional,
                   rel.is_locked as is_locked,
                   rel.lock_description as lock_description",
        )
        .param("id", region_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut connections = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            connections.push(self.row_to_region_connection(row)?);
        }

        Ok(connections)
    }

    async fn save_connection(&self, connection: &RegionConnection) -> Result<(), RepoError> {
        let q = query(
            "MATCH (from:Region {id: $from_id})
            MATCH (to:Region {id: $to_id})
            MERGE (from)-[rel:CONNECTED_TO_REGION]->(to)
            SET rel.description = $description,
                rel.bidirectional = $bidirectional,
                rel.is_locked = $is_locked,
                rel.lock_description = $lock_description
            RETURN from.id as from_id",
        )
        .param("from_id", connection.from_region().to_string())
        .param("to_id", connection.to_region().to_string())
        .param(
            "description",
            connection.description().unwrap_or_default().to_string(),
        )
        .param("bidirectional", connection.bidirectional())
        .param("is_locked", connection.is_locked())
        .param(
            "lock_description",
            connection
                .lock_description()
                .unwrap_or_default()
                .to_string(),
        );

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // If bidirectional, also create the reverse connection
        if connection.bidirectional() {
            let reverse_q = query(
                "MATCH (from:Region {id: $to_id})
                MATCH (to:Region {id: $from_id})
                MERGE (from)-[rel:CONNECTED_TO_REGION]->(to)
                SET rel.description = $description,
                    rel.bidirectional = $bidirectional,
                    rel.is_locked = $is_locked,
                    rel.lock_description = $lock_description
                RETURN from.id as from_id",
            )
            .param("from_id", connection.from_region().to_string())
            .param("to_id", connection.to_region().to_string())
            .param(
                "description",
                connection.description().unwrap_or_default().to_string(),
            )
            .param("bidirectional", connection.bidirectional())
            .param("is_locked", connection.is_locked())
            .param(
                "lock_description",
                connection
                    .lock_description()
                    .unwrap_or_default()
                    .to_string(),
            );

            self.graph
                .run(reverse_q)
                .await
                .map_err(|e| RepoError::database("query", e))?;
        }

        tracing::debug!(
            "Saved connection from {} to {}",
            connection.from_region(),
            connection.to_region()
        );
        Ok(())
    }

    async fn delete_connection(
        &self,
        from_region: RegionId,
        to_region: RegionId,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (from:Region {id: $from_id})-[rel:CONNECTED_TO_REGION]->(to:Region {id: $to_id})
            DELETE rel",
        )
        .param("from_id", from_region.to_string())
        .param("to_id", to_region.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let reverse_q = query(
            "MATCH (from:Region {id: $to_id})-[rel:CONNECTED_TO_REGION]->(to:Region {id: $from_id})
            DELETE rel",
        )
        .param("from_id", from_region.to_string())
        .param("to_id", to_region.to_string());

        self.graph
            .run(reverse_q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        Ok(())
    }

    // =========================================================================
    // Location Connections (exits between locations)
    // =========================================================================

    async fn get_location_exits(
        &self,
        location_id: LocationId,
    ) -> Result<Vec<LocationConnection>, RepoError> {
        let q = query(
            "MATCH (from:Location {id: $id})-[r:CONNECTED_TO]->(to:Location)
            RETURN from.id as from_id, to.id as to_id,
                   r.connection_type as connection_type,
                   r.description as description,
                   r.bidirectional as bidirectional,
                   r.travel_time as travel_time,
                   r.is_locked as is_locked,
                   r.lock_description as lock_description",
        )
        .param("id", location_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut connections = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            connections.push(self.row_to_location_connection(row)?);
        }

        Ok(connections)
    }

    async fn save_location_connection(
        &self,
        connection: &LocationConnection,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (from:Location {id: $from_id})
            MATCH (to:Location {id: $to_id})
            MERGE (from)-[r:CONNECTED_TO]->(to)
            SET r.connection_type = $connection_type,
                r.description = $description,
                r.bidirectional = $bidirectional,
                r.travel_time = $travel_time,
                r.is_locked = $is_locked,
                r.lock_description = $lock_description",
        )
        .param("from_id", connection.from_location().to_string())
        .param("to_id", connection.to_location().to_string())
        .param("connection_type", connection.connection_type().as_str())
        .param(
            "description",
            connection.description().unwrap_or_default().to_string(),
        )
        .param("bidirectional", connection.bidirectional())
        .param("travel_time", connection.travel_time() as i64)
        .param("is_locked", connection.is_locked())
        .param(
            "lock_description",
            connection
                .lock_description()
                .unwrap_or_default()
                .to_string(),
        );

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if connection.bidirectional() {
            let reverse_q = query(
                "MATCH (from:Location {id: $to_id})
                MATCH (to:Location {id: $from_id})
                MERGE (from)-[r:CONNECTED_TO]->(to)
                SET r.connection_type = $connection_type,
                    r.description = $description,
                    r.bidirectional = $bidirectional,
                    r.travel_time = $travel_time,
                    r.is_locked = $is_locked,
                    r.lock_description = $lock_description",
            )
            .param("from_id", connection.from_location().to_string())
            .param("to_id", connection.to_location().to_string())
            .param("connection_type", connection.connection_type().as_str())
            .param(
                "description",
                connection.description().unwrap_or_default().to_string(),
            )
            .param("bidirectional", connection.bidirectional())
            .param("travel_time", connection.travel_time() as i64)
            .param("is_locked", connection.is_locked())
            .param(
                "lock_description",
                connection
                    .lock_description()
                    .unwrap_or_default()
                    .to_string(),
            );

            self.graph
                .run(reverse_q)
                .await
                .map_err(|e| RepoError::database("query", e))?;
        }

        Ok(())
    }

    async fn delete_location_connection(
        &self,
        from_location: LocationId,
        to_location: LocationId,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (from:Location {id: $from_id})-[r:CONNECTED_TO]->(to:Location {id: $to_id})
            DELETE r",
        )
        .param("from_id", from_location.to_string())
        .param("to_id", to_location.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let reverse_q = query(
            "MATCH (from:Location {id: $to_id})-[r:CONNECTED_TO]->(to:Location {id: $from_id})
            DELETE r",
        )
        .param("from_id", from_location.to_string())
        .param("to_id", to_location.to_string());

        self.graph
            .run(reverse_q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        Ok(())
    }

    async fn get_region_exits(&self, region_id: RegionId) -> Result<Vec<RegionExit>, RepoError> {
        let q = query(
            "MATCH (r:Region {id: $id})-[rel:EXITS_TO_LOCATION]->(l:Location)
            RETURN r.id as from_region, l.id as to_location,
                   rel.arrival_region_id as arrival_region_id,
                   rel.description as description,
                   rel.bidirectional as bidirectional",
        )
        .param("id", region_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        let mut exits = Vec::new();
        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            let from_region: String = row
                .get("from_region")
                .map_err(|e| RepoError::database("query", e))?;
            let to_location: String = row
                .get("to_location")
                .map_err(|e| RepoError::database("query", e))?;
            let arrival_region: String = row
                .get("arrival_region_id")
                .map_err(|e| RepoError::database("query", e))?;
            let description: String = row.get("description").unwrap_or_default();
            let bidirectional: bool = row.get("bidirectional").unwrap_or(false);

            exits.push(RegionExit::from_parts(
                RegionId::from(
                    Uuid::parse_str(&from_region).map_err(|e| RepoError::database("query", e))?,
                ),
                LocationId::from(
                    Uuid::parse_str(&to_location).map_err(|e| RepoError::database("query", e))?,
                ),
                RegionId::from(
                    Uuid::parse_str(&arrival_region)
                        .map_err(|e| RepoError::database("query", e))?,
                ),
                if description.is_empty() {
                    None
                } else {
                    Some(description)
                },
                bidirectional,
            ));
        }

        Ok(exits)
    }

    async fn save_region_exit(&self, exit: &RegionExit) -> Result<(), RepoError> {
        let q = query(
            "MATCH (from:Region {id: $from_id})<-[:HAS_REGION]-(from_location:Location)
            MATCH (to:Location {id: $to_id})-[:HAS_REGION]->(arrival_region:Region {id: $arrival_region_id})
            MERGE (from)-[r:EXITS_TO_LOCATION]->(to)
            SET r.arrival_region_id = $arrival_region_id,
                r.description = $description,
                r.bidirectional = $bidirectional",
        )
        .param("from_id", exit.from_region().to_string())
        .param("to_id", exit.to_location().to_string())
        .param("arrival_region_id", exit.arrival_region_id().to_string())
        .param("description", exit.description().unwrap_or_default().to_string())
        .param("bidirectional", exit.bidirectional());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        if exit.bidirectional() {
            let reverse_q = query(
                "MATCH (from:Region {id: $from_id})<-[:HAS_REGION]-(from_location:Location)
                MATCH (to:Location {id: $to_id})-[:HAS_REGION]->(arrival_region:Region {id: $arrival_region_id})
                MERGE (arrival_region)-[r:EXITS_TO_LOCATION]->(from_location)
                SET r.arrival_region_id = $from_id,
                    r.description = $description,
                    r.bidirectional = $bidirectional",
            )
            .param("from_id", exit.from_region().to_string())
            .param("to_id", exit.to_location().to_string())
            .param("arrival_region_id", exit.arrival_region_id().to_string())
            .param("description", exit.description().unwrap_or_default().to_string())
            .param("bidirectional", exit.bidirectional());

            self.graph
                .run(reverse_q)
                .await
                .map_err(|e| RepoError::database("query", e))?;
        } else {
            let cleanup_q = query(
                "MATCH (from:Region {id: $from_id})<-[:HAS_REGION]-(from_location:Location)
                MATCH (to:Location {id: $to_id})-[:HAS_REGION]->(arrival_region:Region {id: $arrival_region_id})
                OPTIONAL MATCH (arrival_region)-[r:EXITS_TO_LOCATION]->(from_location)
                DELETE r",
            )
            .param("from_id", exit.from_region().to_string())
            .param("to_id", exit.to_location().to_string())
            .param("arrival_region_id", exit.arrival_region_id().to_string());

            self.graph
                .run(cleanup_q)
                .await
                .map_err(|e| RepoError::database("query", e))?;
        }

        Ok(())
    }

    async fn delete_region_exit(
        &self,
        region_id: RegionId,
        location_id: LocationId,
    ) -> Result<(), RepoError> {
        let q = query(
            "MATCH (r:Region {id: $region_id})-[rel:EXITS_TO_LOCATION]->(l:Location {id: $location_id})
            WITH r, l, rel.arrival_region_id AS arrival_region_id, rel
            OPTIONAL MATCH (r)<-[:HAS_REGION]-(from_location:Location)
            OPTIONAL MATCH (arrival_region:Region {id: arrival_region_id})-[rev:EXITS_TO_LOCATION]->(from_location)
            DELETE rel, rev",
        )
        .param("region_id", region_id.to_string())
        .param("location_id", location_id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        Ok(())
    }
}
