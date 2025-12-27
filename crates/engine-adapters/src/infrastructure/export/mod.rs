//! World export functionality
//!
//! This module provides export capabilities for worlds,
//! allowing them to be serialized to JSON for the Player to consume.
//!
//! The primary export format is `PlayerWorldSnapshot`, a streamlined snapshot
//! for real-time Player client transmission (defined in application/ports/outbound/world_exporter_port.rs).

pub(crate) mod world_snapshot;

pub use world_snapshot::Neo4jWorldExporter;
