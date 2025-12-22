//! Approval popup component
//!
//! Shows proposed NPC dialogue and actions for DM approval before execution.
//! Supports challenge outcome regeneration and discard functionality.

use dioxus::prelude::*;
use wrldbldr_protocol::{ChallengeSuggestionInfo, NarrativeEventSuggestionInfo, OutcomeDetailData};

/// A proposed action/tool call from the LLM
#[derive(Clone, PartialEq)]
pub struct ProposedAction {
    /// Action name
    pub name: String,
    /// Action description
    pub description: String,
    /// Whether this action is checked/approved
    pub checked: bool,
}

/// Outcome details for display in the approval popup
#[derive(Clone, PartialEq, Default)]
pub struct ChallengeOutcomes {
    pub success: Option<OutcomeDetailData>,
    pub failure: Option<OutcomeDetailData>,
    pub critical_success: Option<OutcomeDetailData>,
    pub critical_failure: Option<OutcomeDetailData>,
}

/// Data for outcome regeneration request
#[derive(Clone, PartialEq)]
pub struct RegenerateOutcomeData {
    pub request_id: String,
    /// Which outcome to regenerate (None = all)
    pub outcome_type: Option<String>,
    pub guidance: Option<String>,
}

/// Data for discard challenge request
#[derive(Clone, PartialEq)]
pub struct DiscardChallengeData {
    pub request_id: String,
    pub feedback: Option<String>,
}

/// Props for the ApprovalPopup component
#[derive(Props, Clone, PartialEq)]
pub struct ApprovalPopupProps {
    /// Unique request ID for this approval item
    #[props(default)]
    pub request_id: String,
    /// The NPC that will perform the action
    pub npc_name: String,
    /// Proposed dialogue text from LLM
    pub dialogue: String,
    /// List of proposed actions/tool calls
    pub proposed_actions: Vec<ProposedAction>,
    /// Optional challenge suggestion from Engine
    pub challenge_suggestion: Option<ChallengeSuggestionInfo>,
    /// Optional challenge outcomes for detailed display
    #[props(default)]
    pub challenge_outcomes: Option<ChallengeOutcomes>,
    /// Optional narrative event suggestion from Engine
    pub narrative_event_suggestion: Option<NarrativeEventSuggestionInfo>,
    /// Handler when Accept is clicked
    pub on_accept: EventHandler<Vec<ProposedAction>>,
    /// Handler when Modify is clicked
    pub on_modify: EventHandler<()>,
    /// Handler when Reject is clicked
    pub on_reject: EventHandler<()>,
    /// Handler when regenerate outcome is requested (optional)
    #[props(default)]
    pub on_regenerate_outcome: Option<EventHandler<RegenerateOutcomeData>>,
    /// Handler when discard challenge is requested (optional)
    #[props(default)]
    pub on_discard_challenge: Option<EventHandler<DiscardChallengeData>>,
}

