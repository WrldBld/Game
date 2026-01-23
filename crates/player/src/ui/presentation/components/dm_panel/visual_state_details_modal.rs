//! Visual state details modal
//!
//! Shows full details of a visual state including assets, activation rules,
//! and metadata. Used when DM clicks "Details" from preview.

use dioxus::prelude::*;

use wrldbldr_shared::types::VisualStateSourceData;

/// Visual state details data (full)
#[derive(Clone, PartialEq, Debug)]
pub struct VisualStateDetailData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub backdrop_override: Option<String>,
    pub atmosphere_override: Option<String>,
    pub ambient_sound: Option<String>,
    pub priority: i32,
    pub is_default: bool,
    pub source: Option<VisualStateSourceData>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub usage_count: Option<u32>,
}

/// Props for VisualStateDetailsModal
#[derive(Props, Clone, PartialEq)]
pub struct VisualStateDetailsModalProps {
    /// State to show details for
    pub state: VisualStateDetailData,
    /// Whether to show "Edit State" button
    pub show_edit: bool,
    /// Handler when modal is closed
    pub on_close: EventHandler<()>,
    /// Handler to edit state (optional, if editor exists)
    pub on_edit: Option<EventHandler<()>>,
}

/// Visual state details modal
#[component]
pub fn VisualStateDetailsModal(props: VisualStateDetailsModalProps) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 bg-black/80 flex items-center justify-center z-[1000] p-4",
            onclick: move |_| props.on_close.call(()),

            // Modal
            div {
                class: "bg-gradient-to-br from-dark-surface to-dark-bg rounded-2xl max-w-3xl w-full max-h-[90vh] overflow-hidden border border-amber-500/30 flex flex-col",
                onclick: |e| e.stop_propagation(),

                // Header
                div {
                    class: "p-6 border-b border-white/10",
                    div {
                        class: "flex justify-between items-start",
                        div {
                            h2 {
                                class: "text-xl font-bold text-amber-400 m-0 mb-2",
                                "🖼️ Visual State Details: {props.state.name}"
                            }
                            if let Some(ref source) = props.state.source {
                                div {
                                    class: "text-xs text-gray-500",
                                    match source {
                                        VisualStateSourceData::HardRulesOnly => "Auto-resolved by rules",
                                        VisualStateSourceData::WithLlmEvaluation => "Auto-resolved with LLM",
                                        VisualStateSourceData::DmOverride => "DM selected",
                                        VisualStateSourceData::Default => "Default state",
                                        _ => "Unknown source",
                                    }
                                }
                            }
                        }
                        button {
                            onclick: move |_| props.on_close.call(()),
                            class: "p-2 text-gray-400 hover:text-white transition-colors",
                            "X"
                        }
                    }
                }

                // Content
                div {
                    class: "flex-1 overflow-y-auto p-6 space-y-6",

                    // Full-size backdrop preview
                    if let Some(ref backdrop) = props.state.backdrop_override {
                        div {
                            class: "bg-black/50 rounded-lg overflow-hidden",
                            img {
                                src: "{backdrop}",
                                class: "w-full h-auto",
                                alt: "{props.state.name}"
                            }
                        }
                    }

                    // Description
                    div {
                        h3 {
                            class: "text-white font-medium mb-2",
                            "Description"
                        }
                        if let Some(ref desc) = props.state.description {
                            p {
                                class: "text-gray-300 leading-relaxed",
                                "{desc}"
                            }
                        } else {
                            p {
                                class: "text-gray-500 italic",
                                "No description provided"
                            }
                        }
                    }

                    // Activation Rules
                    div {
                        h3 {
                            class: "text-white font-medium mb-3",
                            "Priority & Default Status"
                        }
                        div {
                            class: "space-y-2",
                            div {
                                class: "flex items-center gap-2",
                                span { class: "text-gray-400", "Priority:" }
                                span {
                                    class: "px-2 py-0.5 bg-gray-700 rounded text-white font-mono",
                                    "{props.state.priority}"
                                }
                                if props.state.is_default {
                                    span {
                                        class: "px-2 py-0.5 bg-amber-500/30 text-amber-300 rounded text-xs",
                                        "[DEFAULT]"
                                    }
                                }
                            }
                        }
                    }

                    // Assets
                    div {
                        h3 {
                            class: "text-white font-medium mb-3",
                            "Assets"
                        }
                        div {
                            class: "space-y-2",
                            if let Some(ref backdrop) = props.state.backdrop_override {
                                div {
                                    class: "flex justify-between items-center p-2 bg-black/20 rounded",
                                    span { class: "text-gray-400", "Backdrop:" }
                                    code {
                                        class: "text-purple-300 font-mono text-sm ml-2",
                                        "{backdrop}"
                                    }
                                }
                            }
                            if let Some(ref sound) = props.state.ambient_sound {
                                div {
                                    class: "flex justify-between items-center p-2 bg-black/20 rounded",
                                    span { class: "text-gray-400", "Ambient Sound:" }
                                    code {
                                        class: "text-purple-300 font-mono text-sm ml-2",
                                        "{sound}"
                                    }
                                }
                            }
                            if props.state.backdrop_override.is_none() && props.state.ambient_sound.is_none() {
                                div {
                                    class: "p-2 bg-black/20 rounded text-gray-500 italic text-sm",
                                    "No assets configured"
                                }
                            }
                        }
                    }

                    // Metadata
                    div {
                        h3 {
                            class: "text-white font-medium mb-3",
                            "Metadata"
                        }
                        div {
                            class: "space-y-2 text-sm",
                            div {
                                class: "flex justify-between",
                                span { class: "text-gray-400", "State ID:" }
                                code {
                                    class: "text-gray-300 font-mono",
                                    "{props.state.id}"
                                }
                            }
                            if let Some(ref created) = props.state.created_at {
                                div {
                                    class: "flex justify-between",
                                    span { class: "text-gray-400", "Created:" }
                                    span { class: "text-gray-300", "{created}" }
                                }
                            }
                            if let Some(ref updated) = props.state.updated_at {
                                div {
                                    class: "flex justify-between",
                                    span { class: "text-gray-400", "Updated:" }
                                    span { class: "text-gray-300", "{updated}" }
                                }
                            }
                            if let Some(ref count) = props.state.usage_count {
                                div {
                                    class: "flex justify-between",
                                    span { class: "text-gray-400", "Usage count:" }
                                    span {
                                        class: "text-amber-300 font-medium",
                                        "{count}x"
                                    }
                                }
                            }
                        }
                    }
                }

                // Footer
                div {
                    class: "p-6 border-t border-white/10 flex justify-end",
                    if let Some(on_edit) = props.on_edit {
                        if props.show_edit {
                            button {
                                onclick: move |_| on_edit.call(()),
                                class: "px-6 py-2 bg-gradient-to-br from-amber-500 to-amber-600 text-white font-semibold rounded-lg hover:from-amber-400 hover:to-amber-500 transition-all",
                                "✏️ Edit State"
                            }
                        }
                    }
                }
            }
        }
    }
}
