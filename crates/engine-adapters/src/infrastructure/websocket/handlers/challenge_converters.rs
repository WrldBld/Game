//! Challenge type conversion helpers.
//!
//! Converts between protocol types and use case types for the challenge handlers.

use wrldbldr_engine_ports::inbound::{AdHocOutcomes, DiceInputType};
use wrldbldr_engine_ports::outbound::OutcomeDecision;

/// Convert protocol ChallengeOutcomeDecisionData to use case OutcomeDecision
pub fn to_use_case_decision(
    decision: wrldbldr_protocol::ChallengeOutcomeDecisionData,
) -> OutcomeDecision {
    match decision {
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Accept => OutcomeDecision::Accept,
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Edit {
            modified_description,
        } => OutcomeDecision::Edit {
            modified_text: modified_description,
        },
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Suggest { guidance } => {
            OutcomeDecision::Suggest { guidance }
        }
        wrldbldr_protocol::ChallengeOutcomeDecisionData::Unknown => {
            OutcomeDecision::Accept // Default unknown to Accept
        }
    }
}

/// Convert protocol DiceInputType to use case DiceInputType
pub fn to_use_case_dice_input(input: wrldbldr_protocol::DiceInputType) -> DiceInputType {
    match input {
        wrldbldr_protocol::DiceInputType::Formula(formula) => DiceInputType::Formula(formula),
        wrldbldr_protocol::DiceInputType::Manual(value) => DiceInputType::Manual(value),
        wrldbldr_protocol::DiceInputType::Unknown => DiceInputType::Manual(0), // Default unknown to Manual(0)
    }
}

/// Convert protocol AdHocOutcomes to use case AdHocOutcomes
pub fn to_use_case_adhoc_outcomes(outcomes: wrldbldr_protocol::AdHocOutcomes) -> AdHocOutcomes {
    // Explicit conversion instead of using From impl in protocol
    // This allows removing protocol→domain dependency
    AdHocOutcomes::new(
        outcomes.success,
        outcomes.failure,
        outcomes.critical_success,
        outcomes.critical_failure,
    )
}
