//! E2E test helpers for constructing the full application stack.
//!
//! Provides utilities for creating a fully-wired App with real Neo4j repositories
//! and seeded test data for E2E integration testing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use tempfile::TempDir;
use uuid::Uuid;
use wrldbldr_domain::{
    ActId, ChallengeId, CharacterId, LocationId, NarrativeEventId, RegionId, RuleSystemConfig,
    SceneId, WorldId,
};

use crate::app::App;
use crate::infrastructure::clock::FixedClock;
use crate::infrastructure::neo4j::{Neo4jGraph, Neo4jRepositories};
use crate::infrastructure::ports::{
    ClockPort, FinishReason, ImageGenError, ImageGenPort, ImageRequest, ImageResult, LlmError,
    LlmPort, LlmRequest, LlmResponse, QueueError, QueueItem, QueuePort, ToolDefinition,
};
use crate::infrastructure::queue::SqliteQueue;
use crate::infrastructure::settings::SqliteSettingsRepo;
use crate::queue_types::{
    ApprovalRequestData, AssetGenerationData, LlmRequestData, PlayerActionData,
};
use crate::test_fixtures::world_seeder::{load_thornhaven, TestWorld};
use crate::use_cases::content::ContentServiceConfig;

use super::benchmark::{is_benchmark_enabled, BenchmarkLlmDecorator, E2EBenchmark};
use super::event_log::{E2EEventLog, TestOutcome};
use super::logging_queue::LoggingQueue;
use super::neo4j_test_harness::SharedNeo4jHarness;

// =============================================================================
// Seeded World Result
// =============================================================================

/// Result of seeding Thornhaven world to Neo4j.
///
/// Contains all entity IDs for easy lookup during tests.
#[derive(Debug, Clone)]
pub struct SeededWorld {
    pub world_id: WorldId,
    pub location_ids: HashMap<String, LocationId>,
    pub region_ids: HashMap<String, RegionId>,
    pub npc_ids: HashMap<String, CharacterId>,
    pub act_ids: HashMap<String, ActId>,
    pub scene_ids: HashMap<String, SceneId>,
    pub challenge_ids: HashMap<String, ChallengeId>,
    pub event_ids: HashMap<String, NarrativeEventId>,
}

impl SeededWorld {
    /// Get location ID by name.
    pub fn location(&self, name: &str) -> Option<LocationId> {
        self.location_ids.get(name).copied()
    }

    /// Get region ID by name.
    pub fn region(&self, name: &str) -> Option<RegionId> {
        self.region_ids.get(name).copied()
    }

    /// Get NPC character ID by name.
    pub fn npc(&self, name: &str) -> Option<CharacterId> {
        self.npc_ids.get(name).copied()
    }

    /// Get scene ID by name.
    pub fn scene(&self, name: &str) -> Option<SceneId> {
        self.scene_ids.get(name).copied()
    }

    /// Get challenge ID by name.
    pub fn challenge(&self, name: &str) -> Option<ChallengeId> {
        self.challenge_ids.get(name).copied()
    }

    /// Get narrative event ID by name.
    pub fn event(&self, name: &str) -> Option<NarrativeEventId> {
        self.event_ids.get(name).copied()
    }
}

// =============================================================================
// E2E Test Context
// =============================================================================

/// Full E2E test context with application stack and seeded world.
///
/// Uses a shared Neo4j container across all tests for faster execution.
/// Each test gets fresh UUIDs for all entities, ensuring complete isolation.
pub struct E2ETestContext {
    /// Shared Neo4j harness (container is reused across tests)
    pub harness: Arc<SharedNeo4jHarness>,
    /// Graph connection for this test's runtime with optional benchmark timing.
    /// Each test gets its own Graph because neo4rs Graph is tied to the tokio runtime.
    neo4j_graph: Neo4jGraph,
    pub app: App,
    pub world: SeededWorld,
    pub test_world: TestWorld,
    pub clock: Arc<FixedClock>,
    /// Optional event log for comprehensive test analysis.
    pub event_log: Option<Arc<E2EEventLog>>,
    /// Optional benchmark for timing analysis.
    /// Enable via `E2E_BENCHMARK=1` environment variable.
    pub benchmark: Option<Arc<E2EBenchmark>>,
    _temp_dir: TempDir,
}

