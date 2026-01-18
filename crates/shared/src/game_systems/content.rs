use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    #[default]
    Official,
    ThirdParty,
    Homebrew,
    Srd,
}

impl SourceType {
    pub fn label(&self) -> &str {
        match self {
            SourceType::Official => "Official",
            SourceType::ThirdParty => "Third Party",
            SourceType::Homebrew => "Homebrew",
            SourceType::Srd => "SRD",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSource {
    pub code: String,
    pub name: String,
    pub source_type: SourceType,
    pub page: Option<u32>,
}

impl ContentSource {
    pub fn new(code: impl Into<String>, name: impl Into<String>, source_type: SourceType) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            source_type,
            page: None,
        }
    }

    pub fn from_code(code: &str) -> Self {
        Self {
            code: code.to_string(),
            name: code.to_string(),
            source_type: SourceType::Official,
            page: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentFilter {
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
}

impl ContentFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn matches(&self, item: &ContentItem) -> bool {
        if let Some(ref source) = self.source {
            if !item.source.code.eq_ignore_ascii_case(source) {
                return false;
            }
        }

        if let Some(ref search) = self.search {
            let query = search.to_lowercase();
            if !item.name.to_lowercase().contains(&query)
                && !item.description.to_lowercase().contains(&query)
                && !item
                    .tags
                    .iter()
                    .any(|tag| tag.to_lowercase().contains(&query))
            {
                return false;
            }
        }

        if let Some(ref tags) = self.tags {
            if !tags
                .iter()
                .all(|tag| item.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)))
            {
                return false;
            }
        }

        true
    }

    pub fn apply<'a>(&self, items: impl Iterator<Item = &'a ContentItem>) -> Vec<&'a ContentItem> {
        let filtered: Vec<_> = items.filter(|item| self.matches(item)).collect();
        let offset = self.offset.unwrap_or(0);
        let limited: Vec<_> = filtered.into_iter().skip(offset).collect();

        if let Some(limit) = self.limit {
            limited.into_iter().take(limit).collect()
        } else {
            limited
        }
    }
}

#[derive(Debug, Error)]
pub enum ContentError {
    #[error("Failed to load content: {0}")]
    LoadError(String),
    #[error("Content type '{0}' not supported by this system")]
    UnsupportedContentType(String),
    #[error("Content not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilterSchema {
    pub sources: Vec<String>,
    pub tags: Vec<String>,
    pub supports_search: bool,
    pub custom_fields: Vec<FilterField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterField {
    pub key: String,
    pub label: String,
    pub field_type: FilterFieldType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterFieldType {
    Text,
    Select { options: Vec<String> },
    MultiSelect { options: Vec<String> },
    Boolean,
    Number,
    Range { min: i32, max: i32 },
}

/// Generic content types across all game systems.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentType {
    /// Race, Ancestry, Heritage, Kin, Species
    CharacterOrigin,
    /// Class, Playbook, Occupation, Role
    CharacterClass,
    /// Background, Origin Story
    CharacterBackground,
    /// Subrace, Lineage, Variant
    CharacterSuborigin,
    /// Subclass, Specialization, Path
    CharacterSubclass,
    /// Spell, Power, Psionics
    Spell,
    /// Feat, Talent, Edge
    Feat,
    /// Special abilities, Stunts, Moves
    Ability,
    /// Class features, Advances, Talents
    ClassFeature,
    /// Weapons
    Weapon,
    /// Armor and shields
    Armor,
    /// General items, gear, equipment
    Item,
    /// Magic items, artifacts
    MagicItem,
    /// Custom content type for system-specific needs
    Custom(String),
}

impl ContentType {
    pub fn display_name(&self) -> &str {
        match self {
            ContentType::CharacterOrigin => "Origin",
            ContentType::CharacterClass => "Class",
            ContentType::CharacterBackground => "Background",
            ContentType::CharacterSuborigin => "Suborigin",
            ContentType::CharacterSubclass => "Subclass",
            ContentType::Spell => "Spell",
            ContentType::Feat => "Feat",
            ContentType::Ability => "Ability",
            ContentType::ClassFeature => "Class Feature",
            ContentType::Weapon => "Weapon",
            ContentType::Armor => "Armor",
            ContentType::Item => "Item",
            ContentType::MagicItem => "Magic Item",
            ContentType::Custom(name) => name.as_str(),
        }
    }

    pub fn slug(&self) -> String {
        match self {
            ContentType::CharacterOrigin => "origin".to_string(),
            ContentType::CharacterClass => "class".to_string(),
            ContentType::CharacterBackground => "background".to_string(),
            ContentType::CharacterSuborigin => "suborigin".to_string(),
            ContentType::CharacterSubclass => "subclass".to_string(),
            ContentType::Spell => "spell".to_string(),
            ContentType::Feat => "feat".to_string(),
            ContentType::Ability => "ability".to_string(),
            ContentType::ClassFeature => "class_feature".to_string(),
            ContentType::Weapon => "weapon".to_string(),
            ContentType::Armor => "armor".to_string(),
            ContentType::Item => "item".to_string(),
            ContentType::MagicItem => "magic_item".to_string(),
            ContentType::Custom(name) => name.to_lowercase().replace(' ', "_"),
        }
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItem {
    pub id: String,
    pub content_type: ContentType,
    pub name: String,
    pub source: ContentSource,
    pub description: String,
    pub data: serde_json::Value,
    pub tags: Vec<String>,
}

impl ContentItem {
    pub fn new(
        id: impl Into<String>,
        content_type: ContentType,
        name: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        let source_code = source.into();
        Self {
            id: id.into(),
            content_type,
            name: name.into(),
            source: ContentSource::from_code(&source_code),
            description: String::new(),
            data: serde_json::Value::Null,
            tags: Vec::new(),
        }
    }

    pub fn with_source(
        id: impl Into<String>,
        content_type: ContentType,
        name: impl Into<String>,
        source: ContentSource,
    ) -> Self {
        Self {
            id: id.into(),
            content_type,
            name: name.into(),
            source,
            description: String::new(),
            data: serde_json::Value::Null,
            tags: Vec::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}
