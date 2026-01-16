//! Lore entity - Historical knowledge with discoverable chunks
//!
//! Lore represents world knowledge that characters can discover in pieces.
//! Each lore entry has chunks that can be discovered individually or together.
//!
//! # Neo4j Relationships
//! - `(Lore)-[:ABOUT_CHARACTER]->(Character)` - Lore about a character
//! - `(Lore)-[:ABOUT_LOCATION]->(Location)` - Lore about a location
//! - `(Lore)-[:ABOUT_REGION]->(Region)` - Lore about a region
//! - `(Lore)-[:ABOUT_ITEM]->(Item)` - Lore about an item
//! - `(Character)-[:KNOWS_LORE {chunk_ids, source, ...}]->(Lore)` - Character knows lore
//! - `(Location)-[:COMMON_LORE]->(Lore)` - Lore is common knowledge at location

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::{CharacterId, LoreChunkId, LoreId, WorldId};

/// A piece of world knowledge that can be discovered
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lore {
    pub id: LoreId,
    pub world_id: WorldId,

    /// Title of the lore entry (e.g., "The Fall of House Valeren")
    pub title: String,
    /// Brief summary for DM reference
    pub summary: String,
    /// Category of knowledge
    pub category: LoreCategory,

    /// Discoverable pieces of this lore
    pub chunks: Vec<LoreChunk>,

    /// If true, all characters in the world know this lore
    pub is_common_knowledge: bool,

    /// Tags for filtering/searching
    pub tags: Vec<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A discoverable piece of lore
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoreChunk {
    pub id: LoreChunkId,
    /// Display order within the lore entry
    pub order: u32,
    /// Optional title for this chunk
    pub title: Option<String>,
    /// The actual lore content
    pub content: String,
    /// Hint for DM about how this can be discovered
    pub discovery_hint: Option<String>,
}

/// Category of lore
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoreCategory {
    /// Past events (wars, treaties, disasters)
    Historical,
    /// Myths, legends, folklore
    Legend,
    /// Hidden knowledge (conspiracies, true origins)
    Secret,
    /// Widely known information
    Common,
    /// How things work (magic systems, technology)
    Technical,
    /// Factions, alliances, political structures
    Political,
    /// Geography, flora/fauna, natural phenomena
    Natural,
    /// Religious beliefs, prophecies
    Religious,
    /// Unknown category (for forward compatibility)
    #[serde(other)]
    Unknown,
}

/// How a character discovered lore (stored on KNOWS_LORE edge)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoreKnowledge {
    pub lore_id: LoreId,
    /// Can be PC or NPC (uses CharacterId for both)
    pub character_id: CharacterId,
    /// Which chunks they know (empty = all chunks)
    pub known_chunk_ids: Vec<LoreChunkId>,
    /// How they discovered it
    pub discovery_source: LoreDiscoverySource,
    /// When discovered (game time)
    pub discovered_at: DateTime<Utc>,
    /// Optional notes about the discovery
    pub notes: Option<String>,
}

/// How lore was discovered
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum LoreDiscoverySource {
    /// Read in a book/document
    ReadBook { book_name: String },
    /// Told by an NPC in conversation
    Conversation {
        npc_id: CharacterId,
        npc_name: String,
    },
    /// Discovered through investigation/exploration
    Investigation,
    /// DM granted directly
    DmGranted { reason: Option<String> },
    /// Everyone knows this (common knowledge)
    CommonKnowledge,
    /// LLM determined character should know this
    LlmDiscovered { context: String },
}