/// ApprovalPopup component - Shows LLM response for approval
///
/// Displays proposed NPC dialogue and tool calls with checkboxes.
/// DM can approve, modify, or reject the proposed actions.
#[component]
pub fn ApprovalPopup(props: ApprovalPopupProps) -> Element {
    let mut actions = use_signal(|| props.proposed_actions.clone());

    rsx! {
        div {
            class: "approval-popup bg-gray-700 border-2 border-amber-500 rounded-xl p-5 shadow-2xl",

            // Header
            h3 {
                class: "text-amber-500 m-0 mb-4 text-base",
                "Approval Required"
            }

            // NPC and action info
            div {
                class: "mb-4 p-3 bg-black bg-opacity-20 rounded-lg",

                p {
                    class: "text-gray-400 text-xs uppercase m-0 mb-1",
                    "Action from: {props.npc_name}"
                }
            }

            // Dialogue box
            div {
                class: "mb-4 p-4 bg-black bg-opacity-30 border-l-4 border-blue-500 rounded-lg",

                p {
                    class: "text-gray-400 text-xs uppercase m-0 mb-2",
                    "Proposed Dialogue"
                }

                p {
                    class: "text-white italic m-0 leading-normal",
                    "\"{props.dialogue}\""
                }
            }

            // Challenge suggestion section
            if let Some(suggestion) = &props.challenge_suggestion {
                {
                    let request_id = props.request_id.clone();
                    let on_regenerate = props.on_regenerate_outcome.clone();
                    let on_discard = props.on_discard_challenge.clone();
                    let outcomes = props.challenge_outcomes.clone();

                    rsx! {
                        div {
                            class: "mb-4 p-4 bg-amber-500 bg-opacity-10 border border-amber-500 rounded-lg",

                            // Header with title and discard button
                            div {
                                class: "flex justify-between items-center mb-3",

                                h4 {
                                    class: "text-amber-500 m-0 text-sm flex gap-2 items-center",
                                    "Challenge Suggested"
                                }

                                // Discard button
                                if on_discard.is_some() {
                                    {
                                        let request_id_discard = request_id.clone();
                                        let handler = on_discard.clone();
                                        rsx! {
                                            button {
                                                onclick: move |_| {
                                                    if let Some(h) = &handler {
                                                        h.call(DiscardChallengeData {
                                                            request_id: request_id_discard.clone(),
                                                            feedback: None,
                                                        });
                                                    }
                                                },
                                                class: "py-1 px-2 bg-gray-500 bg-opacity-50 text-gray-400 border-0 rounded cursor-pointer text-xs uppercase",
                                                "Discard"
                                            }
                                        }
                                    }
                                }
                            }

                            // Challenge info
                            div {
                                class: "mb-3",

                                div {
                                    class: "flex justify-between items-baseline",

                                    span {
                                        class: "text-white font-bold text-sm",
                                        "{suggestion.challenge_name}"
                                    }

                                    span {
                                        class: "text-gray-400 ml-2 text-xs",
                                        "({suggestion.skill_name} - {suggestion.difficulty_display})"
                                    }
                                }
                            }

                            div {
                                class: "mb-2",

                                p {
                                    class: "text-gray-400 text-xs m-0 mb-1",
                                    "Confidence: {suggestion.confidence}"
                                }
                            }

                            p {
                                class: "text-gray-400 text-xs italic m-0 mb-3 leading-snug",
                                "\"{suggestion.reasoning}\""
                            }

                            // Outcome details (if available)
                            if let Some(ref outcomes) = outcomes {
                                OutcomeDetailsSection {
                                    outcomes: outcomes.clone(),
                                    request_id: request_id.clone(),
                                    on_regenerate: on_regenerate.clone(),
                                }
                            }

                            // Action buttons
                            div {
                                class: "flex gap-2",

                                button {
                                    class: "flex-1 p-2 bg-green-500 bg-opacity-80 text-white border-0 rounded-md cursor-pointer text-xs font-semibold",
                                    "Approve Challenge"
                                }

                                button {
                                    class: "flex-1 p-2 bg-red-500 bg-opacity-80 text-white border-0 rounded-md cursor-pointer text-xs font-semibold",
                                    "Skip Challenge"
                                }
                            }
                        }
                    }
                }
            }

            // Narrative event suggestion section
            if let Some(suggestion) = &props.narrative_event_suggestion {
                div {
                    class: "mb-4 p-4 bg-purple-500 bg-opacity-10 border border-purple-500 rounded-lg",

                    h4 {
                        class: "text-purple-500 m-0 mb-3 text-sm flex gap-2 items-center",
                        "Narrative Event Suggested"
                    }

                    div {
                        class: "mb-3",

                        div {
                            class: "flex justify-between items-baseline",

                            span {
                                class: "text-white font-bold text-sm",
                                "{suggestion.event_name}"
                            }
                        }

                        if let Some(outcome) = &suggestion.suggested_outcome {
                            p {
                                class: "text-gray-400 text-xs m-0 mt-2",
                                "Suggested Outcome: {outcome}"
                            }
                        }
                    }

                    div {
                        class: "mb-2",

                        p {
                            class: "text-gray-400 text-xs m-0 mb-1",
                            "Confidence: {suggestion.confidence}"
                        }
                    }

                    p {
                        class: "text-gray-400 text-xs italic m-0 mb-3 leading-snug",
                        "\"{suggestion.reasoning}\""
                    }

                    div {
                        class: "flex gap-2",

                        button {
                            class: "flex-1 p-2 bg-purple-500 bg-opacity-80 text-white border-0 rounded-md cursor-pointer text-xs font-semibold",
                            "Trigger Event"
                        }

                        button {
                            class: "flex-1 p-2 bg-gray-500 bg-opacity-80 text-white border-0 rounded-md cursor-pointer text-xs font-semibold",
                            "Skip Event"
                        }
                    }
                }
            }

            // Proposed actions list
            if !props.proposed_actions.is_empty() {
                div {
                    class: "mb-4",

                    p {
                        class: "text-gray-400 text-xs uppercase m-0 mb-2",
                        "Proposed Actions"
                    }

                    div {
                        class: "flex flex-col gap-2",

                        for (idx, action) in actions.read().iter().enumerate() {
                            ProposedActionCheckbox {
                                action: action.clone(),
                                on_toggle: move |_| {
                                    let mut updated_actions = actions.read().to_vec();
                                    if idx < updated_actions.len() {
                                        updated_actions[idx].checked = !updated_actions[idx].checked;
                                    }
                                    actions.set(updated_actions);
                                }
                            }
                        }
                    }
                }
            }

            // Action buttons
            div {
                class: "flex gap-3",

                button {
                    onclick: move |_| props.on_accept.call(actions.read().to_vec()),
                    class: "flex-1 p-3 bg-green-500 text-white border-0 rounded-lg cursor-pointer font-semibold text-sm transition-colors duration-200",
                    onmouseover: move |_| {},
                    "Accept"
                }

                button {
                    onclick: move |_| props.on_modify.call(()),
                    class: "flex-1 p-3 bg-blue-500 text-white border-0 rounded-lg cursor-pointer font-semibold text-sm transition-colors duration-200",
                    "Modify"
                }

                button {
                    onclick: move |_| props.on_reject.call(()),
                    class: "flex-1 p-3 bg-red-500 text-white border-0 rounded-lg cursor-pointer font-semibold text-sm transition-colors duration-200",
                    "Reject"
                }
            }
        }
    }
}

