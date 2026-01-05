//! Lore Journal Panel - Player UI for viewing discovered lore
//!
//! US-LORE-005: Player can view their character's known lore in a journal/codex.
//! US-LORE-006: Player sees partial lore entries when they only know some chunks.

use dioxus::prelude::*;

/// Lore entry data for display
#[derive(Clone, Debug, PartialEq)]
pub struct LoreEntryDisplay {
    pub id: String,
    pub title: String,
    pub category: String,
    pub category_icon: &'static str,
    /// All chunks in the lore entry
    pub chunks: Vec<LoreChunkDisplay>,
    /// Which chunk IDs are known (empty = all known)
    pub known_chunk_ids: Vec<String>,
    /// How this lore was discovered
    pub discovery_source: String,
    /// When discovered (formatted string)
    pub discovered_at: String,
    /// Tags for filtering
    pub tags: Vec<String>,
}

impl LoreEntryDisplay {
    /// Returns true if the character knows the entire lore entry
    pub fn knows_all(&self) -> bool {
        self.known_chunk_ids.is_empty() || self.known_chunk_ids.len() >= self.chunks.len()
    }

    /// Returns the number of known chunks
    pub fn known_count(&self) -> usize {
        if self.known_chunk_ids.is_empty() {
            self.chunks.len()
        } else {
            self.known_chunk_ids.len()
        }
    }

    /// Check if a specific chunk is known
    pub fn is_chunk_known(&self, chunk_id: &str) -> bool {
        self.known_chunk_ids.is_empty() || self.known_chunk_ids.iter().any(|id| id == chunk_id)
    }
}

/// Lore chunk data for display
#[derive(Clone, Debug, PartialEq)]
pub struct LoreChunkDisplay {
    pub id: String,
    pub title: Option<String>,
    pub content: String,
    pub order: u32,
}

/// Props for the LoreJournal component
#[derive(Props, Clone, PartialEq)]
pub struct LoreJournalProps {
    /// Character name for display
    pub character_name: String,
    /// All known lore entries
    pub lore_entries: Vec<LoreEntryDisplay>,
    /// Whether data is still loading
    #[props(default = false)]
    pub is_loading: bool,
    /// Handler for closing the panel
    pub on_close: EventHandler<()>,
    /// Handler for clicking a lore entry (to view full details)
    #[props(default)]
    pub on_lore_click: Option<EventHandler<String>>,
}

/// Get icon for lore category
fn category_icon(category: &str) -> &'static str {
    match category.to_lowercase().as_str() {
        "historical" => "@",
        "legend" => "*",
        "secret" => "?",
        "common" => ".",
        "technical" => "#",
        "political" => "%",
        "natural" => "~",
        "religious" => "+",
        _ => "o",
    }
}

