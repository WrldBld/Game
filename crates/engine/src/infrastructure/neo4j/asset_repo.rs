// Asset repo - some helper functions for future use
#![allow(dead_code)]

//! Neo4j asset repository implementation.
//!
//! Handles GalleryAsset persistence for character portraits, location backdrops, etc.

use std::str::FromStr;

use crate::infrastructure::neo4j::Neo4jGraph;
use async_trait::async_trait;
use neo4rs::{query, Node, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wrldbldr_domain::*;

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{AssetRepo, RepoError};

pub struct Neo4jAssetRepo {
    graph: Neo4jGraph,
}

impl Neo4jAssetRepo {
    pub fn new(graph: Neo4jGraph) -> Self {
        Self { graph }
    }
}

#[async_trait]
impl AssetRepo for Neo4jAssetRepo {
    /// Get an asset by ID
    async fn get(&self, id: AssetId) -> Result<Option<GalleryAsset>, RepoError> {
        let q = query("MATCH (a:GalleryAsset {id: $id}) RETURN a").param("id", id.to_string());

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
            Ok(Some(row_to_gallery_asset(row)?))
        } else {
            Ok(None)
        }
    }

    /// Save an asset (upsert) and create relationship to owning entity
    async fn save(&self, asset: &GalleryAsset) -> Result<(), RepoError> {
        let generation_metadata_json = asset
            .generation_metadata()
            .map(|m| serde_json::to_string(&GenerationMetadataStored::from_metadata(m)))
            .transpose()
            .map_err(|e| RepoError::Serialization(e.to_string()))?
            .unwrap_or_default();

        // Upsert the asset node
        let q = query(
            "MERGE (a:GalleryAsset {id: $id})
            ON CREATE SET
                a.entity_type = $entity_type,
                a.entity_id = $entity_id,
                a.asset_type = $asset_type,
                a.file_path = $file_path,
                a.is_active = $is_active,
                a.label = $label,
                a.generation_metadata = $generation_metadata,
                a.created_at = $created_at
            ON MATCH SET
                a.entity_type = $entity_type,
                a.entity_id = $entity_id,
                a.asset_type = $asset_type,
                a.file_path = $file_path,
                a.is_active = $is_active,
                a.label = $label,
                a.generation_metadata = $generation_metadata",
        )
        .param("id", asset.id().to_string())
        .param("entity_type", asset.entity_type().to_string())
        .param("entity_id", asset.entity_id().to_string())
        .param("asset_type", asset.asset_type().to_string())
        .param("file_path", asset.file_path().as_str().to_string())
        .param("is_active", asset.is_active())
        .param("label", asset.label().unwrap_or_default().to_string())
        .param("generation_metadata", generation_metadata_json)
        .param("created_at", asset.created_at().to_rfc3339());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Create relationship to owning entity based on entity_type
        // Only create relationship if asset can have assets
        if asset.entity_type().has_assets() {
            let relationship_query = match asset.entity_type() {
                EntityType::Character => query(
                    "MATCH (e:Character {id: $entity_id}), (a:GalleryAsset {id: $asset_id})
                    MERGE (e)-[:HAS_ASSET]->(a)",
                ),
                EntityType::Location => query(
                    "MATCH (e:Location {id: $entity_id}), (a:GalleryAsset {id: $asset_id})
                    MERGE (e)-[:HAS_ASSET]->(a)",
                ),
                EntityType::Item => query(
                    "MATCH (e:Item {id: $entity_id}), (a:GalleryAsset {id: $asset_id})
                    MERGE (e)-[:HAS_ASSET]->(a)",
                ),
                _ => {
                    return Err(RepoError::constraint(format!(
                        "Entity type {:?} does not support assets",
                        asset.entity_type()
                    )))
                }
            };

            self.graph
                .run(
                    relationship_query
                        .param("entity_id", asset.entity_id().to_string())
                        .param("asset_id", asset.id().to_string()),
                )
                .await
                .map_err(|e| RepoError::database("query", e))?;
        }

        Ok(())
    }

    /// Delete an asset by ID
    async fn delete(&self, id: AssetId) -> Result<(), RepoError> {
        let q = query(
            "MATCH (a:GalleryAsset {id: $id})
            DETACH DELETE a",
        )
        .param("id", id.to_string());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        tracing::debug!("Deleted asset: {}", id);
        Ok(())
    }

    /// List all assets for an entity
    async fn list_for_entity(
        &self,
        entity_type: EntityType,
        entity_id: Uuid,
    ) -> Result<Vec<GalleryAsset>, RepoError> {
        if !entity_type.has_assets() {
            return Err(RepoError::constraint(format!(
                "Entity type {:?} cannot have assets",
                entity_type
            )));
        }

        let q = query(
            "MATCH (a:GalleryAsset {entity_type: $entity_type, entity_id: $entity_id})
            RETURN a
            ORDER BY a.created_at DESC",
        )
        .param("entity_type", entity_type.to_string())
        .param("entity_id", entity_id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("query", e))?;
        let mut assets = Vec::new();

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("query", e))?
        {
            assets.push(row_to_gallery_asset(row)?);
        }

        Ok(assets)
    }

    /// Set an asset as active (deactivates others of same type for same entity)
    /// Uses explicit transaction to ensure atomicity.
    async fn set_active(
        &self,
        entity_type: EntityType,
        entity_id: Uuid,
        asset_id: AssetId,
    ) -> Result<(), RepoError> {
        if !entity_type.has_assets() {
            return Err(RepoError::constraint(format!(
                "Entity type {:?} cannot have assets",
                entity_type
            )));
        }

        // First, get the asset to determine its asset_type
        let asset = self.get(asset_id).await?.ok_or_else(|| {
            RepoError::database("query", format!("Asset not found: {}", asset_id))
        })?;

        // Use single transaction to ensure deactivate and activate are atomic
        let mut txn = self
            .graph
            .start_txn()
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Deactivate all assets of the same type for this entity
        let deactivate_q = query(
            "MATCH (a:GalleryAsset {
                entity_type: $entity_type,
                entity_id: $entity_id,
                asset_type: $asset_type
            })
            SET a.is_active = false",
        )
        .param("entity_type", entity_type.to_string())
        .param("entity_id", entity_id.to_string())
        .param("asset_type", asset.asset_type().to_string());

        txn.run(deactivate_q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Activate the specified asset
        let activate_q = query("MATCH (a:GalleryAsset {id: $id}) SET a.is_active = true")
            .param("id", asset_id.to_string());

        txn.run(activate_q)
            .await
            .map_err(|e| RepoError::database("query", e))?;

        // Commit transaction
        txn.commit()
            .await
            .map_err(|e| RepoError::database("query", e))?;

        Ok(())
    }
}

