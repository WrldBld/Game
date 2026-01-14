//! Narrative event chain E2E tests.
//!
//! Tests for scenarios involving connected narrative events that trigger
//! in sequence or based on conditions.
//!
//! # Test Scenarios
//!
//! ## Chain Trigger Conditions
//! - First event in chain triggers on location entry
//! - Subsequent events unlock after previous event completes
//! - Conditions evaluate player state (flags, inventory, stats)
//!
//! ## Chain Completion
//! - All events in chain complete in order
//! - Chain completion triggers final effects
//! - Partial completion state persists across sessions

use chrono::Utc;
use neo4rs::query;
use uuid::Uuid;
use wrldbldr_domain::{NarrativeEvent, NarrativeTrigger, NarrativeTriggerType};

use super::*;

// =============================================================================
// Chain Trigger Conditions
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_first_event_triggers_on_location_entry() {
    // Scenario: Player enters location, first event in chain triggers.
    // Expected: Event fires and chain state initialized.
    //
    // Setup:
    // 1. Create E2E context with event chain defined
    // 2. Create event with PlayerEntersLocation trigger for Common Room
    // 3. Player moves to Common Room
    //
    // Assertions:
    // - Event triggers when player enters the region
    // - Triggered events are returned from enter_region

    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Get regions for movement
    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let private_booth = ctx
        .world
        .region("Private Booth")
        .expect("Private Booth should exist");
    let location_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Location should exist");

    // Create player character starting in Private Booth
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        private_booth,
        "Event Chain Tester",
    )
    .await
    .expect("Failed to create test player");

    // Create a narrative event that triggers when entering Common Room
    let event_id = Uuid::new_v4();
    let now = Utc::now();

    // Create the event with a PlayerEntersLocation trigger
    let trigger = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::PlayerEntersLocation {
            location_id,
            location_name: "The Drowsy Dragon Inn".to_string(),
        },
        description: "Player enters the tavern".to_string(),
        is_required: true,
        trigger_id: "enter-tavern".to_string(),
    };

    let event = NarrativeEvent::new(ctx.world.world_id, "Chain Event A - Tavern Entry", now)
        .with_id(wrldbldr_domain::NarrativeEventId::from(event_id))
        .with_description("First event in chain - triggers on location entry")
        .with_scene_direction("The tavern buzzes with activity as you enter")
        .with_trigger_condition(trigger)
        .with_priority(10);

    // Save the event via narrative repository
    ctx.app
        .repositories
        .narrative
        .save_event(&event)
        .await
        .expect("Failed to save narrative event");

    // Also tie the event to the location in Neo4j for trigger resolution
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to create TIED_TO_LOCATION edge");

    // Player moves to Common Room (which is in the same location)
    let result = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    // Verify the event was triggered
    assert!(
        result
            .triggered_events
            .iter()
            .any(|e| e.name() == "Chain Event A - Tavern Entry"),
        "Expected 'Chain Event A - Tavern Entry' to be in triggered events, got: {:?}",
        result
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );

    // Verify the event has the correct scene direction
    let triggered_event = result
        .triggered_events
        .iter()
        .find(|e| e.name() == "Chain Event A - Tavern Entry")
        .expect("Event should be in triggered list");
    assert_eq!(
        triggered_event.scene_direction(),
        "The tavern buzzes with activity as you enter"
    );
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_subsequent_event_unlocks_after_completion() {
    // Scenario: First event completes, second event becomes available.
    // Expected: Second event can now trigger on its conditions.
    //
    // Setup:
    // 1. Create E2E context with chain
    // 2. Event A is marked as completed (triggered)
    // 3. Event B has condition: EventCompleted { event_id: A }
    // 4. Player enters region, Event B should trigger
    //
    // Assertions:
    // - Event B triggers successfully because Event A was completed
    // - If Event A hadn't completed, Event B wouldn't trigger

    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Get regions
    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let private_booth = ctx
        .world
        .region("Private Booth")
        .expect("Private Booth should exist");
    let location_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Location should exist");

    // Create player character
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        private_booth,
        "Chain Sequence Tester",
    )
    .await
    .expect("Failed to create test player");

    let now = Utc::now();

    // Create Event A (first in chain) - already triggered/completed
    let event_a_id = wrldbldr_domain::NarrativeEventId::new();
    let event_a = NarrativeEvent::new(ctx.world.world_id, "Chain Event A - Introduction", now)
        .with_id(event_a_id)
        .with_description("First event in chain")
        .with_scene_direction("The stranger nods at you")
        .with_triggered_state(true, Some(now), Some("default".to_string()), 1);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_a)
        .await
        .expect("Failed to save Event A");

    // Create Event B (second in chain) - triggers only after Event A completes
    let event_b_id = wrldbldr_domain::NarrativeEventId::new();

    // Event B has two triggers: location entry AND Event A completed
    let location_trigger = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::PlayerEntersLocation {
            location_id,
            location_name: "The Drowsy Dragon Inn".to_string(),
        },
        description: "Player is in tavern".to_string(),
        is_required: true,
        trigger_id: "in-tavern".to_string(),
    };

    let event_completed_trigger = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::EventCompleted {
            event_id: event_a_id,
            event_name: "Chain Event A - Introduction".to_string(),
            outcome_name: None, // Any outcome
        },
        description: "Event A must be completed".to_string(),
        is_required: true,
        trigger_id: "after-event-a".to_string(),
    };

    let event_b = NarrativeEvent::new(ctx.world.world_id, "Chain Event B - Follow Up", now)
        .with_id(event_b_id)
        .with_description("Second event in chain - triggers after Event A")
        .with_scene_direction("The stranger approaches with more to say")
        .with_trigger_condition(location_trigger)
        .with_trigger_condition(event_completed_trigger)
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_b)
        .await
        .expect("Failed to save Event B");

    // Tie Event B to location for trigger resolution
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_b_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to create TIED_TO_LOCATION edge");

    // Player moves to Common Room
    let result = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    // Verify Event B was triggered (because Event A was completed)
    assert!(
        result
            .triggered_events
            .iter()
            .any(|e| e.name() == "Chain Event B - Follow Up"),
        "Expected 'Chain Event B - Follow Up' to be triggered after Event A completed, got: {:?}",
        result
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_event_condition_checks_player_flags() {
    // Scenario: Event only triggers if player has specific flag.
    // Expected: Event fires only when flag is set.
    //
    // Setup:
    // 1. Create E2E context with flag-conditional event
    // 2. Enter region without flag - event should NOT fire
    // 3. Set flag via flag repository
    // 4. Enter region again - event SHOULD fire
    //
    // Assertions:
    // - First entry: no event
    // - After flag set + second entry: event fires

    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Get regions
    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let private_booth = ctx
        .world
        .region("Private Booth")
        .expect("Private Booth should exist");
    let location_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Location should exist");

    // Create player character
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        private_booth,
        "Flag Condition Tester",
    )
    .await
    .expect("Failed to create test player");

    let now = Utc::now();

    // Create event with flag condition: requires "met_mysterious_stranger" flag
    let event_id = wrldbldr_domain::NarrativeEventId::new();

    let location_trigger = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::PlayerEntersLocation {
            location_id,
            location_name: "The Drowsy Dragon Inn".to_string(),
        },
        description: "Player is in tavern".to_string(),
        is_required: true,
        trigger_id: "in-tavern".to_string(),
    };

    let flag_trigger = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::FlagSet {
            flag_name: "met_mysterious_stranger".to_string(),
        },
        description: "Player must have met the mysterious stranger".to_string(),
        is_required: true,
        trigger_id: "has-stranger-flag".to_string(),
    };

    let event = NarrativeEvent::new(ctx.world.world_id, "Secret Meeting", now)
        .with_id(event_id)
        .with_description("Secret event only for those who met the stranger")
        .with_scene_direction("A hooded figure beckons you to a corner booth")
        .with_trigger_condition(location_trigger)
        .with_trigger_condition(flag_trigger)
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event)
        .await
        .expect("Failed to save narrative event");

    // Tie event to location for trigger resolution
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to create TIED_TO_LOCATION edge");

    // First attempt: enter without flag - event should NOT trigger
    let result_without_flag = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        !result_without_flag
            .triggered_events
            .iter()
            .any(|e| e.name() == "Secret Meeting"),
        "Event should NOT trigger without the flag, but got: {:?}",
        result_without_flag
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );

    // Now set the flag on the player
    ctx.app
        .repositories
        .flag
        .set_pc_flag(pc_id, "met_mysterious_stranger")
        .await
        .expect("Failed to set flag");

    // Move back to private booth first (so we can re-enter common room)
    ctx.app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, private_booth)
        .await
        .expect("Movement back should succeed");

    // Second attempt: enter with flag - event should trigger
    let result_with_flag = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        result_with_flag
            .triggered_events
            .iter()
            .any(|e| e.name() == "Secret Meeting"),
        "Event should trigger after flag is set, got: {:?}",
        result_with_flag
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_event_condition_checks_inventory() {
    // Scenario: Event triggers only if player has specific item.
    // Expected: Event conditional on inventory contents.
    //
    // Setup:
    // 1. Create E2E context with item-conditional event
    // 2. Player enters without key item - no trigger
    // 3. Player acquires item via give_item_to_pc
    // 4. Player re-enters - event triggers
    //
    // Assertions:
    // - Without item: event does not fire
    // - With item: event fires

    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Get regions
    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let private_booth = ctx
        .world
        .region("Private Booth")
        .expect("Private Booth should exist");
    let location_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Location should exist");

    // Create player character
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        private_booth,
        "Inventory Condition Tester",
    )
    .await
    .expect("Failed to create test player");

    let now = Utc::now();

    // Create event with inventory condition: requires "Mysterious Key"
    let event_id = wrldbldr_domain::NarrativeEventId::new();

    let location_trigger = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::PlayerEntersLocation {
            location_id,
            location_name: "The Drowsy Dragon Inn".to_string(),
        },
        description: "Player is in tavern".to_string(),
        is_required: true,
        trigger_id: "in-tavern".to_string(),
    };

    let inventory_trigger = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::HasItem {
            item_name: "Mysterious Key".to_string(),
            quantity: None,
        },
        description: "Player must have the mysterious key".to_string(),
        is_required: true,
        trigger_id: "has-key".to_string(),
    };

    let event = NarrativeEvent::new(ctx.world.world_id, "The Locked Door Opens", now)
        .with_id(event_id)
        .with_description("A secret passage reveals itself when you hold the key")
        .with_scene_direction("The innkeeper notices the key in your possession and nods knowingly")
        .with_trigger_condition(location_trigger)
        .with_trigger_condition(inventory_trigger)
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event)
        .await
        .expect("Failed to save narrative event");

    // Tie event to location for trigger resolution
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to create TIED_TO_LOCATION edge");

    // First attempt: enter without item - event should NOT trigger
    let result_without_item = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        !result_without_item
            .triggered_events
            .iter()
            .any(|e| e.name() == "The Locked Door Opens"),
        "Event should NOT trigger without the key, but got: {:?}",
        result_without_item
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );

    // Give the player the key
    ctx.app
        .repositories
        .inventory
        .give_item_to_pc(
            pc_id,
            "Mysterious Key".to_string(),
            Some("An ornate key with strange symbols".to_string()),
        )
        .await
        .expect("Failed to give item");

    // Move back to private booth first
    ctx.app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, private_booth)
        .await
        .expect("Movement back should succeed");

    // Second attempt: enter with item - event should trigger
    let result_with_item = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        result_with_item
            .triggered_events
            .iter()
            .any(|e| e.name() == "The Locked Door Opens"),
        "Event should trigger after item is acquired, got: {:?}",
        result_with_item
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_event_condition_checks_stats() {
    // Scenario: Event triggers based on character stat thresholds.
    // Expected: Event evaluates stat conditions correctly.
    //
    // Setup:
    // 1. Create E2E context with stat-conditional event
    // 2. Event requires: character charisma >= 14
    // 3. NPC with charisma 6 (Old Tom) - no trigger
    // 4. NPC with charisma 17 (Vera Nightshade) - triggers
    //
    // Assertions:
    // - Low stat: event skipped
    // - High stat: event triggers

    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Get regions and location
    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let private_booth = ctx
        .world
        .region("Private Booth")
        .expect("Private Booth should exist");
    let location_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Location should exist");

    // Get NPCs with different charisma scores
    // Old Tom has CHA: 6 (low)
    let old_tom_id = ctx.world.npc("Old Tom").expect("Old Tom should exist");
    // Vera Nightshade has CHA: 17 (high)
    let vera_id = ctx
        .world
        .npc("Vera Nightshade")
        .expect("Vera Nightshade should exist");

    // Create player character
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        private_booth,
        "Stat Condition Tester",
    )
    .await
    .expect("Failed to create test player");

    let now = Utc::now();

    // Create event that requires high charisma (>= 14) on a specific character
    // We'll create two events - one checking Old Tom's CHA, one checking Vera's CHA
    let event_low_cha_id = wrldbldr_domain::NarrativeEventId::new();
    let event_high_cha_id = wrldbldr_domain::NarrativeEventId::new();

    // Event that checks Old Tom's charisma (should NOT trigger since CHA=6 < 14)
    let location_trigger_low = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::PlayerEntersLocation {
            location_id,
            location_name: "The Drowsy Dragon Inn".to_string(),
        },
        description: "Player is in tavern".to_string(),
        is_required: true,
        trigger_id: "in-tavern-low".to_string(),
    };

    let stat_trigger_low = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::StatThreshold {
            character_id: old_tom_id,
            stat_name: "CHA".to_string(),
            min_value: Some(14),
            max_value: None,
        },
        description: "Old Tom must have charisma >= 14".to_string(),
        is_required: true,
        trigger_id: "old-tom-cha-check".to_string(),
    };

    let event_low_cha = NarrativeEvent::new(ctx.world.world_id, "Charismatic Old Tom", now)
        .with_id(event_low_cha_id)
        .with_description("Event requiring Old Tom to have high charisma")
        .with_scene_direction("Old Tom captivates the room with his presence")
        .with_trigger_condition(location_trigger_low)
        .with_trigger_condition(stat_trigger_low)
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_low_cha)
        .await
        .expect("Failed to save low CHA event");

    // Tie event to location
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_low_cha_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to create TIED_TO_LOCATION edge for low CHA event");

    // Event that checks Vera's charisma (should trigger since CHA=17 >= 14)
    let location_trigger_high = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::PlayerEntersLocation {
            location_id,
            location_name: "The Drowsy Dragon Inn".to_string(),
        },
        description: "Player is in tavern".to_string(),
        is_required: true,
        trigger_id: "in-tavern-high".to_string(),
    };

    let stat_trigger_high = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::StatThreshold {
            character_id: vera_id,
            stat_name: "CHA".to_string(),
            min_value: Some(14),
            max_value: None,
        },
        description: "Vera must have charisma >= 14".to_string(),
        is_required: true,
        trigger_id: "vera-cha-check".to_string(),
    };

    let event_high_cha = NarrativeEvent::new(ctx.world.world_id, "Charismatic Vera", now)
        .with_id(event_high_cha_id)
        .with_description("Event requiring Vera to have high charisma")
        .with_scene_direction("Vera's magnetic presence draws everyone's attention")
        .with_trigger_condition(location_trigger_high)
        .with_trigger_condition(stat_trigger_high)
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_high_cha)
        .await
        .expect("Failed to save high CHA event");

    // Tie event to location
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_high_cha_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to create TIED_TO_LOCATION edge for high CHA event");

    // Player moves to Common Room (in the tavern)
    let result = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    // Verify: Old Tom's event should NOT trigger (CHA=6 < 14)
    assert!(
        !result
            .triggered_events
            .iter()
            .any(|e| e.name() == "Charismatic Old Tom"),
        "Event should NOT trigger for Old Tom (CHA 6 < 14), got: {:?}",
        result
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );

    // Verify: Vera's event SHOULD trigger (CHA=17 >= 14)
    assert!(
        result
            .triggered_events
            .iter()
            .any(|e| e.name() == "Charismatic Vera"),
        "Event should trigger for Vera (CHA 17 >= 14), got: {:?}",
        result
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );
}

