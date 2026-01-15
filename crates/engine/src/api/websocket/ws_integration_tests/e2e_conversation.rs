//! End-to-end conversation flow tests with LLM integration.
//!
//! These tests verify the complete conversation flow from WebSocket to LLM and back.
//! Run with: `cargo test -p wrldbldr-engine e2e_conversation -- --ignored`

use super::*;

use std::sync::Arc;
use std::time::Duration;

use wrldbldr_domain::{value_objects::RuleSystemConfig, WorldId};
use wrldbldr_protocol::{
    ClientMessage, RequestPayload, ResponseResult, ServerMessage, WorldRequest,
};

use crate::infrastructure::ollama::OllamaClient;
use crate::infrastructure::ports::LlmPort;

/// Create a D&D 5e world for testing.
fn create_test_world(
    world_id: WorldId,
    now: chrono::DateTime<chrono::Utc>,
) -> wrldbldr_domain::World {
    let world_name = wrldbldr_domain::WorldName::new("Integration Test World").unwrap();
    wrldbldr_domain::World::new(world_name, now)
        .with_description(
            wrldbldr_domain::Description::new("A test world for LLM integration").unwrap(),
        )
        .with_id(world_id)
        .with_rule_system(RuleSystemConfig::dnd_5e())
}

/// Check if Ollama is available.
async fn check_ollama_available() -> bool {
    use crate::infrastructure::ports::{ChatMessage, LlmRequest};
    let client = OllamaClient::from_env();
    let request = LlmRequest::new(vec![ChatMessage::user("test")])
        .with_temperature(0.0)
        .with_max_tokens(Some(5));
    client.generate(request).await.is_ok()
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_e2e_join_world_and_get_template() {
    // This test verifies the basic flow: join world -> get sheet template
    // It uses the real Ollama client but only for the LLM-independent parts

    let now = chrono::Utc::now();
    let world_id = WorldId::new();
    let world = create_test_world(world_id, now);

    // Setup world repo
    let mut world_repo = MockWorldRepo::new();
    let world_for_get = world.clone();
    world_repo
        .expect_get()
        .returning(move |_| Ok(Some(world_for_get.clone())));

    let repos = TestAppRepos::new(world_repo);

    // Use real Ollama client
    let llm: Arc<dyn LlmPort> = Arc::new(OllamaClient::from_env());
    let queue = Arc::new(NoopQueue);

    let app = build_test_app_with_ports(repos, now, queue, llm);
    let connections = Arc::new(ConnectionManager::new());

    let ws_state = Arc::new(WsState {
        app,
        connections,
        pending_time_suggestions: Arc::new(TimeSuggestionStoreImpl::new()),
        pending_staging_requests: Arc::new(PendingStagingStoreImpl::new()),
        generation_read_state: GenerationStateStoreImpl::new(),
    });

    let (addr, server) = spawn_ws_server(ws_state.clone()).await;
    let mut ws = ws_connect(addr).await;

    // Join world as DM
    ws_send_client(
        &mut ws,
        &ClientMessage::JoinWorld {
            world_id: *world_id.as_uuid(),
            role: ProtoWorldRole::Dm,
            user_id: "test-user".to_string(),
            pc_id: None,
            spectate_pc_id: None,
        },
    )
    .await;

    // Should receive WorldJoined
    let joined = ws_expect_message(&mut ws, Duration::from_secs(5), |m| {
        matches!(m, ServerMessage::WorldJoined { .. })
    })
    .await;

    if let ServerMessage::WorldJoined {
        world_id: joined_id,
        ..
    } = joined
    {
        assert_eq!(joined_id, *world_id.as_uuid());
    } else {
        panic!("Expected WorldJoined");
    }

    // Request sheet template
    let request_id = uuid::Uuid::new_v4().to_string();
    ws_send_client(
        &mut ws,
        &ClientMessage::Request {
            request_id: request_id.clone(),
            payload: RequestPayload::World(WorldRequest::GetSheetTemplate {
                world_id: world_id.as_uuid().to_string(),
            }),
        },
    )
    .await;

    // Should receive schema response
    let response = ws_expect_message(&mut ws, Duration::from_secs(5), |m| {
        if let ServerMessage::Response {
            request_id: rid, ..
        } = m
        {
            rid == &request_id
        } else {
            false
        }
    })
    .await;

    if let ServerMessage::Response { result, .. } = response {
        match result {
            ResponseResult::Success { data } => {
                let data = data.expect("Should have data");
                let system_id = data.get("systemId").and_then(|v| v.as_str());
                assert_eq!(system_id, Some("dnd5e"));
            }
            ResponseResult::Error { code, message, .. } => {
                panic!("Expected success, got error: {:?} - {}", code, message);
            }
            _ => panic!("Unexpected result type"),
        }
    }

    server.abort();
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_e2e_llm_health_check() {
    use crate::infrastructure::ports::{ChatMessage, LlmRequest};

    // Simple test to verify Ollama is available
    let client = OllamaClient::from_env();

    // Note: gpt-oss:20b is a reasoning model that needs more tokens to complete
    let request = LlmRequest::new(vec![ChatMessage::user("Say 'hello' in exactly one word.")])
        .with_system_prompt("You are a helpful assistant.")
        .with_temperature(0.0)
        .with_max_tokens(Some(500));

    let response = client.generate(request).await.expect("LLM request failed");

    assert!(!response.content.is_empty(), "Response should not be empty");
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("hello"),
        "Response should contain 'hello': {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_e2e_llm_tool_calling_available() {
    use crate::infrastructure::ports::{ChatMessage, LlmRequest, ToolDefinition};

    // Test that tool calling works with the configured model
    let client = OllamaClient::from_env();

    let tools = vec![ToolDefinition {
        name: "get_weather".to_string(),
        description: "Get the current weather for a location".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name"
                }
            },
            "required": ["location"]
        }),
    }];

    let request = LlmRequest::new(vec![ChatMessage::user("What's the weather in Paris?")])
        .with_system_prompt("You are an assistant that uses tools. When asked about weather, use the get_weather tool.")
        .with_temperature(0.0)
        .with_max_tokens(Some(500));

    let response = client
        .generate_with_tools(request, tools)
        .await
        .expect("LLM request with tools failed");

    // Model should either call the tool or respond textually
    // Both are acceptable - we're just verifying the API works
    assert!(
        !response.tool_calls.is_empty() || !response.content.is_empty(),
        "Response should have either tool calls or content"
    );

    // If tool calls present, verify structure
    if !response.tool_calls.is_empty() {
        let tool_call = &response.tool_calls[0];
        assert_eq!(tool_call.name, "get_weather");
        assert!(tool_call.arguments.is_object());
    }
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_e2e_narrative_generation_quality() {
    use crate::infrastructure::ports::{ChatMessage, LlmRequest};

    // Test that narrative generation produces quality content
    let client = OllamaClient::from_env();

    let system_prompt = r#"You are a Dungeon Master for a fantasy tabletop RPG.
Generate immersive, atmospheric narrative descriptions.
Be descriptive but concise. Focus on what the character sees, hears, and feels."#;

    let request = LlmRequest::new(vec![ChatMessage::user(
        "The party enters a dimly lit tavern on a stormy night. Describe the scene.",
    )])
    .with_system_prompt(system_prompt)
    .with_temperature(0.7)
    .with_max_tokens(Some(800));

    let response = client.generate(request).await.expect("LLM request failed");

    // Verify response quality
    assert!(
        response.content.len() >= 100,
        "Response should be descriptive (got {} chars)",
        response.content.len()
    );

    let content_lower = response.content.to_lowercase();

    // Should include atmospheric elements
    let has_atmosphere = content_lower.contains("tavern")
        || content_lower.contains("inn")
        || content_lower.contains("bar")
        || content_lower.contains("fireplace")
        || content_lower.contains("candle")
        || content_lower.contains("torch");

    assert!(
        has_atmosphere,
        "Should describe the tavern atmosphere: {}",
        response.content
    );

    // Should reference the storm or weather
    let has_weather = content_lower.contains("storm")
        || content_lower.contains("rain")
        || content_lower.contains("thunder")
        || content_lower.contains("wind")
        || content_lower.contains("wet");

    assert!(
        has_weather,
        "Should reference the stormy weather: {}",
        response.content
    );
}