impl E2ETestContext {
    /// Create a new E2E test context with real Neo4j and mock LLM.
    ///
    /// This is the primary setup method for E2E tests. It:
    /// 1. Starts a Neo4j container
    /// 2. Seeds the Thornhaven test world
    /// 3. Constructs the full App with all use cases
    ///
    /// If `E2E_BENCHMARK=1` is set, timing is tracked automatically.
    pub async fn setup() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::setup_named("unnamed_test", Arc::new(NoopLlm)).await
    }

    /// Create a new E2E test context with a test name for benchmarking.
    ///
    /// Use this when you want benchmark output to identify the test.
    pub async fn setup_named(
        test_name: &str,
        llm: Arc<dyn LlmPort>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::setup_internal(test_name, llm, None).await
    }

    /// Create a new E2E test context with custom LLM implementation.
    pub async fn setup_with_llm(
        llm: Arc<dyn LlmPort>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::setup_internal("unnamed_test", llm, None).await
    }

    /// Create a new E2E test context with event logging (no custom LLM).
    ///
    /// Uses NoopLlm for tests that don't need LLM responses but want event logging.
    pub async fn setup_with_logging(
        event_log: Arc<E2EEventLog>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::setup_internal("unnamed_test", Arc::new(NoopLlm), Some(event_log)).await
    }

    /// Create a new E2E test context with custom LLM and event logging.
    ///
    /// This method enables comprehensive event logging for test analysis.
    /// The event log captures all events, prompts, and responses.
    pub async fn setup_with_llm_and_logging(
        llm: Arc<dyn LlmPort>,
        event_log: Arc<E2EEventLog>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::setup_internal("unnamed_test", llm, Some(event_log)).await
    }

    /// Internal setup method with optional event logging and benchmarking.
    ///
    /// Uses a shared Neo4j container for all tests, with fresh UUIDs per test
    /// to ensure complete isolation without container startup overhead.
    ///
    /// If `E2E_BENCHMARK=1` is set, timing is tracked for:
    /// - Container connection
    /// - World seeding
    /// - App construction
    /// - LLM calls (via decorator)
    async fn setup_internal(
        test_name: &str,
        llm: Arc<dyn LlmPort>,
        event_log: Option<Arc<E2EEventLog>>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create benchmark if enabled
        let benchmark = if is_benchmark_enabled() {
            Some(Arc::new(E2EBenchmark::new(test_name)))
        } else {
            None
        };

        // Start timing container connection
        if let Some(ref b) = benchmark {
            b.start_phase("container");
        }

        // Get shared Neo4j container (started once, reused across all tests)
        let harness = SharedNeo4jHarness::shared().await?;

        if let Some(ref b) = benchmark {
            b.end_phase("container");
            b.start_phase("seed");
        }

        // Load test world from JSON fixtures
        let test_world = load_thornhaven();

        // Create fixed clock for deterministic time
        let now = Utc.with_ymd_and_hms(2026, 1, 12, 9, 0, 0).unwrap(); // Morning
        let clock = Arc::new(FixedClock(now));

        // Create a Graph connection for THIS test's runtime
        // Each test needs its own connection because Graph is tied to the tokio runtime
        let raw_graph = harness.create_graph().await?;

        // Wrap with Neo4jGraph for timing (if benchmarking enabled)
        let neo4j_graph = if let Some(ref b) = benchmark {
            Neo4jGraph::with_benchmark(raw_graph, b.clone())
        } else {
            Neo4jGraph::new(raw_graph)
        };

        // Seed world to Neo4j with FRESH UUIDs for complete test isolation
        // Each test gets its own unique IDs, so tests can run in parallel
        let seeded = seed_thornhaven_to_neo4j(&neo4j_graph, clock.clone(), &test_world).await?;

        if let Some(ref b) = benchmark {
            b.end_phase("seed");
            b.start_phase("app_init");
        }

        // Create temporary directory for SQLite databases
        let temp_dir = TempDir::new()?;
        let queue_db = temp_dir.path().join("queue.db");
        let queue_db_str = queue_db.to_string_lossy().to_string();

        // Create repositories and app using the inner graph (repos don't need timing wrapper)
        let repos = Neo4jRepositories::new(neo4j_graph.clone(), clock.clone());
        let base_queue = Arc::new(SqliteQueue::new(&queue_db_str, clock.clone()).await?);

        // Wrap queue with logging if event_log is provided
        let queue: Arc<dyn QueuePort> = if let Some(ref log) = event_log {
            Arc::new(LoggingQueue::new(base_queue, log.clone()))
        } else {
            base_queue
        };

        // Wrap LLM with benchmark decorator if benchmarking is enabled
        let llm: Arc<dyn LlmPort> = if let Some(ref b) = benchmark {
            Arc::new(BenchmarkLlmDecorator::new(llm, b.clone()))
        } else {
            llm
        };

        let settings_repo = Arc::new(SqliteSettingsRepo::new(&queue_db_str, clock.clone()).await?);
        let image_gen: Arc<dyn ImageGenPort> = Arc::new(NoopImageGen);
        let content_config = ContentServiceConfig::default();

        let app = App::new(repos, llm, image_gen, queue, settings_repo, content_config);

        if let Some(ref b) = benchmark {
            b.end_phase("app_init");
        }

        // Set world ID in event log if logging is enabled
        if let Some(ref log) = event_log {
            log.set_world_id(seeded.world_id);
        }

        Ok(Self {
            harness,
            neo4j_graph,
            app,
            world: seeded,
            test_world,
            clock,
            event_log,
            benchmark,
            _temp_dir: temp_dir,
        })
    }

    /// Get reference to the Neo4j graph connection with benchmark timing.
    ///
    /// This graph is specific to this test's tokio runtime.
    /// When `E2E_BENCHMARK=1`, all queries through this graph are timed.
    pub fn graph(&self) -> &Neo4jGraph {
        &self.neo4j_graph
    }

    /// Print benchmark summary if benchmarking is enabled.
    ///
    /// Call this at the end of a test to see timing breakdown.
    pub fn print_benchmark(&self) {
        if let Some(ref b) = self.benchmark {
            b.print_summary();
        }
    }

    /// Register benchmark results for aggregation across tests.
    ///
    /// Call this at the end of each test when running multiple tests.
    pub fn register_benchmark(&self) {
        if let Some(ref b) = self.benchmark {
            super::benchmark::register_benchmark(b.summary());
        }
    }

    /// Set the clock to a specific time.
    pub fn set_time(&self, hour: u32, minute: u32) {
        let new_time = Utc.with_ymd_and_hms(2026, 1, 12, hour, minute, 0).unwrap();
        // Note: FixedClock is immutable, so tests should create new context
        // or we'd need interior mutability. For now, tests should create
        // context at the desired time.
        let _ = new_time;
    }

    /// Get the current clock time.
    pub fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Log an event to the event log (if logging is enabled).
    pub fn log_event(&self, event: super::event_log::E2EEvent) {
        if let Some(ref log) = self.event_log {
            log.log(event);
        }
    }

    /// Finalize the event log with the test outcome.
    pub fn finalize_event_log(&self, outcome: TestOutcome) {
        if let Some(ref log) = self.event_log {
            log.finalize(outcome);
        }
    }

    /// Save the event log to a file.
    pub fn save_event_log(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(ref log) = self.event_log {
            log.save(path)
        } else {
            Ok(())
        }
    }

    /// Get the default log path for a test.
    pub fn default_log_path(test_name: &str) -> PathBuf {
        PathBuf::from(format!(
            "{}/src/e2e_tests/logs/{}.json",
            env!("CARGO_MANIFEST_DIR"),
            test_name
        ))
    }
}

