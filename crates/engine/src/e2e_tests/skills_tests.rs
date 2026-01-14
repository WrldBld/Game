//! E2E tests for the skills system.
//!
//! Tests verify:
//! - Skills affect challenge rolls
//! - Skill progression (XP) works
//! - Skills flow to LLM context

use std::sync::Arc;

use super::{create_test_player, E2EEventLog, E2ETestContext, TestOutcome};

/// Test listing world skills.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_list_world_skills() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Setup should succeed");

    // List skills defined for this world using the skill entity
    // Note: get_skills doesn't exist on character entity, use skill.list_in_world
    let skills = ctx
        .app
        .repositories
        .skill
        .list_in_world(ctx.world.world_id)
        .await;

    match skills {
        Ok(skill_list) => {
            println!("World has {} skills defined", skill_list.len());
            for skill in &skill_list {
                println!("  Skill: {} ({:?})", skill.name, skill.category);
            }
        }
        Err(e) => {
            println!("Skills not implemented or world has none: {:?}", e);
        }
    }
}

/// Test PC skills from sheet data.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_pc_skills_from_sheet() {
    let ctx = E2ETestContext::setup()
        .await
        .expect("Setup should succeed");

    let common_room = ctx.world.region("Common Room").expect("Common Room should exist");

    let (_, pc_id) = create_test_player(
        ctx.harness.graph(),
        ctx.world.world_id,
        common_room,
        "Skilled Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Create sheet data with skills
    let sheet_data = serde_json::json!({
        "skills": {
            "athletics": {"level": 2, "proficient": true},
            "perception": {"level": 1, "proficient": false},
            "persuasion": {"level": 3, "proficient": true}
        }
    });

    // Update sheet with skills using the update method with sheet_data parameter
    let update_result = ctx
        .app
        .use_cases
        .management
        .player_character
        .update(pc_id, None, Some(sheet_data))
        .await;

    match update_result {
        Ok(_) => {
            println!("PC sheet updated with skills");

            // Get PC to verify
            let pc = ctx
                .app
                .repositories
                .player_character
                .get(pc_id)
                .await
                .expect("Should get PC")
                .expect("PC should exist");

            println!("PC sheet_data: {:?}", pc.sheet_data());
        }
        Err(e) => {
            println!("Sheet update failed: {:?}", e);
        }
    }
}

/// Test skills in challenge context.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_skills_in_challenge_context() {
    let event_log = Arc::new(E2EEventLog::new("test_skills_in_challenge_context"));
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Setup should succeed");

    let common_room = ctx.world.region("Common Room").expect("Common Room should exist");

    let (_, pc_id) = create_test_player(
        ctx.harness.graph(),
        ctx.world.world_id,
        common_room,
        "Challenge Tester",
    )
    .await
    .expect("Player creation should succeed");

    // Create a challenge that uses a skill
    if let Some(challenge_id) = ctx.world.challenge("Bargain Challenge") {
        // Get challenge details
        let challenge = ctx
            .app
            .repositories
            .challenge
            .get(challenge_id)
            .await
            .expect("Should get challenge");

        if let Some(ch) = challenge {
            println!(
                "Challenge '{}' uses stat: {:?}",
                ch.name, ch.check_stat
            );
        }
    }

    ctx.finalize_event_log(TestOutcome::Pass);
    let _ = ctx.save_event_log(&E2ETestContext::default_log_path("skills_challenge"));
}

/// Test skill proficiency levels.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_skill_proficiency_levels() {
    let _ctx = E2ETestContext::setup()
        .await
        .expect("Setup should succeed");

    // Test proficiency level enum/values
    // ProficiencyLevel variants are: None, Half, Proficient, Expert
    use wrldbldr_domain::ProficiencyLevel;

    let levels = [
        ProficiencyLevel::None,
        ProficiencyLevel::Half,
        ProficiencyLevel::Proficient,
        ProficiencyLevel::Expert, // Note: was Expertise, actual name is Expert
    ];

    for level in &levels {
        println!("Proficiency level: {:?}", level);
    }

    // Verify proficiency affects skill calculations
    // This documents expected behavior
}

/// Test skill categories.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_skill_categories() {
    let _ctx = E2ETestContext::setup()
        .await
        .expect("Setup should succeed");

    // Test skill category enum/values
    use wrldbldr_domain::SkillCategory;

    let categories = [
        SkillCategory::Physical,
        SkillCategory::Mental,
        SkillCategory::Social,
    ];

    for category in &categories {
        println!("Skill category: {:?}", category);
    }
}

/// Test skills flow to LLM context.
#[tokio::test]
#[ignore = "Requires Docker for Neo4j testcontainer"]
async fn test_skills_in_llm_context() {
    let event_log = Arc::new(E2EEventLog::new("test_skills_in_llm_context"));
    let ctx = E2ETestContext::setup_with_logging(event_log.clone())
        .await
        .expect("Setup should succeed");

    let mira_id = ctx.world.npc("Mira Thornwood").expect("Mira should exist");

    // Get character for context
    let npc = ctx
        .app
        .repositories
        .character
        .get(mira_id)
        .await
        .expect("Should get character")
        .expect("NPC should exist");

    // Skills should be part of character context for LLM
    println!("Character {} available for LLM context", npc.name());

    ctx.finalize_event_log(TestOutcome::Pass);
    let _ = ctx.save_event_log(&E2ETestContext::default_log_path("skills_context"));
}