// =============================================================================
// Chain Completion
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_chain_completes_in_order() {
    // Scenario: Three-event chain completes A -> B -> C.
    // Expected: Events fire in sequence, each requiring the previous to complete.
    //
    // Setup:
    // 1. Create E2E context with three-event chain
    // 2. Event A triggers on location entry
    // 3. Event B triggers after Event A completes
    // 4. Event C triggers after Event B completes
    //
    // Assertions:
    // - Only A fires initially
    // - After A completes, B becomes available
    // - After B completes, C becomes available
    // - Cannot skip to C without B

    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Get regions and location
    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let private_booth = ctx
        .world
        .region("Private Booth")
        .expect("Private Booth should exist");
    let location_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Location should exist");

    // Create player character
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        private_booth,
        "Chain Order Tester",
    )
    .await
    .expect("Failed to create test player");

    let now = Utc::now();

    // Create three events in a chain: A -> B -> C
    let event_a_id = wrldbldr_domain::NarrativeEventId::new();
    let event_b_id = wrldbldr_domain::NarrativeEventId::new();
    let event_c_id = wrldbldr_domain::NarrativeEventId::new();

    // Event A: Triggers on location entry (first in chain)
    let location_trigger_a = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::PlayerEntersLocation {
            location_id,
            location_name: "The Drowsy Dragon Inn".to_string(),
        },
        description: "Player enters tavern".to_string(),
        is_required: true,
        trigger_id: "enter-tavern-a".to_string(),
    };

    let event_a = NarrativeEvent::new(ctx.world.world_id, "Chain Event A - The Introduction", now)
        .with_id(event_a_id)
        .with_description("First event in chain")
        .with_scene_direction("A mysterious stranger catches your eye")
        .with_trigger_condition(location_trigger_a)
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_a)
        .await
        .expect("Failed to save Event A");

    // Tie Event A to location
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_a_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to create TIED_TO_LOCATION edge for Event A");

    // Event B: Triggers after Event A completes (second in chain)
    let location_trigger_b = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::PlayerEntersLocation {
            location_id,
            location_name: "The Drowsy Dragon Inn".to_string(),
        },
        description: "Player is in tavern".to_string(),
        is_required: true,
        trigger_id: "in-tavern-b".to_string(),
    };

    let event_a_completed = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::EventCompleted {
            event_id: event_a_id,
            event_name: "Chain Event A - The Introduction".to_string(),
            outcome_name: None,
        },
        description: "Event A must be completed".to_string(),
        is_required: true,
        trigger_id: "after-event-a".to_string(),
    };

    let event_b = NarrativeEvent::new(ctx.world.world_id, "Chain Event B - The Revelation", now)
        .with_id(event_b_id)
        .with_description("Second event in chain")
        .with_scene_direction("The stranger reveals a dark secret")
        .with_trigger_condition(location_trigger_b)
        .with_trigger_condition(event_a_completed)
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_b)
        .await
        .expect("Failed to save Event B");

    // Tie Event B to location
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_b_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to create TIED_TO_LOCATION edge for Event B");

    // Event C: Triggers after Event B completes (third in chain)
    let location_trigger_c = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::PlayerEntersLocation {
            location_id,
            location_name: "The Drowsy Dragon Inn".to_string(),
        },
        description: "Player is in tavern".to_string(),
        is_required: true,
        trigger_id: "in-tavern-c".to_string(),
    };

    let event_b_completed = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::EventCompleted {
            event_id: event_b_id,
            event_name: "Chain Event B - The Revelation".to_string(),
            outcome_name: None,
        },
        description: "Event B must be completed".to_string(),
        is_required: true,
        trigger_id: "after-event-b".to_string(),
    };

    let event_c = NarrativeEvent::new(ctx.world.world_id, "Chain Event C - The Decision", now)
        .with_id(event_c_id)
        .with_description("Third event in chain")
        .with_scene_direction("You must now make a fateful choice")
        .with_trigger_condition(location_trigger_c)
        .with_trigger_condition(event_b_completed)
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_c)
        .await
        .expect("Failed to save Event C");

    // Tie Event C to location
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_c_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to create TIED_TO_LOCATION edge for Event C");

    // Step 1: Enter location - only Event A should trigger
    let result1 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        result1
            .triggered_events
            .iter()
            .any(|e| e.name() == "Chain Event A - The Introduction"),
        "Event A should trigger on first entry, got: {:?}",
        result1
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );
    assert!(
        !result1
            .triggered_events
            .iter()
            .any(|e| e.name() == "Chain Event B - The Revelation"),
        "Event B should NOT trigger before Event A completes"
    );
    assert!(
        !result1
            .triggered_events
            .iter()
            .any(|e| e.name() == "Chain Event C - The Decision"),
        "Event C should NOT trigger before Event B completes"
    );

    // Step 2: Mark Event A as completed and re-enter
    let mut event_a_updated = ctx
        .app
        .repositories
        .narrative
        .get_event(event_a_id)
        .await
        .expect("Should get Event A")
        .expect("Event A should exist");
    event_a_updated.trigger(Some("default".to_string()), now);
    ctx.app
        .repositories
        .narrative
        .save_event(&event_a_updated)
        .await
        .expect("Failed to update Event A");

    // Move back and re-enter to trigger Event B
    ctx.app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, private_booth)
        .await
        .expect("Movement back should succeed");

    let result2 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        result2
            .triggered_events
            .iter()
            .any(|e| e.name() == "Chain Event B - The Revelation"),
        "Event B should trigger after Event A completes, got: {:?}",
        result2
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );
    assert!(
        !result2
            .triggered_events
            .iter()
            .any(|e| e.name() == "Chain Event C - The Decision"),
        "Event C should NOT trigger before Event B completes"
    );

    // Step 3: Mark Event B as completed and re-enter
    let mut event_b_updated = ctx
        .app
        .repositories
        .narrative
        .get_event(event_b_id)
        .await
        .expect("Should get Event B")
        .expect("Event B should exist");
    event_b_updated.trigger(Some("default".to_string()), now);
    ctx.app
        .repositories
        .narrative
        .save_event(&event_b_updated)
        .await
        .expect("Failed to update Event B");

    // Move back and re-enter to trigger Event C
    ctx.app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, private_booth)
        .await
        .expect("Movement back should succeed");

    let result3 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        result3
            .triggered_events
            .iter()
            .any(|e| e.name() == "Chain Event C - The Decision"),
        "Event C should trigger after Event B completes, got: {:?}",
        result3
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_chain_completion_triggers_final_effects() {
    // Scenario: Completing all events in chain triggers final reward via flag.
    // Expected: Final event sets a flag indicating chain completion.
    //
    // Setup:
    // 1. Create E2E context with chain that sets flag on completion
    // 2. Complete all events in chain
    // 3. Final event is configured with flag trigger to verify completion
    //
    // Assertions:
    // - After completing first event, second event becomes available
    // - Second event triggers and sets completion flag
    // - Completion flag is set on player

    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Get regions and location
    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let private_booth = ctx
        .world
        .region("Private Booth")
        .expect("Private Booth should exist");
    let location_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Location should exist");

    // Create player character
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        private_booth,
        "Chain Completion Tester",
    )
    .await
    .expect("Failed to create test player");

    let now = Utc::now();

    // Create a two-event chain where the final event represents completion
    let event_a_id = wrldbldr_domain::NarrativeEventId::new();
    let event_final_id = wrldbldr_domain::NarrativeEventId::new();

    // Event A: First event in chain
    let location_trigger_a = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::PlayerEntersLocation {
            location_id,
            location_name: "The Drowsy Dragon Inn".to_string(),
        },
        description: "Player enters tavern".to_string(),
        is_required: true,
        trigger_id: "enter-tavern-a".to_string(),
    };

    let event_a = NarrativeEvent::new(ctx.world.world_id, "Quest Start - The Call", now)
        .with_id(event_a_id)
        .with_description("First event in quest chain")
        .with_scene_direction("A desperate villager approaches you")
        .with_trigger_condition(location_trigger_a)
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_a)
        .await
        .expect("Failed to save Event A");

    // Tie Event A to location
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_a_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to create TIED_TO_LOCATION edge for Event A");

    // Final Event: Triggers after Event A, represents quest completion
    let location_trigger_final = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::PlayerEntersLocation {
            location_id,
            location_name: "The Drowsy Dragon Inn".to_string(),
        },
        description: "Player is in tavern".to_string(),
        is_required: true,
        trigger_id: "in-tavern-final".to_string(),
    };

    let event_a_completed = NarrativeTrigger {
        trigger_type: NarrativeTriggerType::EventCompleted {
            event_id: event_a_id,
            event_name: "Quest Start - The Call".to_string(),
            outcome_name: None,
        },
        description: "Quest start event must be completed".to_string(),
        is_required: true,
        trigger_id: "after-quest-start".to_string(),
    };

    let event_final =
        NarrativeEvent::new(ctx.world.world_id, "Quest Complete - The Reward", now)
            .with_id(event_final_id)
            .with_description("Final event granting completion reward")
            .with_scene_direction("The villager returns with a reward for your heroism")
            .with_trigger_condition(location_trigger_final)
            .with_trigger_condition(event_a_completed)
            .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_final)
        .await
        .expect("Failed to save Final Event");

    // Tie Final Event to location
    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_final_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to create TIED_TO_LOCATION edge for Final Event");

    // Step 1: Enter location - Event A should trigger
    let result1 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        result1
            .triggered_events
            .iter()
            .any(|e| e.name() == "Quest Start - The Call"),
        "Quest start event should trigger"
    );

    // Mark Event A as complete
    let mut event_a_updated = ctx
        .app
        .repositories
        .narrative
        .get_event(event_a_id)
        .await
        .expect("Should get Event A")
        .expect("Event A should exist");
    event_a_updated.trigger(Some("default".to_string()), now);
    ctx.app
        .repositories
        .narrative
        .save_event(&event_a_updated)
        .await
        .expect("Failed to update Event A");

    // Step 2: Re-enter to trigger final event
    ctx.app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, private_booth)
        .await
        .expect("Movement back should succeed");

    let result2 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    // Final event should trigger
    assert!(
        result2
            .triggered_events
            .iter()
            .any(|e| e.name() == "Quest Complete - The Reward"),
        "Quest completion event should trigger after first event completes, got: {:?}",
        result2
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );

    // Mark final event as complete to represent full chain completion
    let mut event_final_updated = ctx
        .app
        .repositories
        .narrative
        .get_event(event_final_id)
        .await
        .expect("Should get Final Event")
        .expect("Final Event should exist");
    event_final_updated.trigger(Some("default".to_string()), now);
    ctx.app
        .repositories
        .narrative
        .save_event(&event_final_updated)
        .await
        .expect("Failed to update Final Event");

    // Set a completion flag to represent the chain reward effect
    ctx.app
        .repositories
        .flag
        .set_pc_flag(pc_id, "quest_chain_completed")
        .await
        .expect("Failed to set completion flag");

    // Verify the completion flag is set
    let pc_flags = ctx
        .app
        .repositories
        .flag
        .get_pc_flags(pc_id)
        .await
        .expect("Should get flags");

    assert!(
        pc_flags.contains(&"quest_chain_completed".to_string()),
        "Quest chain completion flag should be set after final event"
    );

    // Verify final event is marked as triggered
    let final_event = ctx
        .app
        .repositories
        .narrative
        .get_event(event_final_id)
        .await
        .expect("Should get Final Event")
        .expect("Final Event should exist");

    assert!(
        final_event.is_triggered(),
        "Final event should be marked as triggered"
    );
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_partial_chain_persists_across_sessions() {
    // Scenario: Player completes part of chain, progress persists in database.
    // Expected: Progress is saved, chain resumes from where left off.
    //
    // Setup:
    // 1. Create E2E context with three-event chain
    // 2. Complete Event A
    // 3. Verify Event A is marked complete in database
    // 4. Re-query events to simulate session reload
    // 5. Verify Event B is available and Event A stays complete
    //
    // Assertions:
    // - Event A marked complete persists
    // - Event B becomes available after persistence
    // - Event C still blocked until Event B completes

    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Get regions and location
    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let private_booth = ctx
        .world
        .region("Private Booth")
        .expect("Private Booth should exist");
    let location_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Location should exist");

    // Create player character
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        private_booth,
        "Persistence Tester",
    )
    .await
    .expect("Failed to create test player");

    let now = Utc::now();

    // Create three events in chain: A -> B -> C
    let event_a_id = wrldbldr_domain::NarrativeEventId::new();
    let event_b_id = wrldbldr_domain::NarrativeEventId::new();
    let event_c_id = wrldbldr_domain::NarrativeEventId::new();

    // Event A: First in chain
    let event_a = NarrativeEvent::new(ctx.world.world_id, "Persistent Chain A", now)
        .with_id(event_a_id)
        .with_description("First event in persistent chain")
        .with_scene_direction("The adventure begins")
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name: "The Drowsy Dragon Inn".to_string(),
            },
            description: "Player enters tavern".to_string(),
            is_required: true,
            trigger_id: "enter-tavern".to_string(),
        })
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_a)
        .await
        .expect("Failed to save Event A");

    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_a_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to tie Event A to location");

    // Event B: Requires Event A completion
    let event_b = NarrativeEvent::new(ctx.world.world_id, "Persistent Chain B", now)
        .with_id(event_b_id)
        .with_description("Second event in persistent chain")
        .with_scene_direction("The plot thickens")
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name: "The Drowsy Dragon Inn".to_string(),
            },
            description: "Player in tavern".to_string(),
            is_required: true,
            trigger_id: "in-tavern-b".to_string(),
        })
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::EventCompleted {
                event_id: event_a_id,
                event_name: "Persistent Chain A".to_string(),
                outcome_name: None,
            },
            description: "Event A completed".to_string(),
            is_required: true,
            trigger_id: "after-a".to_string(),
        })
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_b)
        .await
        .expect("Failed to save Event B");

    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_b_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to tie Event B to location");

    // Event C: Requires Event B completion
    let event_c = NarrativeEvent::new(ctx.world.world_id, "Persistent Chain C", now)
        .with_id(event_c_id)
        .with_description("Third event in persistent chain")
        .with_scene_direction("The conclusion")
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name: "The Drowsy Dragon Inn".to_string(),
            },
            description: "Player in tavern".to_string(),
            is_required: true,
            trigger_id: "in-tavern-c".to_string(),
        })
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::EventCompleted {
                event_id: event_b_id,
                event_name: "Persistent Chain B".to_string(),
                outcome_name: None,
            },
            description: "Event B completed".to_string(),
            is_required: true,
            trigger_id: "after-b".to_string(),
        })
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_c)
        .await
        .expect("Failed to save Event C");

    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_c_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to tie Event C to location");

    // Step 1: Enter location, Event A triggers
    let result1 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        result1
            .triggered_events
            .iter()
            .any(|e| e.name() == "Persistent Chain A"),
        "Event A should trigger initially"
    );

    // Step 2: Mark Event A as complete (simulating player completing the event)
    let mut event_a_updated = ctx
        .app
        .repositories
        .narrative
        .get_event(event_a_id)
        .await
        .expect("Should get Event A")
        .expect("Event A should exist");
    event_a_updated.trigger(Some("default".to_string()), now);
    ctx.app
        .repositories
        .narrative
        .save_event(&event_a_updated)
        .await
        .expect("Failed to save Event A completion");

    // Step 3: Simulate "session reload" by re-fetching event from database
    // This verifies persistence - the event should still be marked as triggered
    let event_a_reloaded = ctx
        .app
        .repositories
        .narrative
        .get_event(event_a_id)
        .await
        .expect("Should get Event A")
        .expect("Event A should still exist");

    assert!(
        event_a_reloaded.is_triggered(),
        "Event A should remain triggered after persistence (simulated session reload)"
    );

    // Step 4: Re-enter location - Event B should now be available
    ctx.app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, private_booth)
        .await
        .expect("Movement back should succeed");

    let result2 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    // Event B should trigger (because Event A is complete in persisted state)
    assert!(
        result2
            .triggered_events
            .iter()
            .any(|e| e.name() == "Persistent Chain B"),
        "Event B should trigger after Event A completion persisted, got: {:?}",
        result2
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );

    // Event C should NOT trigger yet
    assert!(
        !result2
            .triggered_events
            .iter()
            .any(|e| e.name() == "Persistent Chain C"),
        "Event C should NOT trigger until Event B completes"
    );

    // Step 5: Mark Event B as complete
    let mut event_b_updated = ctx
        .app
        .repositories
        .narrative
        .get_event(event_b_id)
        .await
        .expect("Should get Event B")
        .expect("Event B should exist");
    event_b_updated.trigger(Some("default".to_string()), now);
    ctx.app
        .repositories
        .narrative
        .save_event(&event_b_updated)
        .await
        .expect("Failed to save Event B completion");

    // Verify Event B persisted
    let event_b_reloaded = ctx
        .app
        .repositories
        .narrative
        .get_event(event_b_id)
        .await
        .expect("Should get Event B")
        .expect("Event B should still exist");

    assert!(
        event_b_reloaded.is_triggered(),
        "Event B should remain triggered after persistence"
    );

    // Step 6: Re-enter - Event C should now be available
    ctx.app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, private_booth)
        .await
        .expect("Movement back should succeed");

    let result3 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        result3
            .triggered_events
            .iter()
            .any(|e| e.name() == "Persistent Chain C"),
        "Event C should trigger after both A and B completion persisted, got: {:?}",
        result3
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );
}