// =============================================================================
// Row conversion helpers
// =============================================================================

fn row_to_gallery_asset(row: Row) -> Result<GalleryAsset, RepoError> {
    let node: Node = row.get("a").map_err(|e| RepoError::database("query", e))?;

    let id: AssetId = parse_typed_id(&node, "id")
        .map_err(|e| RepoError::database("query", format!("Failed to parse AssetId: {}", e)))?;
    let entity_type_str: String = node.get("entity_type").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'entity_type' for Asset {}: {}", id, e),
        )
    })?;
    let entity_id: String = node.get("entity_id").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'entity_id' for Asset {}: {}", id, e),
        )
    })?;
    let asset_type_str: String = node.get("asset_type").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'asset_type' for Asset {}: {}", id, e),
        )
    })?;
    let file_path_str: String = node.get("file_path").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'file_path' for Asset {}: {}", id, e),
        )
    })?;
    let file_path = AssetPath::new(file_path_str).map_err(|e| {
        RepoError::database(
            "query",
            format!("Invalid asset path for Asset {}: {}", id, e),
        )
    })?;
    let is_active: bool = node.get_bool_or("is_active", false);
    let label = node.get_optional_string("label");
    let created_at_str: String = node.get("created_at").map_err(|e| {
        RepoError::database(
            "query",
            format!("Failed to get 'created_at' for Asset {}: {}", id, e),
        )
    })?;

    let entity_type = parse_entity_type(&entity_type_str)?;
    let asset_type = AssetType::from_str(&asset_type_str).map_err(|e| {
        RepoError::database(
            "query",
            format!("Invalid asset type for Asset {}: {}", id, e),
        )
    })?;

    let generation_metadata: Option<GenerationMetadata> = node
        .get_json_or_default::<Option<GenerationMetadataStored>>("generation_metadata")
        .map(Into::into);

    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
        .map_err(|e| {
            RepoError::database("query", format!("Invalid datetime for Asset {}: {}", id, e))
        })?
        .with_timezone(&chrono::Utc);

    Ok(GalleryAsset::reconstruct(
        id,
        entity_type,
        entity_id,
        asset_type,
        file_path,
        is_active,
        label,
        generation_metadata,
        created_at,
    ))
}

fn parse_entity_type(s: &str) -> Result<EntityType, RepoError> {
    match s {
        "Character" => Ok(EntityType::Character),
        "Location" => Ok(EntityType::Location),
        "Item" => Ok(EntityType::Item),
        _ => Err(RepoError::database(
            "parse",
            format!("Unknown EntityType: '{}'", s),
        )),
    }
}

// =============================================================================
// Stored types for JSON serialization
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GenerationMetadataStored {
    pub workflow: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub seed: i64,
    pub style_reference_id: Option<String>,
    pub batch_id: String,
}

impl GenerationMetadataStored {
    fn from_metadata(value: &GenerationMetadata) -> Self {
        Self {
            workflow: value.workflow.clone(),
            prompt: value.prompt.clone(),
            negative_prompt: value.negative_prompt.clone(),
            seed: value.seed,
            style_reference_id: value.style_reference_id.map(|id| id.to_string()),
            batch_id: value.batch_id.to_string(),
        }
    }
}

impl From<GenerationMetadataStored> for GenerationMetadata {
    fn from(value: GenerationMetadataStored) -> Self {
        let style_reference_id = value
            .style_reference_id
            .and_then(|s| uuid::Uuid::parse_str(&s).ok())
            .map(AssetId::from_uuid);
        let batch_id = uuid::Uuid::parse_str(&value.batch_id)
            .ok()
            .map(BatchId::from_uuid)
            .unwrap_or_default();

        GenerationMetadata {
            workflow: value.workflow,
            prompt: value.prompt,
            negative_prompt: value.negative_prompt,
            seed: value.seed,
            style_reference_id,
            batch_id,
        }
    }
}
