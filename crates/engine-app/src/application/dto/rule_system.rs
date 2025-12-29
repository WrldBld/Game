//! Rule system DTOs - Re-exports from protocol
//!
//! Wire-format types are defined in `wrldbldr_protocol::dto`.
//! This module re-exports them for backwards compatibility.

pub use wrldbldr_protocol::{
    parse_system_type, parse_variant, RuleSystemPresetDetailsDto, RuleSystemPresetSummaryDto,
    RuleSystemSummaryDto, RuleSystemTypeDetailsDto,
};