/// Individual action checkbox
#[component]
fn ProposedActionCheckbox(
    action: ProposedAction,
    on_toggle: EventHandler<()>,
) -> Element {
    rsx! {
        label {
            class: "flex items-start gap-3 p-3 bg-black bg-opacity-20 rounded-md cursor-pointer transition-colors duration-200",

            input {
                r#type: "checkbox",
                checked: action.checked,
                onchange: move |_| on_toggle.call(()),
                class: "mt-1 cursor-pointer",
            }

            div {
                class: "flex-1",

                div {
                    class: "text-white text-sm font-medium",
                    "{action.name}"
                }

                div {
                    class: "text-gray-400 text-xs mt-1",
                    "{action.description}"
                }
            }
        }
    }
}

/// Props for OutcomeDetailsSection
#[derive(Props, Clone, PartialEq)]
struct OutcomeDetailsSectionProps {
    outcomes: ChallengeOutcomes,
    request_id: String,
    on_regenerate: Option<EventHandler<RegenerateOutcomeData>>,
}

/// Section displaying challenge outcomes with regeneration buttons
#[component]
fn OutcomeDetailsSection(props: OutcomeDetailsSectionProps) -> Element {
    let mut expanded_outcome = use_signal(|| Option::<String>::None);

    rsx! {
        div {
            class: "mb-3 border-t border-amber-500 border-opacity-30 pt-3",

            // Header with "Regenerate All" button
            div {
                class: "flex justify-between items-center mb-2",

                p {
                    class: "text-gray-400 text-xs uppercase m-0",
                    "Outcomes"
                }

                if props.on_regenerate.is_some() {
                    {
                        let request_id = props.request_id.clone();
                        let handler = props.on_regenerate.clone();
                        rsx! {
                            button {
                                onclick: move |_| {
                                    if let Some(h) = &handler {
                                        h.call(RegenerateOutcomeData {
                                            request_id: request_id.clone(),
                                            outcome_type: None, // None = regenerate all
                                            guidance: None,
                                        });
                                    }
                                },
                                class: "py-1 px-2 bg-blue-500 bg-opacity-50 text-white border-0 rounded cursor-pointer text-xs uppercase",
                                "Regenerate All"
                            }
                        }
                    }
                }
            }

            // Outcome tabs
            div {
                class: "flex flex-col gap-1",

                // Success outcome
                if let Some(ref outcome) = props.outcomes.success {
                    OutcomeTab {
                        label: "Success",
                        color: "#22c55e",
                        outcome: outcome.clone(),
                        outcome_type: "success".to_string(),
                        request_id: props.request_id.clone(),
                        on_regenerate: props.on_regenerate.clone(),
                        is_expanded: *expanded_outcome.read() == Some("success".to_string()),
                        on_toggle: move |_| {
                            let current = expanded_outcome.read().clone();
                            if current == Some("success".to_string()) {
                                expanded_outcome.set(None);
                            } else {
                                expanded_outcome.set(Some("success".to_string()));
                            }
                        },
                    }
                }

                // Failure outcome
                if let Some(ref outcome) = props.outcomes.failure {
                    OutcomeTab {
                        label: "Failure",
                        color: "#ef4444",
                        outcome: outcome.clone(),
                        outcome_type: "failure".to_string(),
                        request_id: props.request_id.clone(),
                        on_regenerate: props.on_regenerate.clone(),
                        is_expanded: *expanded_outcome.read() == Some("failure".to_string()),
                        on_toggle: move |_| {
                            let current = expanded_outcome.read().clone();
                            if current == Some("failure".to_string()) {
                                expanded_outcome.set(None);
                            } else {
                                expanded_outcome.set(Some("failure".to_string()));
                            }
                        },
                    }
                }

                // Critical success outcome
                if let Some(ref outcome) = props.outcomes.critical_success {
                    OutcomeTab {
                        label: "Critical Success",
                        color: "#fbbf24",
                        outcome: outcome.clone(),
                        outcome_type: "critical_success".to_string(),
                        request_id: props.request_id.clone(),
                        on_regenerate: props.on_regenerate.clone(),
                        is_expanded: *expanded_outcome.read() == Some("critical_success".to_string()),
                        on_toggle: move |_| {
                            let current = expanded_outcome.read().clone();
                            if current == Some("critical_success".to_string()) {
                                expanded_outcome.set(None);
                            } else {
                                expanded_outcome.set(Some("critical_success".to_string()));
                            }
                        },
                    }
                }

                // Critical failure outcome
                if let Some(ref outcome) = props.outcomes.critical_failure {
                    OutcomeTab {
                        label: "Critical Failure",
                        color: "#dc2626",
                        outcome: outcome.clone(),
                        outcome_type: "critical_failure".to_string(),
                        request_id: props.request_id.clone(),
                        on_regenerate: props.on_regenerate.clone(),
                        is_expanded: *expanded_outcome.read() == Some("critical_failure".to_string()),
                        on_toggle: move |_| {
                            let current = expanded_outcome.read().clone();
                            if current == Some("critical_failure".to_string()) {
                                expanded_outcome.set(None);
                            } else {
                                expanded_outcome.set(Some("critical_failure".to_string()));
                            }
                        },
                    }
                }
            }
        }
    }
}

