//! Monomyth (Hero's Journey) stage enumeration

use serde::{Deserialize, Serialize};

/// The stage of the monomyth (Hero's Journey)
///
/// Based on Joseph Campbell's work in "The Hero with a Thousand Faces",
/// these stages represent the archetypal journey of transformation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum MonomythStage {
    /// The hero's normal life before the adventure
    #[default]
    OrdinaryWorld,
    /// The hero receives a challenge or quest
    CallToAdventure,
    /// The hero hesitates or refuses the call
    RefusalOfTheCall,
    /// The hero encounters a mentor figure
    MeetingTheMentor,
    /// The hero commits to the adventure
    CrossingTheThreshold,
    /// The hero faces challenges and makes allies/enemies
    TestsAlliesEnemies,
    /// The hero approaches the main challenge
    ApproachToInnermostCave,
    /// The hero faces the greatest challenge
    Ordeal,
    /// The hero gains something from the ordeal
    Reward,
    /// The hero begins the journey home
    TheRoadBack,
    /// The hero is transformed by the experience
    Resurrection,
    /// The hero returns with new wisdom
    ReturnWithElixir,
}


impl MonomythStage {
    /// Returns a description of this stage's narrative function
    pub fn description(&self) -> &'static str {
        match self {
            Self::OrdinaryWorld => "The hero's normal life before the adventure begins",
            Self::CallToAdventure => "Something disrupts the ordinary world",
            Self::RefusalOfTheCall => "The hero hesitates, showing their humanity",
            Self::MeetingTheMentor => "A guide appears to prepare the hero",
            Self::CrossingTheThreshold => "The hero commits and enters the special world",
            Self::TestsAlliesEnemies => "The hero learns the rules of the new world",
            Self::ApproachToInnermostCave => "The hero prepares for the central ordeal",
            Self::Ordeal => "The hero faces their greatest fear",
            Self::Reward => "The hero takes possession of the treasure",
            Self::TheRoadBack => "The hero deals with consequences of the ordeal",
            Self::Resurrection => "The hero is tested once more and transformed",
            Self::ReturnWithElixir => "The hero returns with something to share",
        }
    }
}
