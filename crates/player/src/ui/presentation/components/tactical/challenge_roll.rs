//! Challenge Roll Modal Component (P3.3/P3.4)
//!
//! Displays a modal for performing challenge rolls with configurable dice systems.
//! Supports three phases:
//! 1. RollInput - Formula or manual dice entry
//! 2. AwaitingApproval - Spinner while waiting for DM approval
//! 3. ResultDisplay - Animated outcome with description
//!
//! Supports both formula-based rolls (e.g., "1d20+5") and manual result entry
//! for physical dice rolls.
//!
//! # Architecture Note
//!
//! This component uses `wrldbldr_domain::value_objects::DiceFormula` for dice parsing.
//! This is an approved exception per Architecture Remediation Plan Phase 3A:
//! pure value objects from domain may be used directly in UI to avoid duplication.

use crate::presentation::state::challenge_state::ChallengeResultData;
use crate::presentation::state::{use_session_state, RollSubmissionStatus};
use crate::use_platform;
use dioxus::prelude::*;
use wrldbldr_domain::value_objects::DiceFormula;
use crate::application::application::dto::DiceInput;

/// Props for the ChallengeRollModal component
#[derive(Props, Clone, PartialEq)]
pub struct ChallengeRollModalProps {
    /// Unique challenge ID
    pub challenge_id: String,
    /// Human-readable challenge name
    pub challenge_name: String,
    /// Associated skill name
    pub skill_name: String,
    /// Difficulty display (e.g., "DC 12", "Very Hard")
    pub difficulty_display: String,
    /// Challenge description/flavor text
    pub description: String,
    /// Character's skill modifier for this challenge
    pub character_modifier: i32,
    /// Suggested dice formula based on rule system (e.g., "1d20", "1d100", "2d6")
    #[props(default)]
    pub suggested_dice: Option<String>,
    /// Human-readable hint about the rule system
    #[props(default)]
    pub rule_system_hint: Option<String>,
    /// Called with the dice input when roll is submitted
    pub on_roll: EventHandler<DiceInput>,
    /// Called when modal should close
    pub on_close: EventHandler<()>,
    /// Called when user clicks "Continue" after viewing result (P3.3/P3.4)
    #[props(default)]
    pub on_continue: Option<EventHandler<()>>,
}