/// Props for OutcomeTab
#[derive(Props, Clone, PartialEq)]
struct OutcomeTabProps {
    label: &'static str,
    color: &'static str,
    outcome: OutcomeDetailData,
    outcome_type: String,
    request_id: String,
    on_regenerate: Option<EventHandler<RegenerateOutcomeData>>,
    is_expanded: bool,
    on_toggle: EventHandler<()>,
}

/// Individual outcome tab with expandable details and inline editing
#[component]
fn OutcomeTab(props: OutcomeTabProps) -> Element {
    // Extract style variables before rsx! block
    let border_style = format!("border-left-color: {};", props.color);
    let text_color_style = format!("color: {};", props.color);
    let label = props.label;
    let is_expanded = props.is_expanded;
    let editing = use_signal(|| false);
    let mut edited_flavor = use_signal(|| props.outcome.flavor_text.clone());
    let mut edited_direction = use_signal(|| props.outcome.scene_direction.clone());

    rsx! {
        div {
            class: "bg-black bg-opacity-20 rounded-md border-l-4",
            style: "{border_style}",

            // Header (clickable to expand)
            button {
                onclick: move |_| props.on_toggle.call(()),
                class: "w-full flex justify-between items-center p-2 px-3 bg-transparent border-0 cursor-pointer text-left",

                span {
                    class: "text-xs font-semibold",
                    style: "{text_color_style}",
                    "{label}"
                }

                span {
                    class: "text-gray-500 text-xs",
                    if is_expanded { "v" } else { ">" }
                }
            }

            // Expanded content
            if is_expanded {
                div {
                    class: "px-3 pb-3",

                    // Flavor text
                    if !props.outcome.flavor_text.is_empty() || *editing.read() {
                        div {
                            class: "mb-2",

                            p {
                                class: "text-gray-500 text-xs uppercase m-0 mb-1",
                                "Flavor"
                            }
                            if *editing.read() {
                                textarea {
                                    value: "{edited_flavor}",
                                    oninput: move |e| edited_flavor.set(e.value()),
                                    class: "w-full p-2 bg-dark-bg border border-gray-700 rounded-md text-white text-xs min-h-[60px] resize-y",
                                }
                            } else {
                                p {
                                    class: "text-white text-xs italic m-0 leading-snug",
                                    "\"{props.outcome.flavor_text}\""
                                }
                            }
                        }
                    }

                    // Scene direction
                    if !props.outcome.scene_direction.is_empty() || *editing.read() {
                        div {
                            class: "mb-2",

                            p {
                                class: "text-gray-500 text-xs uppercase m-0 mb-1",
                                "Scene Direction"
                            }
                            if *editing.read() {
                                textarea {
                                    value: "{edited_direction}",
                                    oninput: move |e| edited_direction.set(e.value()),
                                    class: "w-full p-2 bg-dark-bg border border-gray-700 rounded-md text-gray-300 text-xs min-h-[60px] resize-y",
                                }
                            } else {
                                p {
                                    class: "text-gray-300 text-xs m-0 leading-snug",
                                    "{props.outcome.scene_direction}"
                                }
                            }
                        }
                    }

                    // Tool calls (if any)
                    if !props.outcome.proposed_tools.is_empty() {
                        div {
                            class: "mb-2",

                            p {
                                class: "text-gray-500 text-xs uppercase m-0 mb-1",
                                "Tool Calls ({props.outcome.proposed_tools.len()})"
                            }
                            div {
                                class: "flex flex-wrap gap-1",

                                for tool in props.outcome.proposed_tools.iter() {
                                    span {
                                        key: "{tool.id}",
                                        class: "py-0.5 px-1.5 bg-blue-500 bg-opacity-20 text-blue-300 rounded text-xs",
                                        "{tool.name}"
                                    }
                                }
                            }
                        }
                    }

                    // Edit / Regenerate controls
                    if props.on_regenerate.is_some() {
                        div {
                            class: "flex justify-end gap-2",

                            // Toggle edit/save
                            {
                                let mut editing_sig = editing.clone();
                                rsx! {
                                    button {
                                        onclick: move |_| {
                                            let current = *editing_sig.read();
                                            editing_sig.set(!current);
                                        },
                                        class: "py-1 px-2 bg-slate-400 bg-opacity-30 text-gray-200 border border-slate-600 rounded cursor-pointer text-xs uppercase",
                                        if *editing.read() { "Done Editing" } else { "Edit" }
                                    }
                                }
                            }

                            // Regenerate (optionally incorporating edited text as guidance)
                            {
                                let request_id = props.request_id.clone();
                                let outcome_type = props.outcome_type.clone();
                                let handler = props.on_regenerate.clone();
                                let edited_flavor_sig = edited_flavor.clone();
                                let edited_direction_sig = edited_direction.clone();

                                rsx! {
                                    button {
                                        onclick: move |_| {
                                            if let Some(h) = &handler {
                                                let mut guidance_parts = Vec::new();
                                                let flavor = edited_flavor_sig.read().trim().to_string();
                                                let direction = edited_direction_sig.read().trim().to_string();
                                                if !flavor.is_empty() {
                                                    guidance_parts.push(format!("Flavor: {}", flavor));
                                                }
                                                if !direction.is_empty() {
                                                    guidance_parts.push(format!("Scene: {}", direction));
                                                }
                                                let guidance = if guidance_parts.is_empty() {
                                                    None
                                                } else {
                                                    Some(guidance_parts.join(" | "))
                                                };

                                                h.call(RegenerateOutcomeData {
                                                    request_id: request_id.clone(),
                                                    outcome_type: Some(outcome_type.clone()),
                                                    guidance,
                                                });
                                            }
                                        },
                                        class: "py-1 px-2 bg-blue-500 bg-opacity-30 text-blue-300 border-0 rounded cursor-pointer text-xs uppercase",
                                        "Regenerate"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
