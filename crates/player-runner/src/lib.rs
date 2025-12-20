use wrldbldr_player_app::application::api::Api;
use wrldbldr_player_ports::{
    config::RunnerConfig,
    outbound::Platform,
};

pub struct RunnerDeps {
    pub platform: Platform,
    pub api: Api,
    pub config: RunnerConfig,
}

pub fn run(deps: RunnerDeps) {
    let RunnerDeps {
        platform,
        api,
        config,
    } = deps;

    dioxus::LaunchBuilder::new()
        .with_context(platform)
        .with_context(config)
        .with_context(wrldbldr_player_ui::presentation::Services::new(api))
        .with_context_provider(|| Box::new(wrldbldr_player_ui::presentation::state::GameState::new()))
        .with_context_provider(|| Box::new(wrldbldr_player_ui::presentation::state::SessionState::new()))
        .with_context_provider(|| Box::new(wrldbldr_player_ui::presentation::state::DialogueState::new()))
        .with_context_provider(|| Box::new(wrldbldr_player_ui::presentation::state::GenerationState::new()))
        .launch(wrldbldr_player_ui::app);
}
