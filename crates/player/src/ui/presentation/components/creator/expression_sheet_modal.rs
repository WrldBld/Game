//! Expression Sheet Generation Modal
//!
//! Specialized modal for generating expression sprite sheets.
//! Shows a grid preview of expressions to be generated and allows
//! customization of the generation workflow.
//!
//! Part of the three-tier emotional model (Tier 3: Expression).

use dioxus::prelude::*;

use wrldbldr_domain::ExpressionConfig;

/// Standard expression order for 4x4 grid generation
pub const STANDARD_EXPRESSION_ORDER: [&str; 16] = [
    "neutral",
    "happy",
    "sad",
    "angry",
    "surprised",
    "afraid",
    "thoughtful",
    "suspicious",
    "curious",
    "confused",
    "worried",
    "excited",
    "confident",
    "nervous",
    "amused",
    "calm",
];

/// Props for ExpressionSheetModal
#[derive(Props, Clone, PartialEq)]
pub struct ExpressionSheetModalProps {
    /// Character ID to generate expressions for
    pub character_id: String,
    /// Character name for display
    pub character_name: String,
    /// Current expression config (to know which expressions are needed)
    pub expression_config: ExpressionConfig,
    /// Handler when generation is triggered
    pub on_generate: EventHandler<ExpressionSheetRequest>,
    /// Handler to close modal
    pub on_close: EventHandler<()>,
}

/// Request for expression sheet generation
#[derive(Clone, Debug)]
pub struct ExpressionSheetRequest {
    pub character_id: String,
    pub workflow: String,
    pub expressions: Vec<String>,
    pub grid_layout: (u32, u32),
    pub style_prompt: Option<String>,
}

