//! World management use cases.

use std::sync::Arc;

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

/// Export world use case.
pub struct ExportWorld {
    #[allow(dead_code)]
    world: Arc<crate::entities::World>,
}

impl ExportWorld {
    pub fn new(world: Arc<crate::entities::World>) -> Self {
        Self { world }
    }

    pub async fn execute(&self, _world_id: wrldbldr_domain::WorldId) -> Result<WorldExport, WorldError> {
        todo!("Export world use case")
    }
}

/// Import world use case.
pub struct ImportWorld {
    #[allow(dead_code)]
    world: Arc<crate::entities::World>,
}

impl ImportWorld {
    pub fn new(world: Arc<crate::entities::World>) -> Self {
        Self { world }
    }

    pub async fn execute(&self, _data: WorldExport) -> Result<wrldbldr_domain::WorldId, WorldError> {
        todo!("Import world use case")
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WorldExport {
    pub world: wrldbldr_domain::World,
    // Would include all related data
}

#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("World not found")]
    NotFound,
    #[error("Export failed: {0}")]
    ExportFailed(String),
    #[error("Import failed: {0}")]
    ImportFailed(String),
}