/// Automatically print benchmark summary when test context is dropped.
///
/// With nextest's `success-output = "immediate"`, this will display
/// timing info right after each test completes.
impl Drop for E2ETestContext {
    fn drop(&mut self) {
        if let Some(ref b) = self.benchmark {
            // Print compact benchmark line to stderr (captured by nextest)
            eprintln!("{}", b.summary().format_compact());
        }
    }
}

// =============================================================================
// World Seeding
// =============================================================================

/// Seed the Thornhaven test world to Neo4j.
///
/// Creates all entities from the JSON fixtures in the database with proper relationships.
///
/// # Fresh UUIDs
///
/// This function generates **fresh UUIDs** for all entities instead of using the fixture IDs.
/// This allows multiple tests to run against the same shared Neo4j container without conflicts.
/// Tests are completely isolated by their unique IDs.
///
/// The fixture data provides structure and names, but IDs are generated per-test:
/// - `WorldId` - Fresh UUID per test
/// - `LocationId` - Fresh UUID per test
/// - `RegionId` - Fresh UUID per test
/// - `CharacterId` - Fresh UUID per test
/// - `ActId`, `SceneId`, `ChallengeId`, `NarrativeEventId` - All fresh per test
pub async fn seed_thornhaven_to_neo4j(
    graph: &Neo4jGraph,
    clock: Arc<dyn ClockPort>,
    test_world: &TestWorld,
) -> Result<SeededWorld, Box<dyn std::error::Error + Send + Sync>> {
    use neo4rs::query;

    // Generate fresh world ID for this test (NOT from fixtures)
    let world_id = WorldId::from(Uuid::new_v4());
    let now = clock.now();

    // Build ID mappings: fixture_id -> fresh_id
    // This allows relationships to be created correctly while using fresh IDs
    let mut location_id_map: HashMap<LocationId, LocationId> = HashMap::new();
    let mut region_id_map: HashMap<RegionId, RegionId> = HashMap::new();
    let mut npc_id_map: HashMap<CharacterId, CharacterId> = HashMap::new();
    let mut act_id_map: HashMap<ActId, ActId> = HashMap::new();

    // Serialize rule_system as JSON (matches WorldRepo::save format)
    let rule_system_json = serde_json::to_string(&RuleSystemConfig::dnd_5e())
        .expect("Failed to serialize rule_system");

    // 1. Create World node
    graph
        .run(
            query(
                "CREATE (w:World {
                    id: $id,
                    name: $name,
                    description: $description,
                    rule_system: $rule_system,
                    created_at: datetime($created_at),
                    updated_at: datetime($updated_at)
                })",
            )
            .param("id", world_id.to_string())
            .param("name", "Thornhaven Village")
            .param("description", "A quaint village for testing")
            .param("rule_system", rule_system_json)
            .param("created_at", now.to_rfc3339())
            .param("updated_at", now.to_rfc3339()),
        )
        .await?;

    // 2. Create Locations with FRESH IDs
    let mut location_ids = HashMap::new();
    for loc in &test_world.locations {
        // Generate fresh ID for this test
        let new_id = LocationId::from(Uuid::new_v4());
        location_id_map.insert(loc.id, new_id);

        graph
            .run(
                query(
                    "CREATE (l:Location {
                        id: $id,
                        world_id: $world_id,
                        name: $name,
                        description: $description,
                        location_type: $location_type,
                        atmosphere: $atmosphere,
                        presence_cache_ttl_hours: $ttl,
                        use_llm_presence: $use_llm
                    })",
                )
                .param("id", new_id.to_string())
                .param("world_id", world_id.to_string())
                .param("name", loc.name.clone())
                .param("description", loc.description.clone())
                .param("location_type", loc.location_type.clone())
                .param("atmosphere", loc.atmosphere.clone())
                .param("ttl", loc.presence_cache_ttl_hours as i64)
                .param("use_llm", loc.use_llm_presence),
            )
            .await?;

        // Create LOCATED_IN relationship to world
        graph
            .run(
                query(
                    "MATCH (l:Location {id: $loc_id}), (w:World {id: $world_id})
                     CREATE (l)-[:LOCATED_IN]->(w)",
                )
                .param("loc_id", new_id.to_string())
                .param("world_id", world_id.to_string()),
            )
            .await?;

        location_ids.insert(loc.name.clone(), new_id);
    }

    // 3. Create Regions with FRESH IDs
    let mut region_ids = HashMap::new();
    for region in &test_world.regions {
        // Generate fresh ID and map the fixture's location_id to the new location ID
        let new_id = RegionId::from(Uuid::new_v4());
        region_id_map.insert(region.id, new_id);
        let new_location_id = location_id_map
            .get(&region.location_id)
            .copied()
            .unwrap_or_else(|| panic!("Location ID not found for region: {}", region.name));

        graph
            .run(
                query(
                    "CREATE (r:Region {
                        id: $id,
                        location_id: $location_id,
                        name: $name,
                        description: $description,
                        atmosphere: $atmosphere,
                        is_spawn_point: $is_spawn,
                        ordering: $ordering
                    })",
                )
                .param("id", new_id.to_string())
                .param("location_id", new_location_id.to_string())
                .param("name", region.name.clone())
                .param("description", region.description.clone())
                .param("atmosphere", region.atmosphere.clone())
                .param("is_spawn", region.is_spawn_point)
                .param("ordering", region.order as i64),
            )
            .await?;

        // Create LOCATED_IN relationship to location
        graph
            .run(
                query(
                    "MATCH (r:Region {id: $region_id}), (l:Location {id: $loc_id})
                     CREATE (r)-[:LOCATED_IN]->(l)",
                )
                .param("region_id", new_id.to_string())
                .param("loc_id", new_location_id.to_string()),
            )
            .await?;

        region_ids.insert(region.name.clone(), new_id);
    }

    // 4. Create Region Connections (using mapped IDs)
    for conn in &test_world.region_connections {
        let from_id = region_id_map
            .get(&conn.from_region_id)
            .copied()
            .unwrap_or_else(|| panic!("From region ID not found for connection"));
        let to_id = region_id_map
            .get(&conn.to_region_id)
            .copied()
            .unwrap_or_else(|| panic!("To region ID not found for connection"));

        graph
            .run(
                query(
                    "MATCH (from:Region {id: $from_id}), (to:Region {id: $to_id})
                     CREATE (from)-[:CONNECTS_TO {
                         description: $description,
                         is_locked: $is_locked
                     }]->(to)",
                )
                .param("from_id", from_id.to_string())
                .param("to_id", to_id.to_string())
                .param("description", conn.description.clone())
                .param("is_locked", conn.is_locked),
            )
            .await?;

        // Create reverse connection if bidirectional
        if conn.bidirectional {
            graph
                .run(
                    query(
                        "MATCH (from:Region {id: $from_id}), (to:Region {id: $to_id})
                         CREATE (to)-[:CONNECTS_TO {
                             description: $description,
                             is_locked: $is_locked
                         }]->(from)",
                    )
                    .param("from_id", from_id.to_string())
                    .param("to_id", to_id.to_string())
                    .param("description", conn.description.clone())
                    .param("is_locked", conn.is_locked),
                )
                .await?;
        }
    }

    // 5. Create NPCs with FRESH IDs
    let mut npc_ids = HashMap::new();
    for npc in &test_world.npcs {
        // Generate fresh ID for this test
        let new_id = CharacterId::from(Uuid::new_v4());
        npc_id_map.insert(npc.id, new_id);

        // Serialize archetype_history and stats as JSON strings
        let archetype_history_json = serde_json::to_string(&Vec::<serde_json::Value>::new())
            .unwrap_or_else(|_| "[]".to_string());
        let stats_json = serde_json::to_string(&npc.stats).unwrap_or_else(|_| "{}".to_string());

        graph
            .run(
                query(
                    "CREATE (c:Character {
                        id: $id,
                        world_id: $world_id,
                        name: $name,
                        description: $description,
                        base_archetype: $base_archetype,
                        current_archetype: $current_archetype,
                        archetype_history: $archetype_history,
                        stats: $stats,
                        is_alive: $is_alive,
                        is_active: $is_active,
                        default_disposition: $disposition,
                        default_mood: $mood,
                        current_hp: $hp,
                        max_hp: $max_hp
                    })",
                )
                .param("id", new_id.to_string())
                .param("world_id", world_id.to_string())
                .param("name", npc.name.clone())
                .param("description", npc.description.clone())
                .param(
                    "base_archetype",
                    format!("{:?}", npc.base_archetype).to_lowercase(),
                )
                .param(
                    "current_archetype",
                    format!("{:?}", npc.current_archetype).to_lowercase(),
                )
                .param("archetype_history", archetype_history_json)
                .param("stats", stats_json)
                .param("is_alive", npc.is_alive)
                .param("is_active", npc.is_active)
                .param(
                    "disposition",
                    format!("{:?}", npc.default_disposition).to_lowercase(),
                )
                .param("mood", format!("{:?}", npc.default_mood).to_lowercase())
                .param("hp", npc.stats.current_hp as i64)
                .param("max_hp", npc.stats.max_hp as i64),
            )
            .await?;

        // Create BELONGS_TO relationship to world
        graph
            .run(
                query(
                    "MATCH (c:Character {id: $char_id}), (w:World {id: $world_id})
                     CREATE (c)-[:BELONGS_TO]->(w)",
                )
                .param("char_id", new_id.to_string())
                .param("world_id", world_id.to_string()),
            )
            .await?;

        npc_ids.insert(npc.name.clone(), new_id);
    }

    // 6. Create NPC-Region relationships (WORKS_AT) using mapped IDs
    for works_at in &test_world.works_at {
        // Find the region for this location and map to fresh IDs
        if let Some(loc_data) = test_world
            .locations
            .iter()
            .find(|l| l.id == works_at.location_id)
        {
            if let Some(default_region) = loc_data.regions.first() {
                let char_id = npc_id_map
                    .get(&works_at.character_id)
                    .copied()
                    .unwrap_or_else(|| panic!("Character ID not found for works_at"));
                let region_id = region_id_map
                    .get(&default_region.id)
                    .copied()
                    .unwrap_or_else(|| panic!("Region ID not found for works_at"));

                graph
                    .run(
                        query(
                            "MATCH (c:Character {id: $char_id}), (r:Region {id: $region_id})
                             CREATE (c)-[:WORKS_AT {
                                 role: $role,
                                 shift: $schedule
                             }]->(r)",
                        )
                        .param("char_id", char_id.to_string())
                        .param("region_id", region_id.to_string())
                        .param("role", works_at.role.clone())
                        .param("schedule", works_at.schedule.clone()),
                    )
                    .await?;
            }
        }
    }

    // 7. Create FREQUENTS relationships using mapped IDs
    for freq in &test_world.frequents {
        if let Some(loc_data) = test_world
            .locations
            .iter()
            .find(|l| l.id == freq.location_id)
        {
            if let Some(default_region) = loc_data.regions.first() {
                let char_id = npc_id_map
                    .get(&freq.character_id)
                    .copied()
                    .unwrap_or_else(|| panic!("Character ID not found for frequents"));
                let region_id = region_id_map
                    .get(&default_region.id)
                    .copied()
                    .unwrap_or_else(|| panic!("Region ID not found for frequents"));

                graph
                    .run(
                        query(
                            "MATCH (c:Character {id: $char_id}), (r:Region {id: $region_id})
                             CREATE (c)-[:FREQUENTS {
                                 frequency: $frequency,
                                 time_of_day: $time_of_day
                             }]->(r)",
                        )
                        .param("char_id", char_id.to_string())
                        .param("region_id", region_id.to_string())
                        .param("frequency", freq.frequency.clone())
                        .param("time_of_day", freq.time_of_day.clone()),
                    )
                    .await?;
            }
        }
    }

    // 8. Create Acts with FRESH IDs
    let mut act_ids = HashMap::new();
    for act in &test_world.acts {
        let new_id = ActId::from(Uuid::new_v4());
        act_id_map.insert(act.id, new_id);

        graph
            .run(
                query(
                    "CREATE (a:Act {
                        id: $id,
                        world_id: $world_id,
                        name: $name,
                        description: $description,
                        stage: $stage,
                        ordering: $ordering
                    })",
                )
                .param("id", new_id.to_string())
                .param("world_id", world_id.to_string())
                .param("name", act.name.clone())
                .param("description", act.description.clone())
                .param("stage", format!("{:?}", act.stage))
                .param("ordering", act.order as i64),
            )
            .await?;

        // Create PART_OF relationship to world
        graph
            .run(
                query(
                    "MATCH (a:Act {id: $act_id}), (w:World {id: $world_id})
                     CREATE (a)-[:PART_OF]->(w)",
                )
                .param("act_id", new_id.to_string())
                .param("world_id", world_id.to_string()),
            )
            .await?;

        act_ids.insert(act.name.clone(), new_id);
    }

    // 9. Create Scenes with FRESH IDs
    let mut scene_ids = HashMap::new();
    for scene in &test_world.scenes {
        let new_id = SceneId::from(Uuid::new_v4());
        let new_act_id = act_id_map
            .get(&scene.act_id)
            .copied()
            .unwrap_or_else(|| panic!("Act ID not found for scene: {}", scene.name));
        let new_location_id = location_id_map
            .get(&scene.location_id)
            .copied()
            .unwrap_or_else(|| panic!("Location ID not found for scene: {}", scene.name));

        graph
            .run(
                query(
                    "CREATE (s:Scene {
                        id: $id,
                        act_id: $act_id,
                        name: $name,
                        location_id: $location_id,
                        directorial_notes: $notes,
                        ordering: $ordering
                    })",
                )
                .param("id", new_id.to_string())
                .param("act_id", new_act_id.to_string())
                .param("name", scene.name.clone())
                .param("location_id", new_location_id.to_string())
                .param("notes", scene.directorial_notes.clone())
                .param("ordering", scene.order as i64),
            )
            .await?;

        // Create PART_OF relationship to act
        graph
            .run(
                query(
                    "MATCH (s:Scene {id: $scene_id}), (a:Act {id: $act_id})
                     CREATE (s)-[:PART_OF]->(a)",
                )
                .param("scene_id", new_id.to_string())
                .param("act_id", new_act_id.to_string()),
            )
            .await?;

        scene_ids.insert(scene.name.clone(), new_id);
    }

    // 10. Create Challenges with FRESH IDs
    let mut challenge_ids = HashMap::new();
    for challenge in &test_world.challenges {
        let new_id = ChallengeId::from(Uuid::new_v4());

        // Serialize difficulty, outcomes, and trigger_conditions as JSON
        let difficulty_json = serde_json::to_string(&challenge.difficulty)
            .unwrap_or_else(|_| r#"{"DC":15}"#.to_string());
        let outcomes_json = serde_json::to_string(&challenge.outcomes)
            .unwrap_or_else(|_| r#"{"success":{"description":"Success","triggers":[]},"failure":{"description":"Failure","triggers":[]}}"#.to_string());
        let triggers_json = serde_json::to_string(&challenge.trigger_conditions)
            .unwrap_or_else(|_| "[]".to_string());
        let tags_json = serde_json::to_string(&challenge.tags).unwrap_or_else(|_| "[]".to_string());

        graph
            .run(
                query(
                    "CREATE (ch:Challenge {
                        id: $id,
                        world_id: $world_id,
                        name: $name,
                        description: $description,
                        challenge_type: $challenge_type,
                        difficulty_json: $difficulty_json,
                        outcomes_json: $outcomes_json,
                        triggers_json: $triggers_json,
                        check_stat: $check_stat,
                        is_active: $active,
                        ordering: $ordering,
                        is_favorite: $favorite,
                        tags_json: $tags_json
                    })",
                )
                .param("id", new_id.to_string())
                .param("world_id", world_id.to_string())
                .param("name", challenge.name.clone())
                .param("description", challenge.description.clone())
                .param("challenge_type", challenge.challenge_type.clone())
                .param("difficulty_json", difficulty_json)
                .param("outcomes_json", outcomes_json)
                .param("triggers_json", triggers_json)
                .param(
                    "check_stat",
                    challenge.check_stat.clone().unwrap_or_default(),
                )
                .param("active", challenge.active)
                .param("ordering", challenge.order as i64)
                .param("favorite", challenge.is_favorite)
                .param("tags_json", tags_json),
            )
            .await?;

        challenge_ids.insert(challenge.name.clone(), new_id);
    }

    // 11. Create Narrative Events with FRESH IDs
    let mut event_ids = HashMap::new();
    for event in &test_world.narrative_events {
        let new_id = NarrativeEventId::from(Uuid::new_v4());

        graph
            .run(
                query(
                    "CREATE (e:NarrativeEvent {
                        id: $id,
                        world_id: $world_id,
                        name: $name,
                        description: $description,
                        scene_direction: $scene_direction,
                        is_active: $is_active,
                        is_triggered: $is_triggered,
                        is_repeatable: $is_repeatable,
                        priority: $priority,
                        is_favorite: $is_favorite
                    })",
                )
                .param("id", new_id.to_string())
                .param("world_id", world_id.to_string())
                .param("name", event.name.clone())
                .param("description", event.description.clone())
                .param("scene_direction", event.scene_direction.clone())
                .param("is_active", event.is_active)
                .param("is_triggered", event.is_triggered)
                .param("is_repeatable", event.is_repeatable)
                .param("priority", event.priority as i64)
                .param("is_favorite", event.is_favorite),
            )
            .await?;

        event_ids.insert(event.name.clone(), new_id);
    }

    Ok(SeededWorld {
        world_id,
        location_ids,
        region_ids,
        npc_ids,
        act_ids,
        scene_ids,
        challenge_ids,
        event_ids,
    })
}