/// Lore Journal Panel - modal overlay showing discovered lore
#[component]
pub fn LoreJournal(props: LoreJournalProps) -> Element {
    let mut selected_category = use_signal(|| "all".to_string());
    let mut search_query = use_signal(|| String::new());
    let mut expanded_entry = use_signal(|| None::<String>);

    // Get unique categories
    let categories: Vec<String> = {
        let mut cats: Vec<String> = props
            .lore_entries
            .iter()
            .map(|l| l.category.clone())
            .collect();
        cats.sort();
        cats.dedup();
        cats
    };

    // Filter entries
    let filtered_entries: Vec<&LoreEntryDisplay> = props
        .lore_entries
        .iter()
        .filter(|entry| {
            let cat = selected_category.read();
            let query = search_query.read();

            // Category filter
            let cat_match = *cat == "all" || entry.category.to_lowercase() == cat.to_lowercase();

            // Search filter
            let search_match = query.is_empty()
                || entry.title.to_lowercase().contains(&query.to_lowercase())
                || entry
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&query.to_lowercase()));

            cat_match && search_match
        })
        .collect();

    rsx! {
        // Overlay background
        div {
            class: "lore-journal-overlay fixed inset-0 bg-black/85 z-[1000] flex items-center justify-center p-4",
            onclick: move |_| props.on_close.call(()),

            // Panel container
            div {
                class: "lore-journal-panel bg-gradient-to-br from-dark-surface to-dark-bg rounded-2xl w-full max-w-3xl max-h-[85vh] overflow-hidden flex flex-col shadow-2xl border border-indigo-500/20",
                onclick: move |e| e.stop_propagation(),

                // Header
                div {
                    class: "p-4 border-b border-white/10",

                    div {
                        class: "flex justify-between items-start mb-4",

                        div {
                            h2 {
                                class: "text-xl font-bold text-white m-0",
                                "Lore Journal"
                            }
                            p {
                                class: "text-gray-400 text-sm m-0 mt-1",
                                "{props.character_name}'s discovered knowledge"
                            }
                        }

                        button {
                            class: "w-8 h-8 flex items-center justify-center bg-white/5 hover:bg-white/10 rounded-lg text-gray-400 hover:text-white transition-colors",
                            onclick: move |_| props.on_close.call(()),
                            "x"
                        }
                    }

                    // Search and filters
                    div {
                        class: "flex gap-3 flex-wrap",

                        // Search input
                        input {
                            class: "flex-1 min-w-[200px] px-3 py-2 bg-black/30 border border-white/10 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-indigo-500/50",
                            r#type: "text",
                            placeholder: "Search lore...",
                            value: "{search_query}",
                            oninput: move |e| search_query.set(e.value().clone()),
                        }

                        // Category filter
                        select {
                            class: "px-3 py-2 bg-black/30 border border-white/10 rounded-lg text-white focus:outline-none focus:border-indigo-500/50",
                            value: "{selected_category}",
                            onchange: move |e| selected_category.set(e.value().clone()),

                            option { value: "all", "All Categories" }
                            for cat in categories.iter() {
                                option {
                                    key: "{cat}",
                                    value: "{cat}",
                                    "{category_icon(cat)} {cat}"
                                }
                            }
                        }
                    }
                }

                // Content
                div {
                    class: "flex-1 overflow-y-auto p-4",

                    if props.is_loading {
                        div {
                            class: "flex items-center justify-center py-12",
                            span {
                                class: "text-gray-400",
                                "Loading lore..."
                            }
                        }
                    } else if filtered_entries.is_empty() {
                        div {
                            class: "flex flex-col items-center justify-center py-12 text-center",
                            span {
                                class: "text-4xl mb-4",
                                "?"
                            }
                            if props.lore_entries.is_empty() {
                                p {
                                    class: "text-gray-400 m-0",
                                    "No lore discovered yet."
                                }
                                p {
                                    class: "text-gray-500 text-sm m-0 mt-2",
                                    "Explore the world and talk to NPCs to learn more."
                                }
                            } else {
                                p {
                                    class: "text-gray-400 m-0",
                                    "No lore matches your search."
                                }
                            }
                        }
                    } else {
                        div {
                            class: "space-y-3",

                            for entry in filtered_entries.iter() {
                                LoreEntryCard {
                                    key: "{entry.id}",
                                    entry: (*entry).clone(),
                                    is_expanded: expanded_entry.read().as_ref() == Some(&entry.id),
                                    on_toggle: {
                                        let entry_id = entry.id.clone();
                                        move |_| {
                                            let current = expanded_entry.read().clone();
                                            if current.as_ref() == Some(&entry_id) {
                                                expanded_entry.set(None);
                                            } else {
                                                expanded_entry.set(Some(entry_id.clone()));
                                            }
                                        }
                                    },
                                    on_click: props.on_lore_click,
                                }
                            }
                        }
                    }
                }

                // Footer with stats
                div {
                    class: "p-3 border-t border-white/10 flex justify-between items-center text-sm text-gray-500",

                    span {
                        "{filtered_entries.len()} of {props.lore_entries.len()} entries"
                    }

                    span {
                        {
                            let total_chunks: usize = props.lore_entries.iter().map(|e| e.chunks.len()).sum();
                            let known_chunks: usize = props.lore_entries.iter().map(|e| e.known_count()).sum();
                            format!("{known_chunks}/{total_chunks} chunks discovered")
                        }
                    }
                }
            }
        }
    }
}

/// Props for LoreEntryCard
#[derive(Props, Clone, PartialEq)]
struct LoreEntryCardProps {
    entry: LoreEntryDisplay,
    is_expanded: bool,
    on_toggle: EventHandler<()>,
    on_click: Option<EventHandler<String>>,
}

