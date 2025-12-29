//! PC (Player Character) creation route handler

use crate::presentation::state::SessionState;
use dioxus::prelude::*;
use wrldbldr_player_ports::outbound::Platform;

/// PC creation route
#[component]
pub fn PCCreationRoute(world_id: String) -> Element {
    let _navigator = use_navigator();
    let platform = use_context::<Platform>();
    let session_state = use_context::<SessionState>();

    // Set page title
    use_effect(move || {
        platform.set_page_title("Create Character");
    });

    // Get session_id from session state
    let session_id = session_state
        .session_id()
        .read()
        .clone()
        .unwrap_or_else(|| "".to_string());

    rsx! {
        crate::presentation::views::pc_creation::PCCreationView {
            session_id: session_id,
            world_id: world_id,
        }
    }
}
