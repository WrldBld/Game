//! Backend E2E integration tests.
//!
//! These tests validate the full gameplay loop using:
//! - Real Neo4j database (via testcontainers)
//! - Real or mock LLM (Ollama)
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
//! ```

mod e2e_helpers;
mod gameplay_loop_tests;
mod neo4j_test_harness;

pub use e2e_helpers::*;
pub use neo4j_test_harness::*;
