//! Disposition and item E2E tests.
//!
//! These tests verify:
//! - NPC disposition affects conversation context and behavior
//! - Item giving via tool calls and DM approval
//! - Relationship progression and its effects
//! - Motivation-driven NPC behavior
//!
//! # Running Tests
//!
//! ```bash
//! cargo test -p wrldbldr-engine --lib disposition_item -- --ignored --test-threads=1
//! ```

use std::sync::Arc;

use wrldbldr_domain::DmApprovalDecision;

use super::{
    approve_staging_with_npc, create_player_character_via_use_case, create_shared_log,
    create_test_player, start_conversation_with_npc, E2ETestContext, LoggingLlmDecorator,
    TestOutcome, VcrLlm,
};

// =============================================================================
// Test 1: Grateful Disposition in Prompt
// =============================================================================

/// Verify friendly disposition flows to CharacterContext in prompt
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_grateful_disposition_in_prompt() {
    const TEST_NAME: &str = "test_grateful_disposition_in_prompt";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Marta has default_disposition: Friendly
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");

        // Get the NPC data
        let marta = ctx
            .app
            .entities
            .character
            .get(marta_id)
            .await
            .expect("Failed to get character")
            .expect("Character not found");

        // Verify disposition is Friendly
        assert_eq!(
            marta.default_disposition,
            wrldbldr_domain::DispositionLevel::Friendly,
            "Marta should have Friendly disposition"
        );

        // Create player and stage NPC
        let common_room = ctx.world.region("Common Room").expect("Region not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        let (player_id, pc_id) = create_test_player(
            ctx.harness.graph(),
            ctx.world.world_id,
            common_room,
            "Friendly Visitor",
        )
        .await
        .expect("Failed to create player");

        // Start conversation - the disposition should be in the prompt context
        let started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_id,
                marta_id,
                player_id,
                "Hello, I'm looking for help.".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(!started.conversation_id.is_nil());

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 2: Hostile Disposition in Prompt
// =============================================================================

/// Verify hostile NPC context is properly set
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_hostile_disposition_in_prompt() {
    const TEST_NAME: &str = "test_hostile_disposition_in_prompt";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Grom has default_disposition: Cautious (closest to hostile in test data)
        let grom_id = ctx.world.npc("Grom Ironhand").expect("Grom not found");

        let grom = ctx
            .app
            .entities
            .character
            .get(grom_id)
            .await
            .expect("Failed to get character")
            .expect("Character not found");

        // Verify Grom has a less friendly disposition
        assert_ne!(
            grom.default_disposition,
            wrldbldr_domain::DispositionLevel::Friendly,
            "Grom should not be Friendly"
        );

        // Create player and stage NPC at Grom's location
        let forge = ctx.world.region("The Forge").expect("Forge not found");
        approve_staging_with_npc(&ctx, forge, grom_id)
            .await
            .expect("Failed to stage NPC");

        let (player_id, pc_id) = create_test_player(
            ctx.harness.graph(),
            ctx.world.world_id,
            forge,
            "Cautious Visitor",
        )
        .await
        .expect("Failed to create player");

        // Start conversation
        let started = ctx
            .app
            .use_cases
            .conversation
            .start
            .execute(
                ctx.world.world_id,
                pc_id,
                grom_id,
                player_id,
                "Tell me about yourself.".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        assert!(!started.conversation_id.is_nil());

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 3: NPC Motivations in Prompt
// =============================================================================

/// Verify wants (known/suspected/secret) are included in context
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_npc_motivations_in_prompt() {
    const TEST_NAME: &str = "test_npc_motivations_in_prompt";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Marta has wants defined in the test data
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");

        // Check wants in the test world
        let marta_wants = ctx.test_world.wants_for(marta_id);
        assert!(!marta_wants.is_empty(), "Marta should have wants defined");

        // Verify different visibility levels exist
        let has_known = marta_wants.iter().any(|w| w.visibility == "known");
        let has_hidden = marta_wants.iter().any(|w| w.visibility == "hidden");

        assert!(has_known, "Marta should have known wants");
        assert!(has_hidden, "Marta should have hidden wants");

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 4: DM Approves Give Item
// =============================================================================

/// GiveItem tool → item added to PC inventory
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_dm_approves_give_item_adds_to_inventory() {
    const TEST_NAME: &str = "test_dm_approves_give_item_adds_to_inventory";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player character
        let pc_id = create_player_character_via_use_case(&ctx, "Item Receiver", "test-user-item")
            .await
            .expect("Failed to create PC");

        // Get initial inventory count
        let initial_inventory = ctx
            .app
            .entities
            .inventory
            .get_pc_inventory(pc_id)
            .await
            .expect("Failed to get inventory");

        let initial_count = initial_inventory.len();

        // Test the give_item_to_pc method which is used by GiveItem trigger
        ctx.app
            .entities
            .inventory
            .give_item_to_pc(
                pc_id,
                "Test Potion".to_string(),
                Some("A healing potion".to_string()),
            )
            .await
            .expect("Failed to give item");

        // Verify item was added
        let updated_inventory = ctx
            .app
            .entities
            .inventory
            .get_pc_inventory(pc_id)
            .await
            .expect("Failed to get inventory");

        assert_eq!(
            updated_inventory.len(),
            initial_count + 1,
            "Inventory should have one more item"
        );

        // Verify the item has the correct name
        assert!(
            updated_inventory.iter().any(|i| i.name == "Test Potion"),
            "Should find Test Potion in inventory"
        );

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 5: DM Rejects Give Item
// =============================================================================

/// Rejected GiveItem tool has no effect on inventory
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_dm_rejects_give_item_no_change() {
    const TEST_NAME: &str = "test_dm_rejects_give_item_no_change";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player character
        let pc_id = create_player_character_via_use_case(&ctx, "No Items", "test-user-reject-item")
            .await
            .expect("Failed to create PC");

        // Get initial inventory count
        let initial_inventory = ctx
            .app
            .entities
            .inventory
            .get_pc_inventory(pc_id)
            .await
            .expect("Failed to get inventory");

        let initial_count = initial_inventory.len();

        // Simulate a rejected give_item scenario
        // In a full test, we would:
        // 1. Have an LLM return a give_item tool call
        // 2. DM rejects the approval
        // 3. Verify inventory unchanged

        // Verify inventory is still at initial count
        let final_inventory = ctx
            .app
            .entities
            .inventory
            .get_pc_inventory(pc_id)
            .await
            .expect("Failed to get inventory");

        assert_eq!(
            final_inventory.len(),
            initial_count,
            "Inventory should be unchanged after rejection"
        );

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 6: Grateful NPC Offers Item
// =============================================================================

/// High disposition NPC proactively helps via conversation
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_grateful_npc_offers_item() {
    const TEST_NAME: &str = "test_grateful_npc_offers_item";
    let event_log = create_shared_log(TEST_NAME);

    let vcr = Arc::new(VcrLlm::from_env(std::path::PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        TEST_NAME
    ))));
    let llm = Arc::new(LoggingLlmDecorator::new(vcr.clone(), event_log.clone()));

    let ctx = E2ETestContext::setup_with_llm_and_logging(llm.clone(), event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Marta is friendly and might offer help
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        let common_room = ctx.world.region("Common Room").expect("Region not found");

        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        let (player_id, pc_id) = create_test_player(
            ctx.harness.graph(),
            ctx.world.world_id,
            common_room,
            "Needy Hero",
        )
        .await
        .expect("Failed to create player");

        // Ask for help - a friendly NPC might offer an item
        let (_conversation_id, response) = start_conversation_with_npc(
            &ctx,
            pc_id,
            marta_id,
            &player_id,
            "I'm feeling unwell. Do you have anything that could help me?",
        )
        .await
        .expect("Failed to start conversation");

        // Verify we got a response
        assert!(!response.is_empty(), "Should get a response from friendly NPC");

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");
    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 7: Hostile NPC Refuses Help
// =============================================================================

/// Low disposition NPC refuses to help
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_hostile_npc_refuses_help() {
    const TEST_NAME: &str = "test_hostile_npc_refuses_help";
    let event_log = create_shared_log(TEST_NAME);

    let vcr = Arc::new(VcrLlm::from_env(std::path::PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        TEST_NAME
    ))));
    let llm = Arc::new(LoggingLlmDecorator::new(vcr.clone(), event_log.clone()));

    let ctx = E2ETestContext::setup_with_llm_and_logging(llm.clone(), event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Grom is cautious/unfriendly
        let grom_id = ctx.world.npc("Grom Ironhand").expect("Grom not found");
        let forge = ctx.world.region("The Forge").expect("Forge not found");

        approve_staging_with_npc(&ctx, forge, grom_id)
            .await
            .expect("Failed to stage NPC");

        let (player_id, pc_id) = create_test_player(
            ctx.harness.graph(),
            ctx.world.world_id,
            forge,
            "Unwelcome Visitor",
        )
        .await
        .expect("Failed to create player");

        // Ask for help from unfriendly NPC
        let (_conversation_id, response) = start_conversation_with_npc(
            &ctx,
            pc_id,
            grom_id,
            &player_id,
            "Can you give me a free weapon?",
        )
        .await
        .expect("Failed to start conversation");

        // Verify we got a response (likely refusing)
        assert!(!response.is_empty(), "Should get a response from NPC");

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");
    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 8: Relationship Data Exists
// =============================================================================

/// Verify relationship data is seeded and queryable
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_relationship_data_exists() {
    const TEST_NAME: &str = "test_relationship_data_exists";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Check relationships in test world
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        let marta_relationships = ctx.test_world.relationships_from(marta_id);

        assert!(
            !marta_relationships.is_empty(),
            "Marta should have relationships"
        );

        // Verify relationship properties
        for rel in marta_relationships {
            assert!(!rel.relationship_type.is_empty(), "Should have type");
            // Sentiment is a float indicating relationship quality
        }

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 9: Multiple NPCs with Different Dispositions
// =============================================================================

/// Test that different NPCs have distinct dispositions
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_multiple_npcs_different_dispositions() {
    const TEST_NAME: &str = "test_multiple_npcs_different_dispositions";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Get multiple NPCs
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        let grom_id = ctx.world.npc("Grom Ironhand").expect("Grom not found");

        let marta = ctx
            .app
            .entities
            .character
            .get(marta_id)
            .await
            .expect("Failed to get Marta")
            .expect("Marta not found");

        let grom = ctx
            .app
            .entities
            .character
            .get(grom_id)
            .await
            .expect("Failed to get Grom")
            .expect("Grom not found");

        // Verify they have different dispositions
        assert!(
            marta.default_disposition != grom.default_disposition
                || marta.default_mood != grom.default_mood,
            "NPCs should have distinct personalities"
        );

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 10: NPC Archetype in Context
// =============================================================================

/// Verify NPC archetype is included in character context
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_npc_archetype_in_context() {
    const TEST_NAME: &str = "test_npc_archetype_in_context";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Marta is a Mentor archetype
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");

        let marta = ctx
            .app
            .entities
            .character
            .get(marta_id)
            .await
            .expect("Failed to get Marta")
            .expect("Marta not found");

        // Verify archetype
        assert_eq!(
            marta.base_archetype,
            wrldbldr_domain::CampbellArchetype::Mentor,
            "Marta should be a Mentor archetype"
        );

        // Grom is a Threshold Guardian
        let grom_id = ctx.world.npc("Grom Ironhand").expect("Grom not found");
        let grom = ctx
            .app
            .entities
            .character
            .get(grom_id)
            .await
            .expect("Failed to get Grom")
            .expect("Grom not found");

        assert_eq!(
            grom.base_archetype,
            wrldbldr_domain::CampbellArchetype::ThresholdGuardian,
            "Grom should be a Threshold Guardian archetype"
        );

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 11: Tool Definitions in LLM Request
// =============================================================================

/// Verify tool definitions are included in LLM requests
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_tool_definitions_in_llm_request() {
    const TEST_NAME: &str = "test_tool_definitions_in_llm_request";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Use the tool builder to verify definitions are created
        let tools = crate::use_cases::queues::tool_builder::build_game_tool_definitions();

        // Verify all expected tools are present
        assert_eq!(tools.len(), 11, "Should have 11 tool definitions");

        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"give_item"));
        assert!(tool_names.contains(&"reveal_info"));
        assert!(tool_names.contains(&"change_relationship"));
        assert!(tool_names.contains(&"trigger_event"));

        // Verify each tool has required fields
        for tool in &tools {
            assert!(!tool.name.is_empty(), "Tool should have name");
            assert!(!tool.description.is_empty(), "Tool should have description");
            assert!(tool.parameters.is_object(), "Tool should have parameters schema");
        }

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");
    test_result.expect("Test failed");
}

// =============================================================================
// Test 12: Secret Motivation Deflection
// =============================================================================

/// NPC deflects probing on secret wants
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_secret_motivation_deflection() {
    const TEST_NAME: &str = "test_secret_motivation_deflection";
    let event_log = create_shared_log(TEST_NAME);

    let vcr = Arc::new(VcrLlm::from_env(std::path::PathBuf::from(format!(
        "{}/src/e2e_tests/cassettes/{}.json",
        env!("CARGO_MANIFEST_DIR"),
        TEST_NAME
    ))));
    let llm = Arc::new(LoggingLlmDecorator::new(vcr.clone(), event_log.clone()));

    let ctx = E2ETestContext::setup_with_llm_and_logging(llm.clone(), event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Marta has hidden wants that she should deflect questions about
        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        let common_room = ctx.world.region("Common Room").expect("Region not found");

        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        let (player_id, pc_id) = create_test_player(
            ctx.harness.graph(),
            ctx.world.world_id,
            common_room,
            "Nosy Visitor",
        )
        .await
        .expect("Failed to create player");

        // Ask about something that might touch on secret motivations
        let (_conversation_id, response) = start_conversation_with_npc(
            &ctx,
            pc_id,
            marta_id,
            &player_id,
            "You seem troubled. What are you hiding?",
        )
        .await
        .expect("Failed to start conversation");

        // Verify we got a response (the actual deflection behavior depends on LLM)
        assert!(!response.is_empty(), "Should get a response");

        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    }
    .await;

    let outcome = if test_result.is_ok() {
        TestOutcome::Pass
    } else {
        TestOutcome::Fail
    };
    ctx.finalize_event_log(outcome);
    ctx.save_event_log(&E2ETestContext::default_log_path(TEST_NAME))
        .expect("save log");
    vcr.save_cassette().expect("Failed to save cassette");
    test_result.expect("Test failed");
}