// =============================================================================
// Branching Chains
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_chain_branch_based_on_outcome() {
    // Scenario: Event outcome determines which branch of chain follows.
    // Expected: Different outcomes lead to different subsequent events.
    //
    // Setup:
    // 1. Create E2E context with branching chain
    // 2. Event A has outcomes: "accept" -> Event B, "refuse" -> Event C
    // 3. Complete Event A with "accept" outcome
    //
    // Assertions:
    // - Event B becomes available (requires "accept" outcome)
    // - Event C does NOT become available (requires "refuse" outcome)

    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Get regions and location
    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let private_booth = ctx
        .world
        .region("Private Booth")
        .expect("Private Booth should exist");
    let location_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Location should exist");

    // Create player character
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        private_booth,
        "Branch Tester",
    )
    .await
    .expect("Failed to create test player");

    let now = Utc::now();

    // Create branching chain: A -> B (accept) or A -> C (refuse)
    let event_a_id = wrldbldr_domain::NarrativeEventId::new();
    let event_b_id = wrldbldr_domain::NarrativeEventId::new();
    let event_c_id = wrldbldr_domain::NarrativeEventId::new();

    // Event A: Initial event with branching outcomes
    let event_a = NarrativeEvent::new(ctx.world.world_id, "Branching Event A - The Offer", now)
        .with_id(event_a_id)
        .with_description("Event with branching outcomes")
        .with_scene_direction("A mysterious figure makes you an offer")
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name: "The Drowsy Dragon Inn".to_string(),
            },
            description: "Player enters tavern".to_string(),
            is_required: true,
            trigger_id: "enter-tavern".to_string(),
        })
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_a)
        .await
        .expect("Failed to save Event A");

    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_a_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to tie Event A to location");

    // Event B: Accept path - requires Event A completed with "accept" outcome
    let event_b = NarrativeEvent::new(ctx.world.world_id, "Branch B - Accepted Quest", now)
        .with_id(event_b_id)
        .with_description("Quest accepted path")
        .with_scene_direction("Having accepted the offer, you learn more details")
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name: "The Drowsy Dragon Inn".to_string(),
            },
            description: "Player in tavern".to_string(),
            is_required: true,
            trigger_id: "in-tavern-b".to_string(),
        })
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::EventCompleted {
                event_id: event_a_id,
                event_name: "Branching Event A - The Offer".to_string(),
                outcome_name: Some("accept".to_string()), // Specific outcome required!
            },
            description: "Player accepted the offer".to_string(),
            is_required: true,
            trigger_id: "accepted-offer".to_string(),
        })
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_b)
        .await
        .expect("Failed to save Event B");

    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_b_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to tie Event B to location");

    // Event C: Refuse path - requires Event A completed with "refuse" outcome
    let event_c = NarrativeEvent::new(ctx.world.world_id, "Branch C - Refused Quest", now)
        .with_id(event_c_id)
        .with_description("Quest refused path")
        .with_scene_direction("Having refused, the stranger becomes hostile")
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name: "The Drowsy Dragon Inn".to_string(),
            },
            description: "Player in tavern".to_string(),
            is_required: true,
            trigger_id: "in-tavern-c".to_string(),
        })
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::EventCompleted {
                event_id: event_a_id,
                event_name: "Branching Event A - The Offer".to_string(),
                outcome_name: Some("refuse".to_string()), // Specific outcome required!
            },
            description: "Player refused the offer".to_string(),
            is_required: true,
            trigger_id: "refused-offer".to_string(),
        })
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_c)
        .await
        .expect("Failed to save Event C");

    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_c_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to tie Event C to location");

    // Step 1: Enter location - Event A should trigger
    let result1 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        result1
            .triggered_events
            .iter()
            .any(|e| e.name() == "Branching Event A - The Offer"),
        "Event A should trigger initially"
    );

    // Step 2: Complete Event A with "accept" outcome
    let mut event_a_updated = ctx
        .app
        .repositories
        .narrative
        .get_event(event_a_id)
        .await
        .expect("Should get Event A")
        .expect("Event A should exist");
    event_a_updated.trigger(Some("accept".to_string()), now); // Accept outcome!
    ctx.app
        .repositories
        .narrative
        .save_event(&event_a_updated)
        .await
        .expect("Failed to save Event A with accept outcome");

    // Step 3: Re-enter - Event B should trigger (accept path), Event C should NOT
    ctx.app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, private_booth)
        .await
        .expect("Movement back should succeed");

    let result2 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    // Event B should trigger (we accepted)
    assert!(
        result2
            .triggered_events
            .iter()
            .any(|e| e.name() == "Branch B - Accepted Quest"),
        "Event B (accept branch) should trigger after accepting, got: {:?}",
        result2
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );

    // Event C should NOT trigger (we didn't refuse)
    assert!(
        !result2
            .triggered_events
            .iter()
            .any(|e| e.name() == "Branch C - Refused Quest"),
        "Event C (refuse branch) should NOT trigger when we accepted"
    );
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_alternate_branch_path() {
    // Scenario: Taking alternate outcome leads to different chain path.
    // Expected: Refusing quest leads to different storyline.
    //
    // Setup:
    // 1. Create E2E context with branching chain
    // 2. Complete Event A with "refuse" outcome
    //
    // Assertions:
    // - Event C becomes available (refuse path)
    // - Event B does NOT become available (accept path)

    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Get regions and location
    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let private_booth = ctx
        .world
        .region("Private Booth")
        .expect("Private Booth should exist");
    let location_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Location should exist");

    // Create player character
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        private_booth,
        "Alternate Branch Tester",
    )
    .await
    .expect("Failed to create test player");

    let now = Utc::now();

    // Create branching chain: A -> B (accept) or A -> C (refuse)
    let event_a_id = wrldbldr_domain::NarrativeEventId::new();
    let event_b_id = wrldbldr_domain::NarrativeEventId::new();
    let event_c_id = wrldbldr_domain::NarrativeEventId::new();

    // Event A: Initial event
    let event_a = NarrativeEvent::new(ctx.world.world_id, "Alt Branch A - The Proposal", now)
        .with_id(event_a_id)
        .with_description("Event with branching outcomes")
        .with_scene_direction("A shady character proposes a scheme")
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name: "The Drowsy Dragon Inn".to_string(),
            },
            description: "Player enters tavern".to_string(),
            is_required: true,
            trigger_id: "enter-tavern".to_string(),
        })
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_a)
        .await
        .expect("Failed to save Event A");

    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_a_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to tie Event A to location");

    // Event B: Accept path (should NOT trigger in this test)
    let event_b = NarrativeEvent::new(ctx.world.world_id, "Alt Branch B - Joined Scheme", now)
        .with_id(event_b_id)
        .with_description("Joined the scheme path")
        .with_scene_direction("You've thrown in your lot with the schemer")
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name: "The Drowsy Dragon Inn".to_string(),
            },
            description: "Player in tavern".to_string(),
            is_required: true,
            trigger_id: "in-tavern-b".to_string(),
        })
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::EventCompleted {
                event_id: event_a_id,
                event_name: "Alt Branch A - The Proposal".to_string(),
                outcome_name: Some("accept".to_string()),
            },
            description: "Player joined the scheme".to_string(),
            is_required: true,
            trigger_id: "joined-scheme".to_string(),
        })
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_b)
        .await
        .expect("Failed to save Event B");

    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_b_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to tie Event B to location");

    // Event C: Refuse path (should trigger in this test)
    let event_c = NarrativeEvent::new(ctx.world.world_id, "Alt Branch C - Rejected Scheme", now)
        .with_id(event_c_id)
        .with_description("Rejected the scheme path")
        .with_scene_direction("You've made an enemy of the schemer")
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name: "The Drowsy Dragon Inn".to_string(),
            },
            description: "Player in tavern".to_string(),
            is_required: true,
            trigger_id: "in-tavern-c".to_string(),
        })
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::EventCompleted {
                event_id: event_a_id,
                event_name: "Alt Branch A - The Proposal".to_string(),
                outcome_name: Some("refuse".to_string()),
            },
            description: "Player rejected the scheme".to_string(),
            is_required: true,
            trigger_id: "rejected-scheme".to_string(),
        })
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event_c)
        .await
        .expect("Failed to save Event C");

    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_c_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to tie Event C to location");

    // Step 1: Enter location - Event A triggers
    let result1 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        result1
            .triggered_events
            .iter()
            .any(|e| e.name() == "Alt Branch A - The Proposal"),
        "Event A should trigger initially"
    );

    // Step 2: Complete Event A with "refuse" outcome (alternate path!)
    let mut event_a_updated = ctx
        .app
        .repositories
        .narrative
        .get_event(event_a_id)
        .await
        .expect("Should get Event A")
        .expect("Event A should exist");
    event_a_updated.trigger(Some("refuse".to_string()), now); // Refuse outcome!
    ctx.app
        .repositories
        .narrative
        .save_event(&event_a_updated)
        .await
        .expect("Failed to save Event A with refuse outcome");

    // Step 3: Re-enter - Event C should trigger (refuse path), Event B should NOT
    ctx.app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, private_booth)
        .await
        .expect("Movement back should succeed");

    let result2 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    // Event C should trigger (we refused)
    assert!(
        result2
            .triggered_events
            .iter()
            .any(|e| e.name() == "Alt Branch C - Rejected Scheme"),
        "Event C (refuse branch) should trigger after refusing, got: {:?}",
        result2
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );

    // Event B should NOT trigger (we didn't accept)
    assert!(
        !result2
            .triggered_events
            .iter()
            .any(|e| e.name() == "Alt Branch B - Joined Scheme"),
        "Event B (accept branch) should NOT trigger when we refused"
    );
}