/// Modal for configuring expression sheet generation
#[component]
pub fn ExpressionSheetModal(props: ExpressionSheetModalProps) -> Element {
    // State for selected expressions
    let mut selected_expressions = use_signal(|| {
        // Pre-select expressions from the character's config
        props.expression_config.expressions().to_vec()
    });
    let mut workflow = use_signal(|| "expression_sheet".to_string());
    let mut style_prompt = use_signal(String::new);
    let mut use_standard_grid = use_signal(|| true);
    let mut is_generating = use_signal(|| false);

    // Calculate grid layout based on expression count
    let grid_layout = use_memo(move || {
        if *use_standard_grid.read() {
            (4, 4) // Standard 16-expression grid
        } else {
            let count = selected_expressions.read().len();
            let cols = (count as f32).sqrt().ceil() as u32;
            let rows = ((count as f32) / cols as f32).ceil() as u32;
            (cols.max(1), rows.max(1))
        }
    });

    rsx! {
        div {
            class: "modal-overlay fixed inset-0 bg-black bg-opacity-80 flex items-center justify-center z-1000",
            onclick: move |_| props.on_close.call(()),

            div {
                class: "modal-content bg-dark-surface rounded-xl p-6 w-11/12 max-w-2xl max-h-[90vh] overflow-y-auto",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex justify-between items-center mb-4",

                    h3 {
                        class: "text-white m-0",
                        "Generate Expression Sheet"
                    }

                    button {
                        class: "text-gray-400 hover:text-white text-xl bg-transparent border-0 cursor-pointer",
                        onclick: move |_| props.on_close.call(()),
                        "×"
                    }
                }

                // Character info
                div {
                    class: "mb-4 p-3 bg-purple-900/20 rounded border border-purple-800/30",

                    div { class: "text-purple-300 text-sm mb-1", "Character" }
                    div { class: "text-white", "{props.character_name}" }
                }

                // Expression selection
                div {
                    class: "mb-4",

                    div {
                        class: "flex justify-between items-center mb-2",

                        label {
                            class: "text-gray-400 text-sm",
                            "Expressions to Generate ({selected_expressions.read().len()})"
                        }

                        div {
                            class: "flex gap-2",

                            button {
                                class: "text-xs text-purple-400 hover:text-purple-300",
                                onclick: move |_| {
                                    selected_expressions.set(
                                        STANDARD_EXPRESSION_ORDER.iter().map(|s| s.to_string()).collect()
                                    );
                                },
                                "Select All Standard"
                            }

                            button {
                                class: "text-xs text-gray-400 hover:text-gray-300",
                                onclick: move |_| {
                                    selected_expressions.set(Vec::new());
                                },
                                "Clear"
                            }
                        }
                    }

                    // Expression grid
                    div {
                        class: "grid grid-cols-4 gap-2 p-3 bg-dark-bg rounded border border-gray-700",

                        for expr in STANDARD_EXPRESSION_ORDER {
                            {
                                let is_selected = selected_expressions.read().iter().any(|e: &String| e.eq_ignore_ascii_case(expr));
                                let expr_str = expr.to_string();
                                rsx! {
                                    button {
                                        key: "{expr}",
                                        class: format!(
                                            "p-2 text-sm rounded border transition-all {}",
                                            if is_selected {
                                                "bg-purple-600 text-white border-purple-500"
                                            } else {
                                                "bg-gray-800 text-gray-400 border-gray-700 hover:border-purple-500"
                                            }
                                        ),
                                        onclick: move |_| {
                                            let mut current = selected_expressions.read().clone();
                                            if is_selected {
                                                current.retain(|e: &String| !e.eq_ignore_ascii_case(&expr_str));
                                            } else {
                                                current.push(expr_str.clone());
                                            }
                                            selected_expressions.set(current);
                                        },
                                        "{expr}"
                                    }
                                }
                            }
                        }
                    }

                    // Custom expression input
                    {
                        let mut custom_expr_input = use_signal(String::new);
                        rsx! {
                            div {
                                class: "mt-2 flex gap-2",

                                input {
                                    class: "flex-1 p-2 bg-dark-bg border border-gray-700 rounded text-white text-sm",
                                    r#type: "text",
                                    placeholder: "Add custom expression...",
                                    value: "{custom_expr_input}",
                                    oninput: move |e| custom_expr_input.set(e.value()),
                                    onkeypress: move |e: KeyboardEvent| {
                                        if e.key() == Key::Enter {
                                            let expr = custom_expr_input.read().trim().to_string();
                                            if !expr.is_empty() {
                                                let mut current = selected_expressions.read().clone();
                                                if !current.iter().any(|e: &String| e.eq_ignore_ascii_case(&expr)) {
                                                    current.push(expr);
                                                    selected_expressions.set(current);
                                                }
                                                custom_expr_input.set(String::new());
                                            }
                                        }
                                    },
                                }

                                button {
                                    class: "px-3 py-2 bg-purple-600 text-white rounded text-sm hover:bg-purple-500",
                                    onclick: move |_| {
                                        let expr = custom_expr_input.read().trim().to_string();
                                        if !expr.is_empty() {
                                            let mut current = selected_expressions.read().clone();
                                            if !current.iter().any(|e: &String| e.eq_ignore_ascii_case(&expr)) {
                                                current.push(expr);
                                                selected_expressions.set(current);
                                            }
                                            custom_expr_input.set(String::new());
                                        }
                                    },
                                    "Add"
                                }
                            }
                        }
                    }
                }

                // Grid layout options
                div {
                    class: "mb-4",

                    label {
                        class: "flex items-center gap-2 text-gray-400 text-sm cursor-pointer",

                        input {
                            r#type: "checkbox",
                            checked: *use_standard_grid.read(),
                            onchange: move |e| use_standard_grid.set(e.checked()),
                        }

                        "Use standard 4x4 grid (16 expressions)"
                    }

                    if !*use_standard_grid.read() {
                        div {
                            class: "mt-2 text-xs text-gray-500",
                            "Grid will be {grid_layout.read().0}x{grid_layout.read().1} based on {selected_expressions.read().len()} expressions"
                        }
                    }
                }

                // Workflow selection
                div {
                    class: "mb-4",

                    label {
                        class: "block text-gray-400 text-sm mb-2",
                        "ComfyUI Workflow"
                    }

                    select {
                        class: "w-full p-2 bg-dark-bg border border-gray-700 rounded text-white",
                        value: "{workflow}",
                        onchange: move |e| workflow.set(e.value()),

                        option { value: "expression_sheet", "Expression Sheet (Default)" }
                        option { value: "expression_sheet_anime", "Expression Sheet (Anime Style)" }
                        option { value: "expression_sheet_realistic", "Expression Sheet (Realistic)" }
                    }
                }

                // Style prompt
                div {
                    class: "mb-6",

                    label {
                        class: "block text-gray-400 text-sm mb-2",
                        "Style Prompt (optional)"
                    }

                    textarea {
                        class: "w-full min-h-[60px] p-2 bg-dark-bg border border-gray-700 rounded text-white resize-y",
                        placeholder: "Additional style guidance for generation...",
                        value: "{style_prompt}",
                        oninput: move |e| style_prompt.set(e.value()),
                    }
                }

                // Preview section
                div {
                    class: "mb-6 p-4 bg-gray-800/50 rounded border border-gray-700",

                    h4 { class: "text-gray-400 text-sm mb-3", "Generation Preview" }

                    // Grid preview
                    div {
                        class: "grid gap-1 mb-3",
                        style: format!("grid-template-columns: repeat({}, 1fr);", grid_layout.read().0),

                        for (idx, expr) in selected_expressions.read().iter().enumerate() {
                            div {
                                key: "{idx}-{expr}",
                                class: "aspect-square bg-gray-700 rounded flex items-center justify-center text-xs text-gray-400 p-1 text-center overflow-hidden",
                                "{expr}"
                            }
                        }

                        // Fill remaining slots in grid
                        {
                            let total_slots = (grid_layout.read().0 * grid_layout.read().1) as usize;
                            let filled = selected_expressions.read().len();
                            let empty_slots = total_slots.saturating_sub(filled);
                            rsx! {
                                for i in 0..empty_slots {
                                    div {
                                        key: "empty-{i}",
                                        class: "aspect-square bg-gray-800 rounded flex items-center justify-center text-xs text-gray-600",
                                        "—"
                                    }
                                }
                            }
                        }
                    }

                    div {
                        class: "text-xs text-gray-500",
                        "This will generate a {grid_layout.read().0}x{grid_layout.read().1} sprite sheet with {selected_expressions.read().len()} expressions"
                    }
                }

                // Action buttons
                div {
                    class: "flex justify-end gap-2",

                    button {
                        class: "px-4 py-2 bg-transparent text-gray-400 border border-gray-700 rounded cursor-pointer",
                        onclick: move |_| props.on_close.call(()),
                        disabled: *is_generating.read(),
                        "Cancel"
                    }

                    button {
                        class: format!(
                            "px-4 py-2 bg-purple-600 text-white border-0 rounded cursor-pointer font-medium {}",
                            if selected_expressions.read().is_empty() || *is_generating.read() { "opacity-50 cursor-not-allowed" } else { "hover:bg-purple-500" }
                        ),
                        disabled: selected_expressions.read().is_empty() || *is_generating.read(),
                        onclick: {
                            let character_id = props.character_id.clone();
                            move |_| {
                                is_generating.set(true);

                                let request = ExpressionSheetRequest {
                                    character_id: character_id.clone(),
                                    workflow: workflow.read().clone(),
                                    expressions: selected_expressions.read().clone(),
                                    grid_layout: *grid_layout.read(),
                                    style_prompt: {
                                        let prompt = style_prompt.read().clone();
                                        if prompt.is_empty() { None } else { Some(prompt) }
                                    },
                                };

                                props.on_generate.call(request);
                            }
                        },
                        if *is_generating.read() { "Generating..." } else { "Generate Expression Sheet" }
                    }
                }
            }
        }
    }
}