/// Create a test player character in the world.
pub async fn create_test_player(
    graph: &Neo4jGraph,
    world_id: WorldId,
    starting_region_id: RegionId,
    name: &str,
) -> Result<(String, PlayerCharacterId), Box<dyn std::error::Error + Send + Sync>> {
    use neo4rs::query;

    let user_id = Uuid::new_v4().to_string();
    let character_id = PlayerCharacterId::from(Uuid::new_v4());
    let now = Utc::now();

    // Get location ID from region
    let mut result = graph
        .execute(
            query(
                "MATCH (r:Region {id: $region_id})-[:LOCATED_IN]->(l:Location)
                 RETURN l.id as location_id",
            )
            .param("region_id", starting_region_id.to_string()),
        )
        .await?;

    let location_id: String = result
        .next()
        .await?
        .ok_or("Region has no location")?
        .get("location_id")
        .map_err(|e| format!("Failed to get location_id: {}", e))?;

    // Create PlayerCharacter node with all required fields
    graph
        .run(
            query(
                "CREATE (pc:PlayerCharacter {
                    id: $id,
                    user_id: $user_id,
                    world_id: $world_id,
                    name: $name,
                    description: '',
                    sheet_data: '{}',
                    current_location_id: $location_id,
                    current_region_id: $region_id,
                    starting_location_id: $location_id,
                    sprite_asset: '',
                    portrait_asset: '',
                    is_alive: true,
                    is_active: true,
                    created_at: $created_at,
                    last_active_at: $last_active_at
                })",
            )
            .param("id", character_id.to_string())
            .param("user_id", user_id.clone())
            .param("world_id", world_id.to_string())
            .param("name", name)
            .param("location_id", location_id.clone())
            .param("region_id", starting_region_id.to_string())
            .param("created_at", now.to_rfc3339())
            .param("last_active_at", now.to_rfc3339()),
        )
        .await?;

    // Create IN_WORLD relationship
    graph
        .run(
            query(
                "MATCH (pc:PlayerCharacter {id: $pc_id}), (w:World {id: $world_id})
                 MERGE (pc)-[:IN_WORLD]->(w)",
            )
            .param("pc_id", character_id.to_string())
            .param("world_id", world_id.to_string()),
        )
        .await?;

    // Create AT_LOCATION relationship
    graph
        .run(
            query(
                "MATCH (pc:PlayerCharacter {id: $pc_id}), (l:Location {id: $location_id})
                 MERGE (pc)-[:AT_LOCATION]->(l)",
            )
            .param("pc_id", character_id.to_string())
            .param("location_id", location_id.clone()),
        )
        .await?;

    // Create CURRENTLY_IN relationship to Region
    graph
        .run(
            query(
                "MATCH (pc:PlayerCharacter {id: $pc_id}), (r:Region {id: $region_id})
                 MERGE (pc)-[:CURRENTLY_IN]->(r)",
            )
            .param("pc_id", character_id.to_string())
            .param("region_id", starting_region_id.to_string()),
        )
        .await?;

    Ok((user_id, character_id))
}

