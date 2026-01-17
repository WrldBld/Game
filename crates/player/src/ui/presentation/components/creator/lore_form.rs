//! Lore Form - Creator Mode component for creating/editing lore entries
//!
//! US-LORE-001: DM can create lore entries with multiple chunks.
//! US-LORE-002: DM can link lore to characters, locations, regions, or items.
//! US-LORE-003: DM can mark lore as common knowledge.

use dioxus::prelude::*;

/// Categories for lore
const LORE_CATEGORIES: &[(&str, &str)] = &[
    ("historical", "Historical"),
    ("legend", "Legend"),
    ("secret", "Secret"),
    ("common", "Common"),
    ("technical", "Technical"),
    ("political", "Political"),
    ("natural", "Natural"),
    ("religious", "Religious"),
];

/// A lore chunk being edited
#[derive(Clone, Debug, PartialEq, Default)]
pub struct LoreChunkEdit {
    pub id: Option<String>,
    pub title: String,
    pub content: String,
    pub discovery_hint: String,
}

/// Props for the LoreForm component
#[derive(Props, Clone, PartialEq)]
pub struct LoreFormProps {
    /// World ID for creating new lore
    pub world_id: String,
    /// Lore ID if editing existing (empty for new)
    #[props(default)]
    pub lore_id: String,
    /// Handler for when form is saved
    #[props(default)]
    pub on_save: Option<EventHandler<LoreFormData>>,
    /// Handler for closing the form
    pub on_close: EventHandler<()>,
}

/// Data from the lore form for submission
#[derive(Clone, Debug, PartialEq)]
pub struct LoreFormData {
    pub lore_id: Option<String>,
    pub title: String,
    pub summary: String,
    pub category: String,
    pub tags: Vec<String>,
    pub is_common_knowledge: bool,
    pub chunks: Vec<LoreChunkEdit>,
}