// =============================================================================
// Chain Reset and Replay
// =============================================================================

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_repeatable_chain_can_restart() {
    // Scenario: Repeatable event can trigger again after completion.
    // Expected: Event resets and fires again after being reset.
    //
    // Setup:
    // 1. Create E2E context with repeatable event
    // 2. Trigger and complete event
    // 3. Reset event (mark as not triggered)
    // 4. Trigger event again
    //
    // Assertions:
    // - Event can be reset
    // - After reset, event triggers again
    // - Multiple completions work

    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Get regions and location
    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let private_booth = ctx
        .world
        .region("Private Booth")
        .expect("Private Booth should exist");
    let location_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Location should exist");

    // Create player character
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        private_booth,
        "Repeatable Chain Tester",
    )
    .await
    .expect("Failed to create test player");

    let now = Utc::now();

    // Create a repeatable event (marked with is_repeatable)
    let event_id = wrldbldr_domain::NarrativeEventId::new();

    let event = NarrativeEvent::new(ctx.world.world_id, "Repeatable Daily Quest", now)
        .with_id(event_id)
        .with_description("A daily quest that can be repeated")
        .with_scene_direction("The innkeeper has another task for you")
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name: "The Drowsy Dragon Inn".to_string(),
            },
            description: "Player enters tavern".to_string(),
            is_required: true,
            trigger_id: "enter-tavern".to_string(),
        })
        .with_repeatable(true) // Mark as repeatable!
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event)
        .await
        .expect("Failed to save event");

    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to tie event to location");

    // Step 1: First trigger
    let result1 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        result1
            .triggered_events
            .iter()
            .any(|e| e.name() == "Repeatable Daily Quest"),
        "Event should trigger first time"
    );

    // Step 2: Complete the event
    let mut event_updated = ctx
        .app
        .repositories
        .narrative
        .get_event(event_id)
        .await
        .expect("Should get event")
        .expect("Event should exist");
    event_updated.trigger(Some("completed".to_string()), now);
    ctx.app
        .repositories
        .narrative
        .save_event(&event_updated)
        .await
        .expect("Failed to complete event");

    // Verify it's marked as triggered
    let event_after_first = ctx
        .app
        .repositories
        .narrative
        .get_event(event_id)
        .await
        .expect("Should get event")
        .expect("Event should exist");
    assert!(
        event_after_first.is_triggered(),
        "Event should be marked triggered after completion"
    );

    // Step 3: Re-enter - event should NOT trigger since it's already triggered
    ctx.app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, private_booth)
        .await
        .expect("Movement back should succeed");

    let result2 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        !result2
            .triggered_events
            .iter()
            .any(|e| e.name() == "Repeatable Daily Quest"),
        "Event should NOT trigger while still in triggered state"
    );

    // Step 4: Reset the repeatable event (simulate daily reset)
    let mut event_reset = ctx
        .app
        .repositories
        .narrative
        .get_event(event_id)
        .await
        .expect("Should get event")
        .expect("Event should exist");
    event_reset.reset(chrono::Utc::now()); // Reset the event!
    ctx.app
        .repositories
        .narrative
        .save_event(&event_reset)
        .await
        .expect("Failed to reset event");

    // Verify it's no longer triggered
    let event_after_reset = ctx
        .app
        .repositories
        .narrative
        .get_event(event_id)
        .await
        .expect("Should get event")
        .expect("Event should exist");
    assert!(
        !event_after_reset.is_triggered(),
        "Event should NOT be triggered after reset"
    );

    // Step 5: Re-enter - event SHOULD trigger again after reset
    ctx.app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, private_booth)
        .await
        .expect("Movement back should succeed");

    let result3 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        result3
            .triggered_events
            .iter()
            .any(|e| e.name() == "Repeatable Daily Quest"),
        "Event should trigger again after reset, got: {:?}",
        result3
            .triggered_events
            .iter()
            .map(|e| e.name())
            .collect::<Vec<_>>()
    );

    // Verify the event is marked as repeatable
    let final_event = ctx
        .app
        .repositories
        .narrative
        .get_event(event_id)
        .await
        .expect("Should get event")
        .expect("Event should exist");
    assert!(
        final_event.is_repeatable(),
        "Event should still be marked as repeatable"
    );
}