// =============================================================================
// Mock Implementations
// =============================================================================

/// No-op LLM for tests that don't need LLM responses.
pub struct NoopLlm;

#[async_trait::async_trait]
impl LlmPort for NoopLlm {
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            content: "Mock LLM response".to_string(),
            tool_calls: vec![],
            finish_reason: FinishReason::Stop,
            usage: None,
        })
    }

    async fn generate_with_tools(
        &self,
        _request: LlmRequest,
        _tools: Vec<ToolDefinition>,
    ) -> Result<LlmResponse, LlmError> {
        Ok(LlmResponse {
            content: "Mock LLM response".to_string(),
            tool_calls: vec![],
            finish_reason: FinishReason::Stop,
            usage: None,
        })
    }
}

/// No-op Image Generator for tests.
pub struct NoopImageGen;

#[async_trait::async_trait]
impl ImageGenPort for NoopImageGen {
    async fn generate(&self, _request: ImageRequest) -> Result<ImageResult, ImageGenError> {
        Err(ImageGenError::Unavailable)
    }

    async fn check_health(&self) -> Result<bool, ImageGenError> {
        Ok(false)
    }
}

/// Recording queue that captures enqueued items for assertions.
#[derive(Default)]
pub struct RecordingQueue {
    player_actions: std::sync::Mutex<Vec<PlayerActionData>>,
    llm_requests: std::sync::Mutex<Vec<LlmRequestData>>,
    approvals: std::sync::Mutex<Vec<ApprovalRequestData>>,
}