impl Lore {
    pub fn new(
        world_id: WorldId,
        title: impl Into<String>,
        category: LoreCategory,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: LoreId::new(),
            world_id,
            title: title.into(),
            summary: String::new(),
            category,
            chunks: Vec::new(),
            is_common_knowledge: false,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    pub fn with_chunk(mut self, content: impl Into<String>) -> Self {
        let chunk = LoreChunk {
            id: LoreChunkId::new(),
            order: self.chunks.len() as u32,
            title: None,
            content: content.into(),
            discovery_hint: None,
        };
        self.chunks.push(chunk);
        self
    }

    pub fn with_chunks(mut self, chunks: Vec<LoreChunk>) -> Self {
        self.chunks = chunks;
        self
    }

    pub fn as_common_knowledge(mut self) -> Self {
        self.is_common_knowledge = true;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Get the full lore text (all chunks combined)
    pub fn full_text(&self) -> String {
        self.chunks
            .iter()
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Get text for specific chunks
    pub fn text_for_chunks(&self, chunk_ids: &[LoreChunkId]) -> String {
        self.chunks
            .iter()
            .filter(|c| chunk_ids.contains(&c.id))
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Get chunk by ID
    pub fn get_chunk(&self, chunk_id: LoreChunkId) -> Option<&LoreChunk> {
        self.chunks.iter().find(|c| c.id == chunk_id)
    }

    /// Get all chunk IDs
    pub fn chunk_ids(&self) -> Vec<LoreChunkId> {
        self.chunks.iter().map(|c| c.id).collect()
    }
}

impl LoreChunk {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            id: LoreChunkId::new(),
            order: 0,
            title: None,
            content: content.into(),
            discovery_hint: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_order(mut self, order: u32) -> Self {
        self.order = order;
        self
    }

    pub fn with_discovery_hint(mut self, hint: impl Into<String>) -> Self {
        self.discovery_hint = Some(hint.into());
        self
    }
}

impl LoreKnowledge {
    /// Create knowledge of the full lore
    pub fn full(
        lore_id: LoreId,
        character_id: CharacterId,
        source: LoreDiscoverySource,
        game_time: DateTime<Utc>,
    ) -> Self {
        Self {
            lore_id,
            character_id,
            known_chunk_ids: Vec::new(), // Empty = knows all
            discovery_source: source,
            discovered_at: game_time,
            notes: None,
        }
    }

    /// Create knowledge of specific chunks only
    pub fn partial(
        lore_id: LoreId,
        character_id: CharacterId,
        chunk_ids: Vec<LoreChunkId>,
        source: LoreDiscoverySource,
        game_time: DateTime<Utc>,
    ) -> Self {
        Self {
            lore_id,
            character_id,
            known_chunk_ids: chunk_ids,
            discovery_source: source,
            discovered_at: game_time,
            notes: None,
        }
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Returns true if character knows all chunks (or lore has no chunks)
    pub fn knows_all(&self) -> bool {
        self.known_chunk_ids.is_empty()
    }

    /// Add additional chunk IDs to known chunks
    pub fn add_chunks(&mut self, new_chunk_ids: &[LoreChunkId]) {
        for id in new_chunk_ids {
            if !self.known_chunk_ids.contains(id) {
                self.known_chunk_ids.push(*id);
            }
        }
    }
}

impl std::fmt::Display for LoreCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoreCategory::Historical => write!(f, "Historical"),
            LoreCategory::Legend => write!(f, "Legend"),
            LoreCategory::Secret => write!(f, "Secret"),
            LoreCategory::Common => write!(f, "Common"),
            LoreCategory::Technical => write!(f, "Technical"),
            LoreCategory::Political => write!(f, "Political"),
            LoreCategory::Natural => write!(f, "Natural"),
            LoreCategory::Religious => write!(f, "Religious"),
            LoreCategory::Unknown => write!(f, "Unknown"),
        }
    }
}

impl LoreCategory {
    pub fn all() -> &'static [LoreCategory] {
        &[
            LoreCategory::Historical,
            LoreCategory::Legend,
            LoreCategory::Secret,
            LoreCategory::Common,
            LoreCategory::Technical,
            LoreCategory::Political,
            LoreCategory::Natural,
            LoreCategory::Religious,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LoreCategory::Historical => "Historical",
            LoreCategory::Legend => "Legend",
            LoreCategory::Secret => "Secret",
            LoreCategory::Common => "Common",
            LoreCategory::Technical => "Technical",
            LoreCategory::Political => "Political",
            LoreCategory::Natural => "Natural",
            LoreCategory::Religious => "Religious",
            LoreCategory::Unknown => "Unknown",
        }
    }
}

impl std::str::FromStr for LoreCategory {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "historical" => Ok(LoreCategory::Historical),
            "legend" => Ok(LoreCategory::Legend),
            "secret" => Ok(LoreCategory::Secret),
            "common" => Ok(LoreCategory::Common),
            "technical" => Ok(LoreCategory::Technical),
            "political" => Ok(LoreCategory::Political),
            "natural" => Ok(LoreCategory::Natural),
            "religious" => Ok(LoreCategory::Religious),
            "unknown" => Ok(LoreCategory::Unknown),
            _ => Err(DomainError::parse(format!(
                "Invalid lore category '{}'. Valid categories: historical, legend, secret, common, technical, political, natural, religious",
                s
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_time() -> DateTime<Utc> {
        Utc.timestamp_opt(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn test_lore_creation() {
        let now = fixed_time();
        let lore = Lore::new(WorldId::new(), "Test Lore", LoreCategory::Historical, now)
            .with_summary("A test lore entry")
            .with_chunk("First chunk of content")
            .with_chunk("Second chunk of content")
            .with_tags(vec!["test".to_string(), "history".to_string()]);

        assert_eq!(lore.title, "Test Lore");
        assert_eq!(lore.summary, "A test lore entry");
        assert_eq!(lore.chunks.len(), 2);
        assert_eq!(lore.chunks[0].order, 0);
        assert_eq!(lore.chunks[1].order, 1);
        assert_eq!(lore.tags.len(), 2);
    }

    #[test]
    fn test_lore_full_text() {
        let now = fixed_time();
        let lore = Lore::new(WorldId::new(), "Test", LoreCategory::Historical, now)
            .with_chunk("First part")
            .with_chunk("Second part");

        assert_eq!(lore.full_text(), "First part\n\nSecond part");
    }

    #[test]
    fn test_lore_knowledge_full() {
        let now = fixed_time();
        let knowledge = LoreKnowledge::full(
            LoreId::new(),
            CharacterId::new(),
            LoreDiscoverySource::Investigation,
            now,
        );

        assert!(knowledge.knows_all());
    }

    #[test]
    fn test_lore_knowledge_partial() {
        let now = fixed_time();
        let chunk_id = LoreChunkId::new();
        let knowledge = LoreKnowledge::partial(
            LoreId::new(),
            CharacterId::new(),
            vec![chunk_id],
            LoreDiscoverySource::Investigation,
            now,
        );

        assert!(!knowledge.knows_all());
        assert_eq!(knowledge.known_chunk_ids.len(), 1);
    }

    #[test]
    fn test_lore_category_from_str() {
        assert_eq!(
            "historical".parse::<LoreCategory>().unwrap(),
            LoreCategory::Historical
        );
        assert_eq!(
            "SECRET".parse::<LoreCategory>().unwrap(),
            LoreCategory::Secret
        );
        assert_eq!(
            "unknown".parse::<LoreCategory>().unwrap(),
            LoreCategory::Unknown
        );
        // Invalid strings return an error
        assert!("invalid".parse::<LoreCategory>().is_err());
    }
}
