//! Backend E2E integration tests.
//!
//! These tests validate the full gameplay loop using:
//! - Real Neo4j database (via testcontainers)
//! - Real or VCR-recorded LLM (Ollama)
//! - Complete App construction with all use cases
//!
//! # Running E2E Tests
//!
//! ```bash
//! # Run all E2E tests (requires Docker)
//! cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=1
//!
//! # Run specific test
//! cargo test -p wrldbldr-engine --lib test_thornhaven_world_seeded -- --ignored
//!
//! # Record LLM responses to cassettes (requires Ollama)
//! E2E_LLM_MODE=record cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=1
//!
//! # Playback from cassettes (fast, no Ollama needed)
//! cargo test -p wrldbldr-engine --lib e2e_tests -- --ignored --test-threads=1
//! ```

mod e2e_helpers;
mod gameplay_flow_tests;
mod gameplay_loop_tests;
mod neo4j_test_harness;
mod vcr_llm;

pub use e2e_helpers::*;
pub use neo4j_test_harness::*;
pub use vcr_llm::*;