#[tokio::test]
#[ignore = "requires ollama"]
async fn test_e2e_conversation_context_maintained() {
    use crate::infrastructure::ports::{ChatMessage, LlmRequest};

    // Test that conversation context is maintained across messages
    let client = OllamaClient::from_env();

    let system_prompt = "You are a helpful NPC tavern keeper named Gareth.";

    // First message establishes context
    let request1 = LlmRequest::new(vec![ChatMessage::user(
        "What's your name and what do you do here?",
    )])
    .with_system_prompt(system_prompt)
    .with_temperature(0.7)
    .with_max_tokens(Some(500));

    let response1 = client
        .generate(request1)
        .await
        .expect("First LLM request failed");

    // Verify first response mentions name
    let content1_lower = response1.content.to_lowercase();
    assert!(
        content1_lower.contains("gareth")
            || content1_lower.contains("tavern")
            || content1_lower.contains("keeper"),
        "First response should establish identity: {}",
        response1.content
    );

    // Second message with conversation history
    let request2 = LlmRequest::new(vec![
        ChatMessage::user("What's your name and what do you do here?"),
        ChatMessage::assistant(&response1.content),
        ChatMessage::user("I'd like to rent a room for the night."),
    ])
    .with_system_prompt(system_prompt)
    .with_temperature(0.7)
    .with_max_tokens(Some(500));

    let response2 = client
        .generate(request2)
        .await
        .expect("Second LLM request failed");

    // Verify second response is about rooms/lodging
    let content2_lower = response2.content.to_lowercase();
    assert!(
        content2_lower.contains("room")
            || content2_lower.contains("night")
            || content2_lower.contains("stay")
            || content2_lower.contains("gold")
            || content2_lower.contains("coin"),
        "Second response should be about renting a room: {}",
        response2.content
    );
}