impl RecordingQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn player_actions(&self) -> Vec<PlayerActionData> {
        self.player_actions.lock().unwrap().clone()
    }

    pub fn llm_requests(&self) -> Vec<LlmRequestData> {
        self.llm_requests.lock().unwrap().clone()
    }

    pub fn approvals(&self) -> Vec<ApprovalRequestData> {
        self.approvals.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl QueuePort for RecordingQueue {
    async fn enqueue_player_action(&self, data: &PlayerActionData) -> Result<Uuid, QueueError> {
        self.player_actions.lock().unwrap().push(data.clone());
        Ok(Uuid::new_v4())
    }

    async fn dequeue_player_action(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_llm_request(&self, data: &LlmRequestData) -> Result<Uuid, QueueError> {
        self.llm_requests.lock().unwrap().push(data.clone());
        Ok(Uuid::new_v4())
    }

    async fn dequeue_llm_request(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_dm_approval(&self, data: &ApprovalRequestData) -> Result<Uuid, QueueError> {
        self.approvals.lock().unwrap().push(data.clone());
        Ok(Uuid::new_v4())
    }

    async fn dequeue_dm_approval(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn enqueue_asset_generation(
        &self,
        _data: &AssetGenerationData,
    ) -> Result<Uuid, QueueError> {
        Ok(Uuid::new_v4())
    }

    async fn dequeue_asset_generation(&self) -> Result<Option<QueueItem>, QueueError> {
        Ok(None)
    }

    async fn mark_complete(&self, _id: Uuid) -> Result<(), QueueError> {
        Ok(())
    }

    async fn mark_failed(&self, _id: Uuid, _error: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn get_pending_count(&self, _queue_type: &str) -> Result<usize, QueueError> {
        Ok(0)
    }

    async fn list_by_type(
        &self,
        _queue_type: &str,
        _limit: usize,
    ) -> Result<Vec<QueueItem>, QueueError> {
        Ok(vec![])
    }

    async fn set_result_json(&self, _id: Uuid, _result_json: &str) -> Result<(), QueueError> {
        Ok(())
    }

    async fn cancel_pending_llm_request_by_callback_id(
        &self,
        _callback_id: &str,
    ) -> Result<bool, QueueError> {
        Ok(false)
    }

    async fn get_approval_request(
        &self,
        _id: Uuid,
    ) -> Result<Option<ApprovalRequestData>, QueueError> {
        Ok(None)
    }

    async fn get_generation_read_state(
        &self,
        _user_id: &str,
        _world_id: WorldId,
    ) -> Result<Option<(Vec<String>, Vec<String>)>, QueueError> {
        Ok(None)
    }

    async fn upsert_generation_read_state(
        &self,
        _user_id: &str,
        _world_id: WorldId,
        _read_batches: &[String],
        _read_suggestions: &[String],
    ) -> Result<(), QueueError> {
        Ok(())
    }

    async fn delete_by_callback_id(&self, _callback_id: &str) -> Result<bool, QueueError> {
        Ok(false)
    }
}

// =============================================================================
// VCR LLM Helpers
// =============================================================================

use super::vcr_llm::VcrLlm;

/// Create a VCR LLM for E2E testing based on environment.
///
/// Uses E2E_LLM_MODE environment variable:
/// - "record": Call real Ollama, save responses to cassette
/// - "playback" or unset: Load from cassette (falls back to record if missing)
/// - "live": Call real Ollama without recording
///
/// Cassettes are stored in `src/e2e_tests/cassettes/<test_name>.json`.
pub fn create_e2e_llm(test_name: &str) -> Arc<VcrLlm> {
    let cassette_path = PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        test_name
    ));
    Arc::new(VcrLlm::from_env(cassette_path))
}

// =============================================================================
// Gameplay Flow Helpers
// =============================================================================

use wrldbldr_domain::{PlayerCharacterId, StagingSource};

use crate::queue_types::DmApprovalDecision;

use crate::use_cases::staging::{ApproveStagingInput, ApprovedNpc};

/// Stage an NPC in a region (simulating DM approval).
pub async fn approve_staging_with_npc(
    ctx: &E2ETestContext,
    region_id: RegionId,
    npc_id: CharacterId,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let npc = ctx
        .app
        .repositories
        .character
        .get(npc_id)
        .await?
        .ok_or("NPC not found")?;

    let input = ApproveStagingInput {
        region_id,
        location_id: None,
        world_id: ctx.world.world_id,
        approved_by: "e2e-test".to_string(),
        ttl_hours: 24,
        source: StagingSource::DmCustomized,
        approved_npcs: vec![ApprovedNpc {
            character_id: npc_id,
            is_present: true,
            reasoning: Some("E2E test staging".to_string()),
            is_hidden_from_players: false,
            mood: Some(format!("{:?}", npc.default_mood()).to_lowercase()),
        }],
        location_state_id: None,
        region_state_id: None,
    };

    ctx.app.use_cases.staging.approve.execute(input).await?;
    Ok(())
}

/// Stage multiple NPCs in a region.
pub async fn approve_staging_with_npcs(
    ctx: &E2ETestContext,
    region_id: RegionId,
    npc_ids: &[CharacterId],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut approved_npcs = Vec::new();

    for &npc_id in npc_ids {
        let npc = ctx
            .app
            .repositories
            .character
            .get(npc_id)
            .await?
            .ok_or("NPC not found")?;

        approved_npcs.push(ApprovedNpc {
            character_id: npc_id,
            is_present: true,
            reasoning: Some("E2E test staging".to_string()),
            is_hidden_from_players: false,
            mood: Some(format!("{:?}", npc.default_mood()).to_lowercase()),
        });
    }

    let input = ApproveStagingInput {
        region_id,
        location_id: None,
        world_id: ctx.world.world_id,
        approved_by: "e2e-test".to_string(),
        ttl_hours: 24,
        source: StagingSource::DmCustomized,
        approved_npcs,
        location_state_id: None,
        region_state_id: None,
    };

    ctx.app.use_cases.staging.approve.execute(input).await?;
    Ok(())
}

/// Create a player character via the management use case.
pub async fn create_player_character_via_use_case(
    ctx: &E2ETestContext,
    name: &str,
    user_id: &str,
) -> Result<PlayerCharacterId, Box<dyn std::error::Error + Send + Sync>> {
    // Get spawn region
    let spawn_region = ctx
        .world
        .region("Common Room")
        .ok_or("Spawn region not found")?;

    // Create via management use case
    // Signature: create(world_id, name, user_id, starting_region_id, sheet_data)
    let pc = ctx
        .app
        .use_cases
        .management
        .player_character
        .create(
            ctx.world.world_id,
            name.to_string(),
            Some(user_id.to_string()),
            Some(spawn_region),
            None, // sheet_data
        )
        .await?;

    Ok(pc.id())
}

/// Run a complete conversation turn through the queue pipeline.
///
/// Returns the final dialogue after DM approval.
/// `turn_number` is the turn number for logging (player turn number, NPC will be turn_number + 1).
pub async fn run_conversation_turn(
    ctx: &E2ETestContext,
    pc_id: PlayerCharacterId,
    npc_id: CharacterId,
    player_id: &str,
    dialogue: &str,
    conversation_id: Option<Uuid>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use super::event_log::E2EEvent;

    // Get NPC name for logging
    let npc = ctx
        .app
        .repositories
        .character
        .get(npc_id)
        .await?
        .ok_or("NPC not found")?;

    // Log player turn (we don't know exact turn number in continue, use 0 as placeholder)
    if let Some(conv_id) = conversation_id {
        ctx.log_event(E2EEvent::ConversationTurn {
            id: conv_id,
            speaker: "player".to_string(),
            content: dialogue.to_string(),
            turn_number: 0, // Turn number not tracked in continue
        });
    }

    // 1. Continue conversation - enqueues player action
    ctx.app
        .use_cases
        .conversation
        .continue_conversation
        .execute(
            ctx.world.world_id,
            pc_id,
            npc_id,
            player_id.to_string(),
            dialogue.to_string(),
            conversation_id,
        )
        .await?;

    // 2. Process player action queue
    let _processed = ctx
        .app
        .use_cases
        .queues
        .process_player_action
        .execute()
        .await?;

    // 3. Process LLM request queue
    let llm_result = ctx
        .app
        .use_cases
        .queues
        .process_llm_request
        .execute(|_| {}) // on_start callback
        .await?;

    // 4. If we got a result, approve it
    if let Some(result) = llm_result {
        // Log approval decision
        ctx.log_event(E2EEvent::ApprovalDecision {
            id: result.approval_id,
            decision: "Accept".to_string(),
            modified: false,
            dm_feedback: None,
        });

        let approval = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(result.approval_id, DmApprovalDecision::Accept)
            .await?;

        let npc_response = approval.final_dialogue.unwrap_or_default();

        // Log NPC response turn
        if let Some(conv_id) = conversation_id {
            ctx.log_event(E2EEvent::ConversationTurn {
                id: conv_id,
                speaker: npc.name().to_string(),
                content: npc_response.clone(),
                turn_number: 0, // Turn number not tracked in continue
            });
        }

        Ok(npc_response)
    } else {
        Err("No LLM request was processed".into())
    }
}

/// Start a conversation with an NPC and process through approval.
///
/// Returns (conversation_id, npc_response).
pub async fn start_conversation_with_npc(
    ctx: &E2ETestContext,
    pc_id: PlayerCharacterId,
    npc_id: CharacterId,
    player_id: &str,
    dialogue: &str,
) -> Result<(Uuid, String), Box<dyn std::error::Error + Send + Sync>> {
    use super::event_log::E2EEvent;

    // Get NPC name for logging
    let npc = ctx
        .app
        .repositories
        .character
        .get(npc_id)
        .await?
        .ok_or("NPC not found")?;

    // 1. Start conversation - enqueues player action
    let started = ctx
        .app
        .use_cases
        .conversation
        .start
        .execute(
            ctx.world.world_id,
            pc_id,
            npc_id,
            player_id.to_string(),
            dialogue.to_string(),
        )
        .await?;

    let conversation_id = started.conversation_id;

    // Log conversation start
    ctx.log_event(E2EEvent::ConversationStarted {
        id: conversation_id,
        pc_id: pc_id.to_string(),
        npc_id: npc_id.to_string(),
        npc_name: npc.name().to_string(),
    });

    // Log player turn
    ctx.log_event(E2EEvent::ConversationTurn {
        id: conversation_id,
        speaker: "player".to_string(),
        content: dialogue.to_string(),
        turn_number: 1,
    });

    // 2. Process player action queue
    let _processed = ctx
        .app
        .use_cases
        .queues
        .process_player_action
        .execute()
        .await?;

    // 3. Process LLM request queue
    let llm_result = ctx
        .app
        .use_cases
        .queues
        .process_llm_request
        .execute(|_| {})
        .await?;

    // 4. If we got a result, approve it
    if let Some(result) = llm_result {
        // Log approval decision
        ctx.log_event(E2EEvent::ApprovalDecision {
            id: result.approval_id,
            decision: "Accept".to_string(),
            modified: false,
            dm_feedback: None,
        });

        let approval = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(result.approval_id, DmApprovalDecision::Accept)
            .await?;

        let npc_response = approval.final_dialogue.unwrap_or_default();

        // Log NPC response turn
        ctx.log_event(E2EEvent::ConversationTurn {
            id: conversation_id,
            speaker: npc.name().to_string(),
            content: npc_response.clone(),
            turn_number: 2,
        });

        Ok((conversation_id, npc_response))
    } else {
        Err("No LLM request was processed".into())
    }
}
