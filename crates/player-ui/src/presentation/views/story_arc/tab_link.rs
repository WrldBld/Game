//! Story Arc tab link component

use dioxus::prelude::*;
use crate::routes::Route;

#[derive(Props, Clone, PartialEq)]
pub struct StoryArcTabLinkProps {
    pub label: &'static str,
    pub icon: &'static str,
    pub subtab: &'static str,
    pub world_id: String,
    pub is_active: bool,
}

#[component]
pub fn StoryArcTabLink(props: StoryArcTabLinkProps) -> Element {
    // Extract conditional classes before rsx! block
    let link_classes = if props.is_active {
        "py-3 px-5 cursor-pointer flex items-center gap-2 text-sm transition-all duration-200 no-underline bg-dark-surface text-white border-b-2 border-purple-500"
    } else {
        "py-3 px-5 cursor-pointer flex items-center gap-2 text-sm transition-all duration-200 no-underline bg-transparent text-gray-400 border-b-2 border-transparent"
    };

    rsx! {
        Link {
            to: Route::DMStoryArcSubTabRoute {
                world_id: props.world_id.clone(),
                subtab: props.subtab.to_string(),
            },
            class: "{link_classes}",
            span { "{props.icon}" }
            span { "{props.label}" }
        }
    }
}
