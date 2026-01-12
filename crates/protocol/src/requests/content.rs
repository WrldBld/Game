//! Content request types for fetching game content.
//!
//! Provides a unified content API for accessing races, classes, backgrounds,
//! spells, feats, and other game content through the CompendiumProvider system.

use serde::{Deserialize, Serialize};

/// Filter for content queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentFilterRequest {
    /// Filter by source (e.g., "PHB", "XGE").
    #[serde(default)]
    pub source: Option<String>,

    /// Text search across name, description, and tags.
    #[serde(default)]
    pub search: Option<String>,

    /// Filter by specific tags.
    #[serde(default)]
    pub tags: Option<Vec<String>>,

    /// Maximum number of results.
    #[serde(default)]
    pub limit: Option<usize>,

    /// Offset for pagination.
    #[serde(default)]
    pub offset: Option<usize>,
}

/// Requests for game content operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ContentRequest {
    /// List registered content providers.
    ListProviders,

    /// List content types supported by a system.
    ListContentTypes { system_id: String },

    /// List content of a specific type.
    ListContent {
        system_id: String,
        /// Content type: "origin", "class", "background", "spell", "feat", etc.
        content_type: String,
        #[serde(default)]
        filter: Option<ContentFilterRequest>,
    },

    /// Get a specific content item by ID.
    GetContent {
        system_id: String,
        content_type: String,
        content_id: String,
    },

    /// Search across all content types.
    SearchContent {
        system_id: String,
        query: String,
        #[serde(default = "default_search_limit")]
        limit: usize,
    },

    /// Get content statistics for a system.
    GetContentStats { system_id: String },
}

fn default_search_limit() -> usize {
    20
}
