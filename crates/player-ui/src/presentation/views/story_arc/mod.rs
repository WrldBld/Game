//! Story Arc module - Timeline, Narrative Events, Event Chains

mod content;
mod event_chains;
mod tab_link;

pub use content::StoryArcContent;
pub use event_chains::EventChainsView;
pub use tab_link::StoryArcTabLink;

/// Story Arc sub-tab within Story Arc mode
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum StoryArcSubTab {
    #[default]
    Timeline,
    NarrativeEvents,
    EventChains,
}

impl StoryArcSubTab {
    pub(crate) fn from_str(s: &str) -> Self {
        match s {
            "timeline" => Self::Timeline,
            "events" => Self::NarrativeEvents,
            "chains" => Self::EventChains,
            _ => Self::Timeline,
        }
    }

    pub(crate) fn to_route_str(&self) -> &'static str {
        match self {
            Self::Timeline => "timeline",
            Self::NarrativeEvents => "events",
            Self::EventChains => "chains",
        }
    }
}
