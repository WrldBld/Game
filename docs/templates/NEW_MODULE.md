# New Module Templates

Code templates for common module patterns in WrldBldr.

## Use Case Module

```rust
//! [Brief description of what this use case does]
//!
//! [Optional: when this use case is triggered]

use std::sync::Arc;
use thiserror::Error;

use crate::entities::{Entity1, Entity2};
use crate::infrastructure::ports::RepoError;

// =============================================================================
// Error Type
// =============================================================================

/// Error type for [use case name] operations.
#[derive(Debug, Error)]
pub enum MyUseCaseError {
    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Invalid state: {0}")]
    InvalidState(String),

    #[error(transparent)]
    Repo(#[from] RepoError),
}

// =============================================================================
// Input/Output Types
// =============================================================================

/// Input for the operation.
pub struct MyOperationInput {
    pub entity_id: EntityId,
    pub optional_field: Option<String>,
}

/// Result of the operation.
pub struct MyOperationResult {
    pub entity: Entity,
    pub changes_made: Vec<String>,
}

// =============================================================================
// Use Case
// =============================================================================

/// Use case for [description].
pub struct MyOperation {
    entity1: Arc<Entity1>,
    entity2: Arc<Entity2>,
}

impl MyOperation {
    pub fn new(entity1: Arc<Entity1>, entity2: Arc<Entity2>) -> Self {
        Self { entity1, entity2 }
    }

    /// Execute the operation.
    ///
    /// # Arguments
    /// * `input` - The operation input
    ///
    /// # Returns
    /// * `Ok(MyOperationResult)` - Operation succeeded
    /// * `Err(MyUseCaseError)` - Operation failed
    pub async fn execute(
        &self,
        input: MyOperationInput,
    ) -> Result<MyOperationResult, MyUseCaseError> {
        // 1. Validate inputs
        let entity = self
            .entity1
            .get(input.entity_id)
            .await?
            .ok_or_else(|| MyUseCaseError::NotFound(input.entity_id.to_string()))?;

        // 2. Perform business logic
        // ...

        // 3. Persist changes
        // ...

        // 4. Return result
        Ok(MyOperationResult {
            entity,
            changes_made: vec![],
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::ports::{MockEntity1Repo, MockEntity2Repo};

    fn create_use_case(
        entity1_repo: MockEntity1Repo,
        entity2_repo: MockEntity2Repo,
    ) -> MyOperation {
        MyOperation::new(
            Arc::new(Entity1::new(Arc::new(entity1_repo))),
            Arc::new(Entity2::new(Arc::new(entity2_repo))),
        )
    }

    #[tokio::test]
    async fn when_entity_not_found_returns_error() {
        let mut repo = MockEntity1Repo::new();
        repo.expect_get().returning(|_| Ok(None));

        let use_case = create_use_case(repo, MockEntity2Repo::new());
        let input = MyOperationInput {
            entity_id: EntityId::new(),
            optional_field: None,
        };

        let result = use_case.execute(input).await;
        assert!(matches!(result, Err(MyUseCaseError::NotFound(_))));
    }

    #[tokio::test]
    async fn when_valid_input_succeeds() {
        // ... test happy path
    }
}
```

---

## Entity Module

```rust
//! [Entity name] operations.
//!
//! Encapsulates all operations for [entity] domain objects.

use std::sync::Arc;

use wrldbldr_domain::{Entity, EntityId};

use crate::infrastructure::ports::{EntityRepo, RepoError};

/// [Entity] operations.
pub struct EntityOps {
    repo: Arc<dyn EntityRepo>,
}

impl EntityOps {
    pub fn new(repo: Arc<dyn EntityRepo>) -> Self {
        Self { repo }
    }

    /// Get an entity by ID.
    pub async fn get(&self, id: EntityId) -> Result<Option<Entity>, RepoError> {
        self.repo.get(id).await
    }

    /// Save an entity.
    pub async fn save(&self, entity: &Entity) -> Result<(), RepoError> {
        self.repo.save(entity).await
    }

    /// Delete an entity.
    pub async fn delete(&self, id: EntityId) -> Result<(), RepoError> {
        self.repo.delete(id).await
    }

    /// List entities by criteria.
    pub async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Entity>, RepoError> {
        self.repo.list_by_world(world_id).await
    }
}
```