/// Post-generation selection component for mapping expressions to sprites
///
/// Shown after an expression sheet is generated, allowing users to
/// assign expression names to each generated sprite.
#[derive(Props, Clone, PartialEq)]
pub struct ExpressionMappingProps {
    /// Generated sprite URLs (from sliced expression sheet)
    pub sprite_urls: Vec<String>,
    /// Expected expression names (in order)
    pub expected_expressions: Vec<String>,
    /// Handler when mapping is confirmed
    pub on_confirm: EventHandler<Vec<(String, String)>>,
    /// Handler to cancel
    pub on_cancel: EventHandler<()>,
}

/// Expression mapping component
#[component]
pub fn ExpressionMapping(props: ExpressionMappingProps) -> Element {
    // State: mapping from sprite index to expression name
    let mut mappings = use_signal(|| {
        // Initialize with expected expressions in order
        props
            .expected_expressions
            .iter()
            .enumerate()
            .map(|(idx, expr)| (idx, expr.clone()))
            .collect::<Vec<_>>()
    });

    rsx! {
        div {
            class: "expression-mapping p-4 bg-dark-surface rounded-lg",

            h3 { class: "text-white mb-4", "Map Expressions to Sprites" }

            div {
                class: "grid gap-4 mb-6",
                style: "grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));",

                for (idx, url) in props.sprite_urls.iter().enumerate() {
                    div {
                        key: "{idx}",
                        class: "flex flex-col items-center gap-2 p-3 bg-dark-bg rounded border border-gray-700",

                        // Sprite preview
                        img {
                            src: "{url}",
                            class: "w-24 h-24 object-contain rounded",
                            alt: "Expression sprite {idx}",
                        }

                        // Expression name dropdown
                        select {
                            class: "w-full p-2 bg-gray-800 border border-gray-700 rounded text-white text-sm",
                            value: mappings.read().iter().find(|(i, _)| *i == idx).map(|(_, e)| e.clone()).unwrap_or_default(),
                            onchange: move |e| {
                                let mut current = mappings.read().clone();
                                if let Some(entry) = current.iter_mut().find(|(i, _)| *i == idx) {
                                    entry.1 = e.value();
                                } else {
                                    current.push((idx, e.value()));
                                }
                                mappings.set(current);
                            },

                            option { value: "", "— Select —" }
                            for expr in &props.expected_expressions {
                                option {
                                    value: "{expr}",
                                    "{expr}"
                                }
                            }
                        }
                    }
                }
            }

            // Action buttons
            div {
                class: "flex justify-end gap-2",

                button {
                    class: "px-4 py-2 bg-transparent text-gray-400 border border-gray-700 rounded cursor-pointer",
                    onclick: move |_| props.on_cancel.call(()),
                    "Cancel"
                }

                button {
                    class: "px-4 py-2 bg-green-600 text-white border-0 rounded cursor-pointer font-medium hover:bg-green-500",
                    onclick: move |_| {
                        let result: Vec<(String, String)> = mappings.read().iter()
                            .filter_map(|(idx, expr)| {
                                if expr.is_empty() {
                                    None
                                } else {
                                    props.sprite_urls.get(*idx).map(|url| (expr.clone(), url.clone()))
                                }
                            })
                            .collect();
                        props.on_confirm.call(result);
                    },
                    "Confirm Mapping"
                }
            }
        }
    }
}
