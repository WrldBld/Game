//! Content source implementations for external data providers.
//!
//! This module contains infrastructure implementations for loading game content
//! from external sources such as 5etools, D&D Beyond, etc. These implementations
//! provide concrete types that satisfy the `CompendiumProvider` trait.

pub mod fivetools;

pub use fivetools::{Dnd5eContentProvider, FiveToolsImporter, ImportError};