/// ChallengeRollModal component (P3.3/P3.4)
///
/// Displays a modal with three phases:
/// 1. RollInput - Challenge name, skill, difficulty, and roll input
/// 2. AwaitingApproval - Spinner while DM reviews the outcome
/// 3. ResultDisplay - Animated result with outcome description
#[component]
pub fn ChallengeRollModal(props: ChallengeRollModalProps) -> Element {
    // Get roll status from session state (P3.3/P3.4)
    let session_state = use_session_state();
    let roll_status = session_state.roll_status();

    // Read current roll status
    let current_status = roll_status.read().clone();

    // Get suggested dice display
    let suggested_dice_display = props
        .suggested_dice
        .clone()
        .unwrap_or_else(|| "1d20".to_string());
    let rule_hint = props.rule_system_hint.clone();

    // Determine border color based on status
    let border_class = match &current_status {
        RollSubmissionStatus::ResultReady(result) => match result.outcome.as_str() {
            "critical_success" => {
                "border-2 border-yellow-400 shadow-[0_20px_60px_rgba(250,204,21,0.3)]"
            }
            "success" => "border-2 border-green-500 shadow-[0_20px_60px_rgba(34,197,94,0.3)]",
            "failure" => "border-2 border-red-500 shadow-[0_20px_60px_rgba(239,68,68,0.3)]",
            "critical_failure" => {
                "border-2 border-red-700 shadow-[0_20px_60px_rgba(185,28,28,0.3)]"
            }
            _ => "border-2 border-amber-500 shadow-[0_20px_60px_rgba(245,158,11,0.2)]",
        },
        RollSubmissionStatus::AwaitingApproval { .. } => {
            "border-2 border-blue-500 shadow-[0_20px_60px_rgba(59,130,246,0.2)]"
        }
        _ => "border-2 border-amber-500 shadow-[0_20px_60px_rgba(245,158,11,0.2)]",
    };

    rsx! {
        // Modal overlay
        div {
            id: "challenge-overlay",
            class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000]",
            onclick: move |_| {
                // Only allow close on click-off during roll input phase
                if matches!(current_status, RollSubmissionStatus::NotSubmitted) {
                    props.on_close.call(());
                }
            },

            // Modal content - stop propagation to prevent closing when clicking inside
            div {
                id: "challenge-modal",
                class: "bg-gradient-to-br from-dark-surface to-dark-bg p-8 rounded-2xl max-w-[500px] w-[90%] {border_class}",
                onclick: |e| e.stop_propagation(),

                // Phase-based content
                match &current_status {
                    // Phase 2: Awaiting DM Approval
                    RollSubmissionStatus::AwaitingApproval { roll, modifier, total, outcome_type } => {
                        rsx! {
                            AwaitingApprovalPhase {
                                challenge_name: props.challenge_name.clone(),
                                roll: *roll,
                                modifier: *modifier,
                                total: *total,
                                outcome_type: outcome_type.clone(),
                            }
                        }
                    }

                    // Phase 3: Result Display
                    RollSubmissionStatus::ResultReady(result) => {
                        rsx! {
                            ResultDisplayPhase {
                                result: result.clone(),
                                on_continue: move |_| {
                                    if let Some(handler) = &props.on_continue {
                                        handler.call(());
                                    } else {
                                        props.on_close.call(());
                                    }
                                },
                            }
                        }
                    }

                    // Phase 1: Roll Input (NotSubmitted or Dismissed)
                    RollSubmissionStatus::NotSubmitted | RollSubmissionStatus::Dismissed => {
                        rsx! {
                            RollInputPhase {
                                challenge_name: props.challenge_name.clone(),
                                description: props.description.clone(),
                                skill_name: props.skill_name.clone(),
                                difficulty_display: props.difficulty_display.clone(),
                                character_modifier: props.character_modifier,
                                suggested_dice_display: suggested_dice_display.clone(),
                                rule_hint: rule_hint.clone(),
                                on_close: move |_| props.on_close.call(()),
                                on_roll: move |input: DiceInput| props.on_roll.call(input),
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Phase 1: Roll Input (P3.3/P3.4)
#[component]
fn RollInputPhase(
    challenge_name: String,
    description: String,
    skill_name: String,
    difficulty_display: String,
    character_modifier: i32,
    suggested_dice_display: String,
    rule_hint: Option<String>,
    on_close: EventHandler<()>,
    on_roll: EventHandler<DiceInput>,
) -> Element {
    // Input mode: true = use formula roll, false = manual input
    let mut use_formula_mode = use_signal(|| true);
    let mut formula_input = use_signal(move || suggested_dice_display.clone());
    let mut manual_input = use_signal(String::new);
    let mut roll_result = use_signal(|| None::<RollDisplayState>);
    let mut is_rolling = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);

    let platform = use_platform();

    rsx! {
        // Header
        div {
            class: "flex justify-between items-center mb-6",

            h2 {
                class: "text-amber-500 m-0 text-2xl",
                "Skill Challenge"
            }

            button {
                onclick: move |_| on_close.call(()),
                class: "bg-transparent border-none text-gray-400 cursor-pointer text-2xl p-0",
                "×"
            }
        }

        // Challenge details
        div {
            class: "mb-6",

            h3 {
                class: "text-white m-0 mb-2 text-xl",
                "{challenge_name}"
            }

            p {
                class: "text-gray-400 m-0 mb-4 leading-relaxed",
                "{description}"
            }
        }

        // Skill and difficulty info
        div {
            class: "flex justify-between mb-4 p-4 bg-black/30 rounded-lg",

            div {
                span {
                    class: "text-gray-400 text-xs uppercase block mb-1",
                    "Skill"
                }
                span {
                    class: "text-white font-bold",
                    "{skill_name}"
                }
            }

            div {
                span {
                    class: "text-gray-400 text-xs uppercase block mb-1",
                    "Difficulty"
                }
                span {
                    class: "text-amber-500 font-bold",
                    "{difficulty_display}"
                }
            }

            div {
                span {
                    class: "text-gray-400 text-xs uppercase block mb-1",
                    "Modifier"
                }
                span {
                    class: "text-blue-500 font-bold",
                    if character_modifier >= 0 {
                        "+{character_modifier}"
                    } else {
                        "{character_modifier}"
                    }
                }
            }
        }

        // Rule system hint
        if let Some(hint) = &rule_hint {
            p {
                class: "text-gray-400 text-xs text-center m-0 mb-4 italic",
                "{hint}"
            }
        }

        // Mode toggle
        div {
            class: "flex gap-2 mb-4",

            button {
                onclick: move |_| use_formula_mode.set(true),
                class: if *use_formula_mode.read() {
                    "flex-1 p-3 bg-amber-500 text-white border-none rounded-l-lg cursor-pointer font-semibold"
                } else {
                    "flex-1 p-3 bg-white/10 text-gray-400 border border-white/20 rounded-l-lg cursor-pointer"
                },
                "Digital Roll"
            }

            button {
                onclick: move |_| use_formula_mode.set(false),
                class: if !*use_formula_mode.read() {
                    "flex-1 p-3 bg-amber-500 text-white border-none rounded-r-lg cursor-pointer font-semibold"
                } else {
                    "flex-1 p-3 bg-white/10 text-gray-400 border border-white/20 rounded-r-lg cursor-pointer"
                },
                "Physical Dice"
            }
        }

        // Roll/Input section
        if let Some(result) = roll_result.read().clone() {
            // Show roll result (pre-submit)
            RollResultDisplay {
                result: result.clone(),
                on_submit: move |_| {
                    let result = result.clone();
                    if result.is_manual {
                        on_roll.call(DiceInput::Manual(result.total - character_modifier));
                    } else {
                        on_roll.call(DiceInput::Formula(result.formula.clone()));
                    }
                },
                on_reroll: move |_| {
                    roll_result.set(None);
                    error_message.set(None);
                },
            }
        } else if *use_formula_mode.read() {
            // Formula input mode
            div {
                // Formula input
                div {
                    class: "mb-4",

                    label {
                        class: "text-gray-400 text-xs block mb-2",
                        "Dice Formula (e.g., 1d20)"
                    }

                    input {
                        r#type: "text",
                        value: "{formula_input}",
                        oninput: move |e| formula_input.set(e.value().to_string()),
                        placeholder: "1d20",
                        class: "w-full p-4 bg-black/30 border border-white/20 rounded-lg text-white text-xl text-center font-mono box-border",
                    }
                }

                // Error message
                if let Some(err) = error_message.read().as_ref() {
                    p {
                        class: "text-red-500 text-sm text-center m-0 mb-4",
                        "{err}"
                    }
                }

                // Roll button
                button {
                    onclick: move |_| {
                        let formula_str = formula_input.read().clone();
                        match DiceFormula::parse(&formula_str) {
                            Ok(dice_formula) => {
                                is_rolling.set(true);
                                error_message.set(None);

                                // Roll using domain's DiceFormula with platform RNG
                                let result = dice_formula.roll(|min, max| platform.random_range(min, max));
                                let total = result.total + character_modifier;

                                roll_result.set(Some(RollDisplayState {
                                    formula: dice_formula.display(),
                                    individual_rolls: result.individual_rolls,
                                    dice_total: result.dice_total,
                                    formula_modifier: result.modifier_applied,
                                    character_modifier,
                                    total,
                                    is_manual: false,
                                }));

                                is_rolling.set(false);
                            }
                            Err(e) => {
                                error_message.set(Some(e.to_string()));
                            }
                        }
                    },
                    disabled: *is_rolling.read(),
                    class: "w-full p-6 bg-gradient-to-br from-amber-500 to-amber-600 text-white border-none rounded-lg cursor-pointer text-xl font-bold transition-all",

                    if *is_rolling.read() {
                        "Rolling..."
                    } else {
                        "Roll"
                    }
                }
            }
        } else {
            // Manual input mode
            div {
                p {
                    class: "text-gray-400 text-sm text-center m-0 mb-4",
                    "Enter the result from your physical dice roll"
                }

                div {
                    class: "mb-4",

                    label {
                        class: "text-gray-400 text-xs block mb-2",
                        "Dice Result (before modifiers)"
                    }

                    input {
                        r#type: "number",
                        value: "{manual_input}",
                        oninput: move |e| manual_input.set(e.value().to_string()),
                        placeholder: "Enter roll result",
                        class: "w-full p-4 bg-black/30 border border-white/20 rounded-lg text-white text-2xl text-center box-border",
                    }
                }

                // Error message
                if let Some(err) = error_message.read().as_ref() {
                    p {
                        class: "text-red-500 text-sm text-center m-0 mb-4",
                        "{err}"
                    }
                }

                // Submit button
                button {
                    onclick: move |_| {
                        let input = manual_input.read().clone();
                        match input.trim().parse::<i32>() {
                            Ok(value) if value >= 1 => {
                                error_message.set(None);
                                let total = value + character_modifier;
                                roll_result.set(Some(RollDisplayState {
                                    formula: "Manual".to_string(),
                                    individual_rolls: vec![value],
                                    dice_total: value,
                                    formula_modifier: 0,
                                    character_modifier,
                                    total,
                                    is_manual: true,
                                }));
                            }
                            Ok(_) => {
                                error_message.set(Some("Value must be at least 1".to_string()));
                            }
                            Err(_) => {
                                error_message.set(Some("Please enter a valid number".to_string()));
                            }
                        }
                    },
                    disabled: manual_input.read().is_empty(),
                    class: "w-full p-6 bg-gradient-to-br from-amber-500 to-amber-600 text-white border-none rounded-lg cursor-pointer text-xl font-bold transition-all",
                    "Submit Result"
                }
            }
        }
    }
}

/// Phase 2: Awaiting DM Approval (P3.3/P3.4)
#[component]
fn AwaitingApprovalPhase(
    challenge_name: String,
    roll: i32,
    modifier: i32,
    total: i32,
    outcome_type: String,
) -> Element {
    rsx! {
        // Centered content
        div {
            class: "text-center py-8",

            // Animated spinner
            div {
                class: "mb-6",
                div {
                    class: "w-16 h-16 mx-auto border-4 border-blue-500 border-t-transparent rounded-full animate-spin",
                }
            }

            h2 {
                class: "text-white text-2xl mb-4",
                "Roll Submitted"
            }

            // Roll breakdown card
            div {
                class: "bg-black/30 rounded-lg p-6 mb-6",

                h3 {
                    class: "text-gray-400 text-sm uppercase mb-4",
                    "{challenge_name}"
                }

                div {
                    class: "text-5xl font-bold text-blue-500 mb-4",
                    "{total}"
                }

                p {
                    class: "text-gray-400",
                    "Roll: {roll}  +  Modifier: {modifier}"
                }
            }

            p {
                class: "text-gray-400 italic",
                "Waiting for DM to confirm the outcome..."
            }
        }
    }
}

/// Phase 3: Result Display (P3.3/P3.4)
#[component]
fn ResultDisplayPhase(result: ChallengeResultData, on_continue: EventHandler<()>) -> Element {
    // Determine display colors and text based on outcome
    let (outcome_text, outcome_class, glow_class) = match result.outcome.as_str() {
        "critical_success" => (
            "CRITICAL SUCCESS",
            "text-yellow-400",
            "shadow-[0_0_30px_rgba(250,204,21,0.5)]",
        ),
        "success" => (
            "SUCCESS",
            "text-green-500",
            "shadow-[0_0_20px_rgba(34,197,94,0.5)]",
        ),
        "failure" => (
            "FAILURE",
            "text-red-500",
            "shadow-[0_0_20px_rgba(239,68,68,0.5)]",
        ),
        "critical_failure" => (
            "CRITICAL FAILURE",
            "text-red-700",
            "shadow-[0_0_30px_rgba(185,28,28,0.5)]",
        ),
        _ => (
            "RESULT",
            "text-amber-500",
            "shadow-[0_0_20px_rgba(245,158,11,0.5)]",
        ),
    };

    rsx! {
        div {
            class: "text-center py-4",

            // Outcome header with glow effect
            div {
                class: "mb-6",

                h2 {
                    class: "text-4xl font-bold {outcome_class} {glow_class} mb-2",
                    "*** {outcome_text} ***"
                }
            }

            // Roll breakdown
            div {
                class: "bg-black/30 rounded-lg p-4 mb-6",

                div {
                    class: "flex justify-between mb-2",
                    span { class: "text-gray-400", "Roll" }
                    span { class: "text-white font-bold", "{result.roll}" }
                }

                div {
                    class: "flex justify-between mb-2",
                    span { class: "text-gray-400", "Modifier" }
                    span {
                        class: "text-blue-500 font-bold",
                        if result.modifier >= 0 { "+{result.modifier}" } else { "{result.modifier}" }
                    }
                }

                div {
                    class: "border-t border-white/10 pt-2 flex justify-between",
                    span { class: "text-gray-400 font-bold", "Total" }
                    span { class: "{outcome_class} font-bold text-xl", "{result.total}" }
                }
            }

            // Optional roll breakdown string
            if let Some(breakdown) = &result.roll_breakdown {
                p {
                    class: "text-gray-500 text-sm mb-4 font-mono",
                    "{breakdown}"
                }
            }

            // Outcome description
            div {
                class: "bg-black/20 rounded-lg p-4 mb-6 text-left",

                p {
                    class: "text-gray-300 leading-relaxed italic",
                    "{result.outcome_description}"
                }
            }

            // Continue button
            button {
                onclick: move |_| on_continue.call(()),
                class: "w-full p-4 bg-gradient-to-br from-amber-500 to-amber-600 text-white border-none rounded-lg cursor-pointer text-lg font-semibold transition-all hover:from-amber-400 hover:to-amber-500",
                "Continue"
            }
        }
    }
}

/// State for displaying roll results
#[derive(Clone, PartialEq)]
struct RollDisplayState {
    formula: String,
    individual_rolls: Vec<i32>,
    dice_total: i32,
    formula_modifier: i32,
    character_modifier: i32,
    total: i32,
    is_manual: bool,
}

/// Component for displaying roll results
#[component]
fn RollResultDisplay(
    result: RollDisplayState,
    on_submit: EventHandler<()>,
    on_reroll: EventHandler<()>,
) -> Element {
    // Format individual rolls for display
    let rolls_display = result
        .individual_rolls
        .iter()
        .map(|r| r.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    // Determine if this is a critical (natural 20 or 1 on d20)
    let is_nat_20 = result.individual_rolls.len() == 1 && result.individual_rolls[0] == 20;
    let is_nat_1 = result.individual_rolls.len() == 1 && result.individual_rolls[0] == 1;

    let result_text_class = if is_nat_20 {
        "text-5xl font-bold text-green-500 mb-2 shadow-[0_0_20px_rgba(34,197,94,0.7)]"
    } else if is_nat_1 {
        "text-5xl font-bold text-red-500 mb-2 shadow-[0_0_20px_rgba(239,68,68,0.7)]"
    } else {
        "text-5xl font-bold text-amber-500 mb-2 shadow-[0_0_10px_rgba(245,158,11,0.5)]"
    };

    rsx! {
        div {
            class: "mb-4",

            // Roll result display
            div {
                class: "text-center mb-4",

                div {
                    class: "{result_text_class}",

                    if is_nat_20 {
                        "Natural 20!"
                    } else if is_nat_1 {
                        "Natural 1!"
                    } else if result.is_manual {
                        "Manual: {result.dice_total}"
                    } else {
                        "{result.formula}({rolls_display})"
                    }
                }
            }

            // Calculation breakdown
            div {
                class: "bg-black/30 p-4 rounded-lg mb-4",

                div {
                    class: "flex justify-between mb-2",

                    span { class: "text-gray-400", "Dice:" }
                    span { class: "text-white font-bold", "{result.dice_total}" }
                }

                if result.formula_modifier != 0 {
                    div {
                        class: "flex justify-between mb-2",

                        span { class: "text-gray-400", "Formula Mod:" }
                        span {
                            class: "text-purple-500 font-bold",
                            if result.formula_modifier >= 0 {
                                "+{result.formula_modifier}"
                            } else {
                                "{result.formula_modifier}"
                            }
                        }
                    }
                }

                div {
                    class: "flex justify-between mb-2",

                    span { class: "text-gray-400", "Skill Mod:" }
                    span {
                        class: "text-blue-500 font-bold",
                        if result.character_modifier >= 0 {
                            "+{result.character_modifier}"
                        } else {
                            "{result.character_modifier}"
                        }
                    }
                }

                div {
                    class: "border-t border-white/10 pt-2 flex justify-between",

                    span { class: "text-gray-400 font-bold", "Total:" }
                    span {
                        class: "text-green-500 font-bold text-xl",
                        "{result.total}"
                    }
                }
            }

            // Action buttons
            div {
                class: "flex gap-2",

                button {
                    onclick: move |_| on_reroll.call(()),
                    class: "flex-1 p-4 bg-white/10 text-gray-400 border border-white/20 rounded-lg cursor-pointer text-base",
                    "Re-roll"
                }

                button {
                    onclick: move |_| on_submit.call(()),
                    class: "flex-[2] p-4 bg-gradient-to-br from-green-500 to-green-600 text-white border-none rounded-lg cursor-pointer text-base font-semibold",
                    "Submit Roll"
                }
            }
        }
    }
}
