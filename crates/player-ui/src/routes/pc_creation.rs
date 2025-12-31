//! PC (Player Character) creation route handler

use crate::use_platform;
use dioxus::prelude::*;

/// PC creation route
#[component]
pub fn PCCreationRoute(world_id: String) -> Element {
    let _navigator = use_navigator();
    let platform = use_platform();

    // Set page title
    use_effect(move || {
        platform.set_page_title("Create Character");
    });

    rsx! {
        crate::presentation::views::pc_creation::PCCreationView {
            world_id: world_id,
        }
    }
}
