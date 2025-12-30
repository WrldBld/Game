//! Main menu view - Connect to a game server

use dioxus::prelude::*;

#[component]
pub fn MainMenu(on_connect: EventHandler<String>) -> Element {
    let mut server_url = use_signal(|| "ws://localhost:3000/ws".to_string());

    rsx! {
        div {
            class: "main-menu flex flex-col items-center justify-center h-full bg-gradient-to-br from-dark-surface to-dark-gradient-end",

            div {
                class: "menu-card bg-dark-bg p-12 rounded-xl shadow-2xl max-w-md w-[90%]",

                h1 {
                    class: "text-white text-center mb-2 text-4xl",
                    "WrldBldr"
                }
                p {
                    class: "text-gray-400 text-center mb-8",
                    "TTRPG Game Client"
                }

                div {
                    class: "mb-6",

                    label {
                        class: "block text-gray-400 mb-2 text-sm",
                        "Server Address"
                    }
                    input {
                        r#type: "text",
                        value: "{server_url}",
                        oninput: move |e| server_url.set(e.value()),
                        class: "w-full p-3 border border-gray-700 rounded-lg bg-gray-800 text-white text-base box-border",
                        placeholder: "ws://localhost:3000/ws"
                    }
                }

                button {
                    onclick: move |_| on_connect.call(server_url.read().clone()),
                    class: "w-full py-3.5 bg-blue-500 text-white border-0 rounded-lg text-base font-semibold cursor-pointer transition-colors duration-200 hover:bg-blue-600",
                    "Connect to Server"
                }
            }
        }
    }
}
