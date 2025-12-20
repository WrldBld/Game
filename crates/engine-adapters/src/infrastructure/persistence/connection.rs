//! Neo4j connection management

use anyhow::Result;
use neo4rs::{ConfigBuilder, Graph};

/// Shared Neo4j connection
#[derive(Clone)]
pub struct Neo4jConnection {
    graph: Graph,
}

impl Neo4jConnection {
    pub async fn new(uri: &str, user: &str, password: &str, database: &str) -> Result<Self> {
        let config = ConfigBuilder::default()
            .uri(uri)
            .user(user)
            .password(password)
            .db(database)
            .build()?;

        let graph = Graph::connect(config).await?;
        tracing::info!("Connected to Neo4j at {}", uri);

        Ok(Self { graph })
    }

    /// Get a reference to the graph connection
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Initialize the database schema (create constraints and indexes)
    pub async fn initialize_schema(&self) -> Result<()> {
        // Create uniqueness constraints
        //
        // Neo4j doesn't need "collections" to exist up front, but it *does* benefit
        // from bootstrapping constraints/indexes at startup so a fresh database is
        // immediately usable.
        let constraints = [
            "CREATE CONSTRAINT world_id IF NOT EXISTS FOR (w:World) REQUIRE w.id IS UNIQUE",
            "CREATE CONSTRAINT location_id IF NOT EXISTS FOR (l:Location) REQUIRE l.id IS UNIQUE",
            "CREATE CONSTRAINT region_id IF NOT EXISTS FOR (r:Region) REQUIRE r.id IS UNIQUE",
            "CREATE CONSTRAINT character_id IF NOT EXISTS FOR (c:Character) REQUIRE c.id IS UNIQUE",
            "CREATE CONSTRAINT want_id IF NOT EXISTS FOR (w:Want) REQUIRE w.id IS UNIQUE",
            "CREATE CONSTRAINT player_character_id IF NOT EXISTS FOR (pc:PlayerCharacter) REQUIRE pc.id IS UNIQUE",
            "CREATE CONSTRAINT scene_id IF NOT EXISTS FOR (s:Scene) REQUIRE s.id IS UNIQUE",
            "CREATE CONSTRAINT act_id IF NOT EXISTS FOR (a:Act) REQUIRE a.id IS UNIQUE",
            "CREATE CONSTRAINT interaction_id IF NOT EXISTS FOR (i:Interaction) REQUIRE i.id IS UNIQUE",
            "CREATE CONSTRAINT skill_id IF NOT EXISTS FOR (s:Skill) REQUIRE s.id IS UNIQUE",
            "CREATE CONSTRAINT challenge_id IF NOT EXISTS FOR (c:Challenge) REQUIRE c.id IS UNIQUE",
            "CREATE CONSTRAINT story_event_id IF NOT EXISTS FOR (e:StoryEvent) REQUIRE e.id IS UNIQUE",
            "CREATE CONSTRAINT narrative_event_id IF NOT EXISTS FOR (e:NarrativeEvent) REQUIRE e.id IS UNIQUE",
            "CREATE CONSTRAINT event_chain_id IF NOT EXISTS FOR (c:EventChain) REQUIRE c.id IS UNIQUE",
            "CREATE CONSTRAINT sheet_template_id IF NOT EXISTS FOR (t:SheetTemplate) REQUIRE t.id IS UNIQUE",
            "CREATE CONSTRAINT item_id IF NOT EXISTS FOR (i:Item) REQUIRE i.id IS UNIQUE",
            "CREATE CONSTRAINT grid_map_id IF NOT EXISTS FOR (g:GridMap) REQUIRE g.id IS UNIQUE",
            "CREATE CONSTRAINT staging_id IF NOT EXISTS FOR (s:Staging) REQUIRE s.id IS UNIQUE",
            "CREATE CONSTRAINT gallery_asset_id IF NOT EXISTS FOR (a:GalleryAsset) REQUIRE a.id IS UNIQUE",
            "CREATE CONSTRAINT generation_batch_id IF NOT EXISTS FOR (b:GenerationBatch) REQUIRE b.id IS UNIQUE",
            "CREATE CONSTRAINT workflow_slot IF NOT EXISTS FOR (w:WorkflowConfiguration) REQUIRE w.slot IS UNIQUE",
        ];

        for constraint in constraints {
            if let Err(e) = self.graph.run(neo4rs::query(constraint)).await {
                tracing::warn!("Constraint creation warning: {}", e);
            }
        }

        // Create indexes for common queries
        let indexes = [
            "CREATE INDEX world_name IF NOT EXISTS FOR (w:World) ON (w.name)",
            "CREATE INDEX character_name IF NOT EXISTS FOR (c:Character) ON (c.name)",
            "CREATE INDEX location_name IF NOT EXISTS FOR (l:Location) ON (l.name)",
            "CREATE INDEX character_world IF NOT EXISTS FOR (c:Character) ON (c.world_id)",
            "CREATE INDEX location_world IF NOT EXISTS FOR (l:Location) ON (l.world_id)",
            "CREATE INDEX region_location IF NOT EXISTS FOR (r:Region) ON (r.location_id)",
            "CREATE INDEX skill_world IF NOT EXISTS FOR (s:Skill) ON (s.world_id)",
            "CREATE INDEX challenge_world IF NOT EXISTS FOR (c:Challenge) ON (c.world_id)",
            "CREATE INDEX story_event_world IF NOT EXISTS FOR (e:StoryEvent) ON (e.world_id)",
            "CREATE INDEX narrative_event_world IF NOT EXISTS FOR (e:NarrativeEvent) ON (e.world_id)",
            "CREATE INDEX event_chain_world IF NOT EXISTS FOR (c:EventChain) ON (c.world_id)",
            "CREATE INDEX scene_act IF NOT EXISTS FOR (s:Scene) ON (s.act_id)",
            "CREATE INDEX sheet_template_world IF NOT EXISTS FOR (t:SheetTemplate) ON (t.world_id)",
            "CREATE INDEX player_character_world IF NOT EXISTS FOR (pc:PlayerCharacter) ON (pc.world_id)",
            "CREATE INDEX player_character_session IF NOT EXISTS FOR (pc:PlayerCharacter) ON (pc.session_id)",
            "CREATE INDEX staging_world IF NOT EXISTS FOR (s:Staging) ON (s.world_id)",
            "CREATE INDEX generation_batch_world IF NOT EXISTS FOR (b:GenerationBatch) ON (b.world_id)",
        ];

        for index in indexes {
            if let Err(e) = self.graph.run(neo4rs::query(index)).await {
                tracing::warn!("Index creation warning: {}", e);
            }
        }

        tracing::info!("Database schema initialized");
        Ok(())
    }
}