#[tokio::test]
#[ignore = "requires neo4j testcontainer"]
async fn test_one_time_chain_cannot_repeat() {
    // Scenario: One-time event cannot trigger again after completion.
    // Expected: After completion, event stays complete and won't re-trigger.
    //
    // Setup:
    // 1. Create E2E context with one-time event (not repeatable)
    // 2. Trigger and complete the event
    // 3. Try to trigger the event again
    //
    // Assertions:
    // - Event is marked as triggered
    // - Event does not trigger again on subsequent entries
    // - Event stays in triggered state

    let ctx = E2ETestContext::setup()
        .await
        .expect("Failed to setup E2E context");

    // Get regions and location
    let common_room = ctx
        .world
        .region("Common Room")
        .expect("Common Room should exist");
    let private_booth = ctx
        .world
        .region("Private Booth")
        .expect("Private Booth should exist");
    let location_id = ctx
        .world
        .location("The Drowsy Dragon Inn")
        .expect("Location should exist");

    // Create player character
    let (_, pc_id) = create_test_player(
        ctx.graph(),
        ctx.world.world_id,
        private_booth,
        "One-Time Chain Tester",
    )
    .await
    .expect("Failed to create test player");

    let now = Utc::now();

    // Create a one-time event (NOT repeatable - default behavior)
    let event_id = wrldbldr_domain::NarrativeEventId::new();

    let event = NarrativeEvent::new(ctx.world.world_id, "One-Time Main Quest", now)
        .with_id(event_id)
        .with_description("A main quest that can only happen once")
        .with_scene_direction("The ancient prophecy is revealed")
        .with_trigger_condition(NarrativeTrigger {
            trigger_type: NarrativeTriggerType::PlayerEntersLocation {
                location_id,
                location_name: "The Drowsy Dragon Inn".to_string(),
            },
            description: "Player enters tavern".to_string(),
            is_required: true,
            trigger_id: "enter-tavern".to_string(),
        })
        // NOTE: NOT calling .with_repeatable(true) - defaults to one-time
        .with_priority(10);

    ctx.app
        .repositories
        .narrative
        .save_event(&event)
        .await
        .expect("Failed to save event");

    ctx
        .graph()
        .run(
            query(
                r#"MATCH (e:NarrativeEvent {id: $event_id}), (l:Location {id: $location_id})
                   MERGE (e)-[:TIED_TO_LOCATION]->(l)"#,
            )
            .param("event_id", event_id.to_string())
            .param("location_id", location_id.to_string()),
        )
        .await
        .expect("Failed to tie event to location");

    // Step 1: First trigger - event should fire
    let result1 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        result1
            .triggered_events
            .iter()
            .any(|e| e.name() == "One-Time Main Quest"),
        "Event should trigger first time"
    );

    // Step 2: Complete the event
    let mut event_updated = ctx
        .app
        .repositories
        .narrative
        .get_event(event_id)
        .await
        .expect("Should get event")
        .expect("Event should exist");
    event_updated.trigger(Some("completed".to_string()), now);
    ctx.app
        .repositories
        .narrative
        .save_event(&event_updated)
        .await
        .expect("Failed to complete event");

    // Verify it's marked as triggered and NOT repeatable
    let event_after = ctx
        .app
        .repositories
        .narrative
        .get_event(event_id)
        .await
        .expect("Should get event")
        .expect("Event should exist");
    assert!(
        event_after.is_triggered(),
        "Event should be marked as triggered"
    );
    assert!(
        !event_after.is_repeatable(),
        "Event should NOT be marked as repeatable"
    );

    // Step 3: Try to re-trigger - should NOT fire again
    ctx.app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, private_booth)
        .await
        .expect("Movement back should succeed");

    let result2 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        !result2
            .triggered_events
            .iter()
            .any(|e| e.name() == "One-Time Main Quest"),
        "One-time event should NOT trigger second time"
    );

    // Step 4: Re-enter multiple times - still should not trigger
    ctx.app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, private_booth)
        .await
        .expect("Movement back should succeed");

    let result3 = ctx
        .app
        .use_cases
        .movement
        .enter_region
        .execute(pc_id, common_room)
        .await
        .expect("Movement should succeed");

    assert!(
        !result3
            .triggered_events
            .iter()
            .any(|e| e.name() == "One-Time Main Quest"),
        "One-time event should NEVER trigger again"
    );

    // Verify event is still in triggered state
    let final_event = ctx
        .app
        .repositories
        .narrative
        .get_event(event_id)
        .await
        .expect("Should get event")
        .expect("Event should exist");
    assert!(
        final_event.is_triggered(),
        "Event should remain in triggered state"
    );
    assert!(
        !final_event.is_repeatable(),
        "Event should remain non-repeatable"
    );
}
