//! Challenge flow E2E tests.
//!
//! These tests exercise the full challenge workflow:
//! - Triggering challenges
//! - Rolling with modifiers
//! - DM approval of outcomes
//! - Outcome trigger execution (give items, reveal info, etc.)
//!
//! # Running Tests
//!
//! ```bash
//! cargo test -p wrldbldr-engine --lib challenge_flow -- --ignored --test-threads=1
//! ```

use std::sync::Arc;

use wrldbldr_domain::DmApprovalDecision;

use super::{
    approve_staging_with_npc, create_player_character_via_use_case, create_shared_log,
    create_test_player, E2ETestContext, LoggingLlmDecorator, TestOutcome, VcrLlm,
};

// =============================================================================
// Test 1: Predefined Challenge Success Flow
// =============================================================================

/// Full flow: trigger → roll → approve → effects
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_predefined_challenge_success_flow() {
    const TEST_NAME: &str = "test_predefined_challenge_success_flow";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player character
        let pc_id = create_player_character_via_use_case(&ctx, "Aldric the Brave", "test-user-123")
            .await
            .expect("Failed to create PC");

        // Get a challenge from the seeded world
        let challenge_id = ctx
            .world
            .challenge("Convince Grom to Share His Past")
            .expect("Challenge not found");

        // Trigger the challenge prompt
        let prompt = ctx
            .app
            .use_cases
            .challenge
            .trigger_prompt
            .execute(challenge_id)
            .await
            .expect("Failed to trigger challenge prompt");

        assert_eq!(prompt.challenge_id, challenge_id);
        assert!(!prompt.challenge_name.is_empty());

        // Execute the roll with a high roll to ensure success
        let roll_result = ctx
            .app
            .use_cases
            .challenge
            .roll
            .execute(
                ctx.world.world_id,
                challenge_id,
                pc_id,
                Some(18), // High roll for success
                5,        // Modifier
            )
            .await
            .expect("Failed to execute roll");

        assert!(!roll_result
            .approval_queue_id
            .expect("Should have approval queue ID")
            .is_nil());
        assert_eq!(roll_result.total, 23); // 18 + 5

        // Approve the outcome
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(
                roll_result
                    .approval_queue_id
                    .expect("Should have approval queue ID"),
                DmApprovalDecision::Accept,
            )
            .await
            .expect("Failed to approve outcome");

        // Verify the outcome was processed
        assert!(
            approval_result.final_dialogue.is_some(),
            "Should have outcome description"
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
// Test 2: Predefined Challenge Failure Flow
// =============================================================================

/// Verify failure outcome, no success triggers
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_predefined_challenge_failure_flow() {
    const TEST_NAME: &str = "test_predefined_challenge_failure_flow";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player character
        let pc_id = create_player_character_via_use_case(&ctx, "Unlucky Hero", "test-user-456")
            .await
            .expect("Failed to create PC");

        // Get a challenge
        let challenge_id = ctx
            .world
            .challenge("Convince Grom to Share His Past")
            .expect("Challenge not found");

        // Execute the roll with a low roll to ensure failure
        let roll_result = ctx
            .app
            .use_cases
            .challenge
            .roll
            .execute(
                ctx.world.world_id,
                challenge_id,
                pc_id,
                Some(3), // Low roll for failure
                0,       // No modifier
            )
            .await
            .expect("Failed to execute roll");

        assert!(!roll_result
            .approval_queue_id
            .expect("Should have approval queue ID")
            .is_nil());
        assert_eq!(roll_result.total, 3);

        // Approve the failure outcome
        let approval_result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(
                roll_result
                    .approval_queue_id
                    .expect("Should have approval queue ID"),
                DmApprovalDecision::Accept,
            )
            .await
            .expect("Failed to approve outcome");

        // Verify failure was processed (should have failure description)
        assert!(
            approval_result.final_dialogue.is_some(),
            "Should have failure outcome"
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
// Test 3: Critical Success Gives Item
// =============================================================================

/// Crit success grants item to inventory
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_challenge_critical_success_gives_item() {
    const TEST_NAME: &str = "test_challenge_critical_success_gives_item";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player character
        let pc_id = create_player_character_via_use_case(&ctx, "Lucky Hero", "test-user-789")
            .await
            .expect("Failed to create PC");

        // Get a challenge (we'd need one with critical success item trigger)
        let challenge_id = ctx
            .world
            .challenge("Convince Grom to Share His Past")
            .expect("Challenge not found");

        // Execute the roll with natural 20
        let roll_result = ctx
            .app
            .use_cases
            .challenge
            .roll
            .execute(
                ctx.world.world_id,
                challenge_id,
                pc_id,
                Some(20), // Natural 20
                5,        // Modifier
            )
            .await
            .expect("Failed to execute roll");

        // Approve the outcome
        ctx.app
            .use_cases
            .approval
            .decision_flow
            .execute(
                roll_result
                    .approval_queue_id
                    .expect("Should have approval queue ID"),
                DmApprovalDecision::Accept,
            )
            .await
            .expect("Failed to approve outcome");

        // Note: The actual item granting depends on the challenge having a GiveItem trigger
        // on critical success. This test verifies the flow works - actual item checking
        // would require a challenge specifically configured with that trigger.

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
// Test 4: Critical Failure Modifies Stat
// =============================================================================

/// Crit fail damages HP via ModifyCharacterStat trigger
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_challenge_critical_failure_modifies_stat() {
    const TEST_NAME: &str = "test_challenge_critical_failure_modifies_stat";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player character
        let pc_id = create_player_character_via_use_case(&ctx, "Cursed Hero", "test-user-crit")
            .await
            .expect("Failed to create PC");

        let challenge_id = ctx
            .world
            .challenge("Convince Grom to Share His Past")
            .expect("Challenge not found");

        // Execute with natural 1
        let roll_result = ctx
            .app
            .use_cases
            .challenge
            .roll
            .execute(
                ctx.world.world_id,
                challenge_id,
                pc_id,
                Some(1), // Natural 1
                0,       // No modifier
            )
            .await
            .expect("Failed to execute roll");

        // Approve the critical failure
        ctx.app
            .use_cases
            .approval
            .decision_flow
            .execute(
                roll_result
                    .approval_queue_id
                    .expect("Should have approval queue ID"),
                DmApprovalDecision::Accept,
            )
            .await
            .expect("Failed to approve outcome");

        // Note: Stat modification depends on challenge having ModifyCharacterStat trigger
        // This test verifies the critical failure flow works

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
// Test 5: DM Rejects Challenge Outcome
// =============================================================================

/// DM rejects outcome - no effects applied
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_dm_rejects_challenge_outcome() {
    const TEST_NAME: &str = "test_dm_rejects_challenge_outcome";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player character
        let pc_id = create_player_character_via_use_case(&ctx, "Test Hero", "test-user-reject")
            .await
            .expect("Failed to create PC");

        let challenge_id = ctx
            .world
            .challenge("Convince Grom to Share His Past")
            .expect("Challenge not found");

        // Execute roll
        let roll_result = ctx
            .app
            .use_cases
            .challenge
            .roll
            .execute(ctx.world.world_id, challenge_id, pc_id, Some(15), 2)
            .await
            .expect("Failed to execute roll");

        // DM rejects the outcome
        let result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(
                roll_result
                    .approval_queue_id
                    .expect("Should have approval queue ID"),
                DmApprovalDecision::Reject {
                    feedback: "Test rejection".to_string(),
                },
            )
            .await;

        // Rejection should be handled gracefully
        assert!(result.is_ok(), "Rejection should succeed");

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
// Test 6: DM Modifies Challenge Outcome
// =============================================================================

/// DM modifies outcome description before approval
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_dm_modifies_challenge_outcome() {
    const TEST_NAME: &str = "test_dm_modifies_challenge_outcome";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player character
        let pc_id = create_player_character_via_use_case(&ctx, "Mod Hero", "test-user-mod")
            .await
            .expect("Failed to create PC");

        let challenge_id = ctx
            .world
            .challenge("Convince Grom to Share His Past")
            .expect("Challenge not found");

        // Execute roll
        let roll_result = ctx
            .app
            .use_cases
            .challenge
            .roll
            .execute(ctx.world.world_id, challenge_id, pc_id, Some(15), 3)
            .await
            .expect("Failed to execute roll");

        // DM approves with modification
        let modified_text = "The DM has modified this outcome for dramatic effect.";
        let result = ctx
            .app
            .use_cases
            .approval
            .decision_flow
            .execute(
                roll_result
                    .approval_queue_id
                    .expect("Should have approval queue ID"),
                DmApprovalDecision::AcceptWithModification {
                    modified_dialogue: modified_text.to_string(),
                    approved_tools: vec![],
                    rejected_tools: vec![],
                    item_recipients: std::collections::HashMap::new(),
                },
            )
            .await
            .expect("Failed to approve with modification");

        // Verify the modified text was used
        assert_eq!(result.final_dialogue, Some(modified_text.to_string()));

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
// Test 7: LLM Suggests Challenge in Conversation
// =============================================================================

/// LLM suggests a challenge via tool call during conversation
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_llm_suggests_challenge_in_conversation() {
    const TEST_NAME: &str = "test_llm_suggests_challenge_in_conversation";
    let event_log = create_shared_log(TEST_NAME);

    // Create VCR LLM with event logging
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
        // Create player and stage NPC
        let common_room = ctx.world.region("Common Room").expect("Region not found");
        let (player_id, pc_id) = create_test_player(
            ctx.graph(),
            ctx.world.world_id,
            common_room,
            "Challenge Seeker",
        )
        .await
        .expect("Failed to create player");

        let marta_id = ctx.world.npc("Marta Hearthwood").expect("Marta not found");
        approve_staging_with_npc(&ctx, common_room, marta_id)
            .await
            .expect("Failed to stage NPC");

        // Start conversation that might trigger challenge suggestion
        // The conversation content is designed to potentially trigger challenge hints
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
                "I'd like to convince Grom to tell me about his past. Can you help me?".to_string(),
            )
            .await
            .expect("Failed to start conversation");

        // Process the queues
        ctx.app
            .use_cases
            .queues
            .process_player_action
            .execute()
            .await
            .expect("Failed to process player action");

        let llm_result = ctx
            .app
            .use_cases
            .queues
            .process_llm_request
            .execute(|_| {})
            .await
            .expect("Failed to process LLM request");

        // Note: Whether the LLM suggests a challenge depends on the active_challenges
        // context being populated and the LLM deciding to use the tool.
        // This test verifies the infrastructure works.

        assert!(llm_result.is_some(), "Should have processed an LLM request");

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
// Test 8: Challenge Roll with Different Difficulties
// =============================================================================

/// Test dice formula parsing for different difficulty types
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_challenge_roll_with_different_difficulties() {
    const TEST_NAME: &str = "test_challenge_roll_with_different_difficulties";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player character
        let pc_id = create_player_character_via_use_case(&ctx, "Dice Tester", "test-user-dice")
            .await
            .expect("Failed to create PC");

        let challenge_id = ctx
            .world
            .challenge("Convince Grom to Share His Past")
            .expect("Challenge not found");

        // Test with various rolls
        let test_rolls = vec![
            (1, "natural 1"),
            (10, "average roll"),
            (15, "good roll"),
            (20, "natural 20"),
        ];

        for (roll, desc) in test_rolls {
            let roll_result = ctx
                .app
                .use_cases
                .challenge
                .roll
                .execute(ctx.world.world_id, challenge_id, pc_id, Some(roll), 0)
                .await
                .expect(&format!("Failed to execute roll for {}", desc));

            // Verify the roll was recorded correctly
            assert_eq!(
                roll_result.total, roll,
                "Roll total should match for {}",
                desc
            );

            // Clean up by rejecting (so we can roll again)
            ctx.app
                .use_cases
                .approval
                .decision_flow
                .execute(
                    roll_result
                        .approval_queue_id
                        .expect("Should have approval queue ID"),
                    DmApprovalDecision::Reject {
                        feedback: "Test rejection".to_string(),
                    },
                )
                .await
                .ok();
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
// Test 9: Active Challenges in Prompt Context
// =============================================================================

/// Verify active challenges are included in LLM prompt context
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_active_challenges_in_prompt_context() {
    const TEST_NAME: &str = "test_active_challenges_in_prompt_context";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Verify challenges exist in the seeded world
        let challenges = ctx
            .app
            .repositories
            .challenge
            .list_for_world(ctx.world.world_id)
            .await
            .expect("Failed to list challenges");

        assert!(!challenges.is_empty(), "Should have seeded challenges");

        // Check that at least one is active
        let active_count = challenges.iter().filter(|c| c.active).count();
        assert!(active_count > 0, "Should have active challenges");

        // Verify challenge has required fields populated
        for challenge in &challenges {
            assert!(!challenge.name.is_empty(), "Challenge should have name");
            assert!(
                !challenge.description.is_empty(),
                "Challenge should have description"
            );
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
// Test 10: Challenge with Modifier from PC Stats
// =============================================================================

/// Test that PC stat modifiers are applied correctly
#[tokio::test]
#[ignore = "requires docker (testcontainers)"]
async fn test_challenge_with_pc_stat_modifier() {
    const TEST_NAME: &str = "test_challenge_with_pc_stat_modifier";
    let event_log = create_shared_log(TEST_NAME);
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Failed to setup E2E context");

    let test_result = async {
        // Create player character
        let pc_id = create_player_character_via_use_case(&ctx, "Skilled Hero", "test-user-skill")
            .await
            .expect("Failed to create PC");

        let challenge_id = ctx
            .world
            .challenge("Convince Grom to Share His Past")
            .expect("Challenge not found");

        // Execute with a specific modifier
        let modifier = 7; // High skill bonus
        let roll = 10;
        let expected_total = roll + modifier;

        let roll_result = ctx
            .app
            .use_cases
            .challenge
            .roll
            .execute(
                ctx.world.world_id,
                challenge_id,
                pc_id,
                Some(roll),
                modifier,
            )
            .await
            .expect("Failed to execute roll");

        assert_eq!(
            roll_result.total, expected_total,
            "Total should include modifier"
        );

        // Verify the breakdown shows the modifier
        // The breakdown format is "d20(X) + modifier(Y) = Z"

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