/// Card displaying a single lore entry
#[component]
fn LoreEntryCard(props: LoreEntryCardProps) -> Element {
    let completion_pct = if props.entry.chunks.is_empty() {
        100
    } else {
        (props.entry.known_count() * 100) / props.entry.chunks.len()
    };

    let border_class = if props.entry.knows_all() {
        "border-indigo-500/30"
    } else {
        "border-white/10"
    };

    rsx! {
        div {
            class: "lore-entry bg-black/30 rounded-lg border {border_class} overflow-hidden",

            // Entry header (always visible)
            button {
                class: "w-full p-4 flex items-start gap-3 text-left bg-transparent border-none cursor-pointer hover:bg-white/5 transition-colors",
                onclick: move |_| props.on_toggle.call(()),

                // Category icon
                span {
                    class: "text-lg w-6 text-center text-indigo-400 mt-0.5",
                    "{category_icon(&props.entry.category)}"
                }

                // Title and metadata
                div {
                    class: "flex-1 min-w-0",

                    div {
                        class: "flex items-center gap-2 flex-wrap",
                        span {
                            class: "text-white font-medium",
                            "{props.entry.title}"
                        }
                        if !props.entry.knows_all() {
                            span {
                                class: "text-xs text-amber-400/70 bg-amber-500/20 px-1.5 py-0.5 rounded",
                                "Partial"
                            }
                        }
                    }

                    div {
                        class: "flex items-center gap-2 mt-1 text-xs text-gray-500",
                        span { "{props.entry.category}" }
                        span { "|" }
                        span { "{props.entry.known_count()}/{props.entry.chunks.len()} chunks" }
                    }

                    // Tags
                    if !props.entry.tags.is_empty() {
                        div {
                            class: "flex flex-wrap gap-1 mt-2",
                            for tag in props.entry.tags.iter().take(3) {
                                span {
                                    key: "{tag}",
                                    class: "text-xs px-1.5 py-0.5 bg-white/5 text-gray-400 rounded",
                                    "#{tag}"
                                }
                            }
                            if props.entry.tags.len() > 3 {
                                span {
                                    class: "text-xs text-gray-500",
                                    "+{props.entry.tags.len() - 3}"
                                }
                            }
                        }
                    }
                }

                // Completion indicator
                div {
                    class: "flex flex-col items-end gap-1",

                    span {
                        class: "text-gray-500 text-sm",
                        if props.is_expanded { "v" } else { ">" }
                    }

                    // Progress bar
                    div {
                        class: "w-16 h-1 bg-white/10 rounded-full overflow-hidden",
                        div {
                            class: if completion_pct == 100 { "h-full bg-indigo-500" } else { "h-full bg-amber-500/70" },
                            style: "width: {completion_pct}%",
                        }
                    }
                }
            }

            // Expanded content
            if props.is_expanded {
                div {
                    class: "px-4 pb-4 border-t border-white/5 pt-4",

                    // Chunks
                    div {
                        class: "space-y-4",

                        for chunk in props.entry.chunks.iter() {
                            LoreChunkDisplay {
                                key: "{chunk.id}",
                                chunk: chunk.clone(),
                                is_known: props.entry.is_chunk_known(&chunk.id),
                            }
                        }
                    }

                    // Discovery info
                    div {
                        class: "mt-4 pt-3 border-t border-white/5 flex items-center justify-between text-xs text-gray-500",

                        span {
                            "Discovered: {props.entry.discovery_source}"
                        }
                        span {
                            "{props.entry.discovered_at}"
                        }
                    }
                }
            }
        }
    }
}

/// Props for LoreChunkDisplay
#[derive(Props, Clone, PartialEq)]
struct LoreChunkDisplayProps {
    chunk: LoreChunkDisplay,
    is_known: bool,
}

/// Display a single lore chunk (known or unknown)
#[component]
fn LoreChunkDisplay(props: LoreChunkDisplayProps) -> Element {
    rsx! {
        div {
            class: if props.is_known {
                "lore-chunk bg-white/5 rounded-lg p-3"
            } else {
                "lore-chunk bg-black/20 rounded-lg p-3 opacity-50"
            },

            if let Some(ref title) = props.chunk.title {
                h4 {
                    class: "text-sm font-medium text-indigo-300 m-0 mb-2",
                    if props.is_known {
                        "{title}"
                    } else {
                        "???"
                    }
                }
            }

            p {
                class: "text-gray-300 text-sm m-0 leading-relaxed whitespace-pre-wrap",
                if props.is_known {
                    "{props.chunk.content}"
                } else {
                    "This knowledge has not yet been discovered..."
                }
            }
        }
    }
}