---

## Repository Port Trait

```rust
// In crates/engine/src/infrastructure/ports.rs

/// Repository for [Entity] persistence.
#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait EntityRepo: Send + Sync {
    /// Get entity by ID.
    async fn get(&self, id: EntityId) -> Result<Option<Entity>, RepoError>;

    /// Save entity (upsert).
    async fn save(&self, entity: &Entity) -> Result<(), RepoError>;

    /// Delete entity.
    async fn delete(&self, id: EntityId) -> Result<(), RepoError>;

    /// List entities in a world.
    async fn list_by_world(&self, world_id: WorldId) -> Result<Vec<Entity>, RepoError>;
}
```

---

## Neo4j Repository Implementation

```rust
//! Neo4j [entity] repository implementation.

use std::sync::Arc;

use async_trait::async_trait;
use neo4rs::{query, Graph, Node, Row};
use wrldbldr_domain::*;

use super::helpers::{parse_typed_id, NodeExt};
use crate::infrastructure::ports::{EntityRepo, RepoError};

pub struct Neo4jEntityRepo {
    graph: Graph,
}

impl Neo4jEntityRepo {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }
}

#[async_trait]
impl EntityRepo for Neo4jEntityRepo {
    async fn get(&self, id: EntityId) -> Result<Option<Entity>, RepoError> {
        let q = query("MATCH (e:Entity {id: $id}) RETURN e")
            .param("id", id.to_string());

        let mut result = self
            .graph
            .execute(q)
            .await
            .map_err(|e| RepoError::database("get", e))?;

        if let Some(row) = result
            .next()
            .await
            .map_err(|e| RepoError::database("get", e))?
        {
            Ok(Some(row_to_entity(row)?))
        } else {
            Ok(None)
        }
    }

    async fn save(&self, entity: &Entity) -> Result<(), RepoError> {
        let q = query(
            "MERGE (e:Entity {id: $id})
            ON CREATE SET
                e.name = $name,
                e.created_at = $created_at
            ON MATCH SET
                e.name = $name",
        )
        .param("id", entity.id.to_string())
        .param("name", entity.name.clone())
        .param("created_at", entity.created_at.to_rfc3339());

        self.graph
            .run(q)
            .await
            .map_err(|e| RepoError::database("save", e))?;

        Ok(())
    }

    // ... other methods
}

fn row_to_entity(row: Row) -> Result<Entity, RepoError> {
    let node: Node = row
        .get("e")
        .map_err(|e| RepoError::database("parse", e))?;

    let id = parse_typed_id(&node, "id")
        .map_err(|e| RepoError::database("parse", e))?;

    let name: String = node
        .get("name")
        .map_err(|e| RepoError::database("parse", e))?;

    Ok(Entity { id, name })
}
```

---

## WebSocket Handler Module

```rust
//! WebSocket handlers for [feature] requests.

use super::*;

pub(super) async fn handle_my_request(
    state: &WsState,
    session: &mut WsSession,
    payload: MyRequestPayload,
) -> Result<serde_json::Value, ServerMessage> {
    let world_id = session.world_id.ok_or_else(|| error_msg(ErrorCode::NotInWorld))?;

    // Parse IDs
    let entity_id = payload.entity_id.parse::<Uuid>()
        .map(EntityId::from)
        .map_err(|_| error_msg(ErrorCode::InvalidRequest))?;

    // Call use case
    let result = state.app.use_cases.my_feature
        .execute(entity_id)
        .await
        .map_err(|e| error_msg_str(ErrorCode::InternalError, e.to_string()))?;

    // Return response
    Ok(serde_json::to_value(&result).unwrap())
}
```

---

## Checklist for New Modules

- [ ] Error type with context (entity type, ID)
- [ ] Public API documented with doc comments
- [ ] Unit tests for happy path and error cases
- [ ] Added to parent `mod.rs` exports
- [ ] Wired into `App` struct if use case
- [ ] Added to WebSocket handler dispatch if needed