/// Lore Form - for creating and editing lore entries
#[component]
pub fn LoreForm(props: LoreFormProps) -> Element {
    let is_new = props.lore_id.is_empty();

    // Form state
    let mut title = use_signal(String::new);
    let mut summary = use_signal(String::new);
    let mut category = use_signal(|| "common".to_string());
    let mut tags_input = use_signal(String::new);
    let mut is_common_knowledge = use_signal(|| false);
    let mut chunks: Signal<Vec<LoreChunkEdit>> = use_signal(Vec::new);

    // Loading state
    let is_loading = use_signal(|| false);
    let is_saving = use_signal(|| false);
    let error: Signal<Option<String>> = use_signal(|| None);

    // Load existing lore data if editing
    // TODO: Fetch lore by ID when editing

    // Handle save
    let handle_save = {
        let title = title;
        let summary = summary;
        let category = category;
        let tags_input = tags_input;
        let is_common_knowledge = is_common_knowledge;
        let chunks = chunks;
        let lore_id = props.lore_id.clone();
        let on_save = props.on_save;

        move |_| {
            if let Some(handler) = on_save {
                let tags: Vec<String> = tags_input
                    .read()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                let data = LoreFormData {
                    lore_id: if lore_id.is_empty() {
                        None
                    } else {
                        Some(lore_id.clone())
                    },
                    title: title.read().clone(),
                    summary: summary.read().clone(),
                    category: category.read().clone(),
                    tags,
                    is_common_knowledge: *is_common_knowledge.read(),
                    chunks: chunks.read().clone(),
                };

                handler.call(data);
            }
        }
    };

    // Add a new chunk
    let add_chunk = move |_| {
        let mut current = chunks.read().clone();
        current.push(LoreChunkEdit::default());
        chunks.set(current);
    };

    // Remove a chunk by index
    let mut remove_chunk = move |index: usize| {
        let mut current = chunks.read().clone();
        if index < current.len() {
            current.remove(index);
            chunks.set(current);
        }
    };

    rsx! {
        div {
            class: "lore-form flex flex-col h-full bg-dark-surface rounded-lg overflow-hidden",

            // Header
            div {
                class: "p-4 border-b border-white/10 flex justify-between items-center",

                h2 {
                    class: "text-lg font-bold text-white m-0",
                    if is_new { "New Lore Entry" } else { "Edit Lore Entry" }
                }

                button {
                    class: "w-8 h-8 flex items-center justify-center bg-white/5 hover:bg-white/10 rounded-lg text-gray-400 hover:text-white transition-colors",
                    onclick: move |_| props.on_close.call(()),
                    "x"
                }
            }

            // Form content
            div {
                class: "flex-1 overflow-y-auto p-4 space-y-4",

                if *is_loading.read() {
                    div {
                        class: "flex items-center justify-center py-12",
                        span { class: "text-gray-400", "Loading..." }
                    }
                } else {
                    // Error display
                    if let Some(ref err) = *error.read() {
                        div {
                            class: "p-3 bg-red-500/20 border border-red-500/30 rounded-lg text-red-400 text-sm",
                            "{err}"
                        }
                    }

                    // Title
                    div {
                        class: "form-group",
                        label {
                            class: "block text-sm font-medium text-gray-400 mb-1",
                            "Title *"
                        }
                        input {
                            class: "w-full px-3 py-2 bg-black/30 border border-white/10 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500/50",
                            r#type: "text",
                            placeholder: "e.g., The Fall of House Valeren",
                            value: "{title}",
                            oninput: move |e| title.set(e.value().clone()),
                        }
                    }

                    // Category and Common Knowledge
                    div {
                        class: "grid grid-cols-2 gap-4",

                        div {
                            class: "form-group",
                            label {
                                class: "block text-sm font-medium text-gray-400 mb-1",
                                "Category"
                            }
                            select {
                                class: "w-full px-3 py-2 bg-black/30 border border-white/10 rounded-lg text-white focus:outline-none focus:border-indigo-500/50",
                                value: "{category}",
                                onchange: move |e| category.set(e.value().clone()),

                                for (value, label) in LORE_CATEGORIES.iter() {
                                    option {
                                        key: "{value}",
                                        value: "{value}",
                                        "{label}"
                                    }
                                }
                            }
                        }

                        div {
                            class: "form-group flex items-end",
                            label {
                                class: "flex items-center gap-2 cursor-pointer",
                                input {
                                    class: "w-4 h-4 rounded border-white/10 bg-black/30 text-indigo-500 focus:ring-indigo-500/50",
                                    r#type: "checkbox",
                                    checked: *is_common_knowledge.read(),
                                    onchange: move |e| is_common_knowledge.set(e.checked()),
                                }
                                span {
                                    class: "text-sm text-gray-300",
                                    "Common Knowledge"
                                }
                            }
                        }
                    }

                    // Summary
                    div {
                        class: "form-group",
                        label {
                            class: "block text-sm font-medium text-gray-400 mb-1",
                            "Summary (DM Reference)"
                        }
                        textarea {
                            class: "w-full px-3 py-2 bg-black/30 border border-white/10 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500/50 resize-none",
                            rows: 2,
                            placeholder: "Brief description for DM reference...",
                            value: "{summary}",
                            oninput: move |e| summary.set(e.value().clone()),
                        }
                    }

                    // Tags
                    div {
                        class: "form-group",
                        label {
                            class: "block text-sm font-medium text-gray-400 mb-1",
                            "Tags (comma-separated)"
                        }
                        input {
                            class: "w-full px-3 py-2 bg-black/30 border border-white/10 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500/50",
                            r#type: "text",
                            placeholder: "e.g., history, noble houses, cursed",
                            value: "{tags_input}",
                            oninput: move |e| tags_input.set(e.value().clone()),
                        }
                    }

                    // Chunks section
                    div {
                        class: "form-group",

                        div {
                            class: "flex justify-between items-center mb-2",
                            label {
                                class: "text-sm font-medium text-gray-400",
                                "Lore Chunks"
                            }
                            button {
                                class: "px-3 py-1 text-sm bg-indigo-500/20 hover:bg-indigo-500/30 text-indigo-400 rounded transition-colors",
                                onclick: add_chunk,
                                "+ Add Chunk"
                            }
                        }

                        if chunks.read().is_empty() {
                            div {
                                class: "p-4 bg-black/20 rounded-lg text-center text-gray-500 text-sm",
                                "No chunks yet. Add chunks to break the lore into discoverable pieces."
                            }
                        } else {
                            div {
                                class: "space-y-3",

                                for (index, _chunk) in chunks.read().iter().enumerate() {
                                    LoreChunkEditor {
                                        key: "{index}",
                                        index: index,
                                        chunks_signal: chunks,
                                        on_remove: move |_| remove_chunk(index),
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Footer with actions
            div {
                class: "p-4 border-t border-white/10 flex justify-end gap-3",

                button {
                    class: "px-4 py-2 bg-white/5 hover:bg-white/10 text-gray-400 rounded-lg transition-colors",
                    onclick: move |_| props.on_close.call(()),
                    "Cancel"
                }

                button {
                    class: "px-4 py-2 bg-indigo-500 hover:bg-indigo-600 text-white rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                    disabled: *is_saving.read() || title.read().is_empty(),
                    onclick: handle_save,
                    if *is_saving.read() { "Saving..." } else { "Save Lore" }
                }
            }
        }
    }
}

/// Props for LoreChunkEditor
#[derive(Props, Clone, PartialEq)]
struct LoreChunkEditorProps {
    index: usize,
    chunks_signal: Signal<Vec<LoreChunkEdit>>,
    on_remove: EventHandler<()>,
}

/// Editor for a single lore chunk
#[component]
fn LoreChunkEditor(props: LoreChunkEditorProps) -> Element {
    let mut chunks = props.chunks_signal;
    let index = props.index;

    // Get current chunk data
    let chunk = chunks.read().get(index).cloned().unwrap_or_default();

    // Update chunk field
    let update_field = move |field: &str, value: String| {
        let mut current = chunks.read().clone();
        if let Some(c) = current.get_mut(index) {
            match field {
                "title" => c.title = value,
                "content" => c.content = value,
                "discovery_hint" => c.discovery_hint = value,
                _ => {}
            }
            chunks.set(current);
        }
    };

    rsx! {
        div {
            class: "lore-chunk-editor bg-black/20 rounded-lg p-3 border border-white/5",

            // Chunk header
            div {
                class: "flex justify-between items-center mb-2",
                span {
                    class: "text-sm font-medium text-gray-400",
                    "Chunk {index + 1}"
                }
                button {
                    class: "w-6 h-6 flex items-center justify-center text-gray-500 hover:text-red-400 transition-colors",
                    onclick: move |_| props.on_remove.call(()),
                    "x"
                }
            }

            // Chunk title
            input {
                class: "w-full px-2 py-1.5 mb-2 bg-black/30 border border-white/10 rounded text-white text-sm placeholder-gray-500 focus:outline-none focus:border-indigo-500/50",
                r#type: "text",
                placeholder: "Chunk title (optional)",
                value: "{chunk.title}",
                oninput: {
                    let mut update = update_field;
                    move |e| update("title", e.value().clone())
                },
            }

            // Chunk content
            textarea {
                class: "w-full px-2 py-1.5 mb-2 bg-black/30 border border-white/10 rounded text-white text-sm placeholder-gray-500 focus:outline-none focus:border-indigo-500/50 resize-none",
                rows: 3,
                placeholder: "Lore content for this chunk...",
                value: "{chunk.content}",
                oninput: {
                    let mut update = update_field;
                    move |e| update("content", e.value().clone())
                },
            }

            // Discovery hint
            input {
                class: "w-full px-2 py-1.5 bg-black/30 border border-white/10 rounded text-white text-sm placeholder-gray-500 focus:outline-none focus:border-indigo-500/50",
                r#type: "text",
                placeholder: "Discovery hint (e.g., 'Found in ancient library')",
                value: "{chunk.discovery_hint}",
                oninput: {
                    let mut update = update_field;
                    move |e| update("discovery_hint", e.value().clone())
                },
            }
        }
    }
}
