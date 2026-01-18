//! Backend E2E integration tests.
//!
//! These tests validate the full gameplay loop using:
//! - Real Neo4j database (via testcontainers) - **shared across all tests**
//! - Real or VCR-recorded LLM (Ollama)
//! - Complete App construction with all use cases
//!
//! # Shared Container Architecture
//!
//! Tests use a **single shared Neo4j container** that is started once and reused
//! across all tests. Each test gets **fresh UUIDs** for all entities, ensuring
//! complete isolation without the overhead of starting a new container per test.
//!
//! This enables:
//! - **Parallel execution**: Tests can run with `--test-threads=4` (or higher)
//! - **Faster runs**: ~60 seconds container startup only happens once
//! - **Complete isolation**: Fresh UUIDs mean no cross-test interference
//!
//! # Running E2E Tests
//!
//! ## Recommended: cargo test (parallel, shared container)
//!
//! Use `cargo test` for e2e tests - it runs tests in a single process, allowing
//! the shared Neo4j container to be reused across all tests via `OnceLock`.
//!
//! ```bash
//! # Run all E2E tests in parallel (requires Docker)
//! cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=4
//!
//! # Run specific test
//! cargo test -p wrldbldr-engine --lib test_thornhaven_world_seeded -- --ignored
//!
//! # Record LLM responses to cassettes (requires Ollama)
//! E2E_LLM_MODE=record cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=1
//!
//! # Playback from cassettes (fast, no Ollama needed)
//! E2E_LLM_MODE=playback cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=4
//!
//! # Run with benchmarking
//! E2E_BENCHMARK=1 cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --nocapture
//! ```
//!
//! ## Alternative: cargo nextest (serial, per-test containers)
//!
//! Nextest runs each test in a separate process, so the shared container doesn't work.
//! Each test starts its own container, making it slower but with better isolation.
//!
//! ```bash
//! # Run with nextest (serial execution, one container per test)
//! E2E_BENCHMARK=1 cargo nextest run -p wrldbldr-engine --lib -E 'test(e2e_tests)' \
//!     --run-ignored all --profile e2e
//! ```
//!
//! # Benchmarking
//!
//! Set `E2E_BENCHMARK=1` to enable timing instrumentation. This tracks:
//! - Total test time
//! - Setup time (container connection, seeding)
//! - LLM call time (via VCR or real)
//! - "Own code" time (total - external calls)

mod approval_timeout_tests;
mod benchmark;
mod challenge_flow_tests;
mod character_stats_tests;
mod disposition_item_tests;
mod e2e_helpers;
mod event_chain_tests;
mod event_log;
mod flag_tests;
mod gameplay_flow_tests;
mod gameplay_loop_tests;
mod location_event_tests;
mod logging_queue;
mod lore_tests;
mod movement_tests;
mod multiplayer_tests;
mod neo4j_test_harness;
mod observations_tests;
mod scene_act_tests;
mod semantic_assert;
mod skills_tests;
mod story_event_tests;
mod structured_output_tests;
mod time_tests;
mod tool_call_tests;
mod tool_execution_tests;
mod trigger_tests;
mod vcr_fingerprint;
mod vcr_llm;

pub use benchmark::*;
pub use e2e_helpers::*;
pub use event_log::*;
pub use logging_queue::*;
pub use neo4j_test_harness::*;
pub use semantic_assert::*;
pub use vcr_llm::*;
