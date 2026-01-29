use super::test_support::*;

#[tokio::test]
async fn grant_lore_emits_lore_discovered_event() {
    let (mut harness, _db_guard) = setup_test_harness().await;

    // Create a world
    let world = harness.create_world("Test World").await;

    // Create DM user
    let (dm_user_id, mut dm_ws) = harness.create_user_and_ws("dm_user").await;

    // Join as DM
    harness
        .join_world(&mut dm_ws, world.id, wrldbldr_shared::WorldRole::Dm)
        .await;

    // Create player character
    let pc = harness.create_player_character(&world.id, "Hero").await;

    // Create player user
    let (player_user_id, mut player_ws) = harness.create_user_and_ws("player_user").await;

    // Join as player with the created PC
    harness
        .join_world_with_pc(&mut player_ws, world.id, pc.id)
        .await;

    // Create lore entry
    let lore = harness.create_lore(&world.id, "Ancient History").await;

    // Grant lore knowledge
    let grant_request = serde_json::json!({
        "request_id": "grant-1",
        "type": "LoreRequest",
        "payload": {
            "type": "GrantLoreKnowledge",
            "character_id": pc.id.to_string(),
            "lore_id": lore.id.to_string(),
            "discovery_source": {
                "type": "DmGranted",
                "reason": "Test grant"
            }
        }
    });

    // Capture messages sent to player
    let player_receiver = player_ws.take_receiver().unwrap();

    // Send grant request from DM
    dm_ws.send_json(&grant_request).await;

    // Player should receive LoreDiscovered event
    let discovered_msg = player_receiver.recv_timeout(Duration::from_secs(5)).await;
    assert!(discovered_msg.is_some(), "Player should receive LoreDiscovered event");

    if let Some(msg) = discovered_msg {
        let parsed: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(parsed["type"], "LoreDiscovered");
        assert_eq!(parsed["character_id"], pc.id.to_string());
        assert_eq!(parsed["lore"]["id"], lore.id.to_string());
        assert_eq!(parsed["lore"]["title"], "Ancient History");
        assert_eq!(parsed["discovered_chunk_ids"].as_array().unwrap().len(), 0); // Empty = all chunks
    }
}

#[tokio::test]
async fn get_character_lore_returns_journal_entries() {
    let (mut harness, _db_guard) = setup_test_harness().await;

    // Create a world
    let world = harness.create_world("Test World").await;

    // Create DM user
    let (dm_user_id, mut dm_ws) = harness.create_user_and_ws("dm_user").await;

    // Join as DM
    harness
        .join_world(&mut dm_ws, world.id, wrldbldr_shared::WorldRole::Dm)
        .await;

    // Create player character
    let pc = harness.create_player_character(&world.id, "Hero").await;

    // Create player user
    let (player_user_id, mut player_ws) = harness.create_user_and_ws("player_user").await;

    // Join as player with the created PC
    harness
        .join_world_with_pc(&mut player_ws, world.id, pc.id)
        .await;

    // Create two lore entries
    let lore1 = harness.create_lore(&world.id, "Lore 1").await;
    let lore2 = harness.create_lore(&world.id, "Lore 2").await;

    // Grant both lores to player
    harness
        .grant_lore_to_pc(&mut dm_ws, pc.id, lore1.id, None)
        .await;
    harness
        .grant_lore_to_pc(&mut dm_ws, pc.id, lore2.id, None)
        .await;

    // Request character lore
    let get_lore_request = serde_json::json!({
        "request_id": "get-lore-1",
        "type": "LoreRequest",
        "payload": {
            "type": "GetCharacterLore",
            "character_id": pc.id.to_string()
        }
    });

    let player_receiver = player_ws.take_receiver().unwrap();
    player_ws.send_json(&get_lore_request).await;

    // Should receive CharacterLoreResponse
    let response_msg = player_receiver.recv_timeout(Duration::from_secs(5)).await;
    assert!(response_msg.is_some(), "Should receive CharacterLoreResponse");

    if let Some(msg) = response_msg {
        let parsed: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(parsed["type"], "Response");
        assert_eq!(parsed["request_id"], "get-lore-1");
        assert_eq!(parsed["result"]["status"], "success");

        // Check that response contains lore journal entries
        let data = &parsed["result"]["data"];
        assert!(data.is_array());
        let entries = data.as_array().unwrap();
        assert_eq!(entries.len(), 2); // Both lores

        // Check first entry structure
        let entry1 = &entries[0];
        assert!(entry1["lore"].is_object());
        assert!(entry1["knowledge"].is_object());
        assert_eq!(entry1["knowledge"]["character_id"], pc.id.to_string());
        assert_eq!(entry1["knowledge"]["known_chunk_ids"].as_array().unwrap().len(), 0); // All chunks known
    }
}

#[tokio::test]
async fn revoke_lore_emits_lore_revoked_event() {
    let (mut harness, _db_guard) = setup_test_harness().await;

    // Create a world
    let world = harness.create_world("Test World").await;

    // Create DM user
    let (dm_user_id, mut dm_ws) = harness.create_user_and_ws("dm_user").await;

    // Join as DM
    harness
        .join_world(&mut dm_ws, world.id, wrldbldr_shared::WorldRole::Dm)
        .await;

    // Create player character
    let pc = harness.create_player_character(&world.id, "Hero").await;

    // Create player user
    let (player_user_id, mut player_ws) = harness.create_user_and_ws("player_user").await;

    // Join as player with the created PC
    harness
        .join_world_with_pc(&mut player_ws, world.id, pc.id)
        .await;

    // Create lore entry
    let lore = harness.create_lore(&world.id, "Ancient History").await;

    // Grant lore knowledge first
    harness
        .grant_lore_to_pc(&mut dm_ws, pc.id, lore.id, None)
        .await;

    // Clear player receiver to ignore grant event
    let _ = player_ws.take_receiver();

    // Capture messages sent to player
    let player_receiver = player_ws.take_receiver().unwrap();

    // Revoke lore knowledge
    let revoke_request = serde_json::json!({
        "request_id": "revoke-1",
        "type": "LoreRequest",
        "payload": {
            "type": "RevokeLoreKnowledge",
            "character_id": pc.id.to_string(),
            "lore_id": lore.id.to_string()
        }
    });

    // Send revoke request from DM
    dm_ws.send_json(&revoke_request).await;

    // Player should receive LoreRevoked event
    let revoked_msg = player_receiver.recv_timeout(Duration::from_secs(5)).await;
    assert!(revoked_msg.is_some(), "Player should receive LoreRevoked event");

    if let Some(msg) = revoked_msg {
        let parsed: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(parsed["type"], "LoreRevoked");
        assert_eq!(parsed["character_id"], pc.id.to_string());
        assert_eq!(parsed["lore_id"], lore.id.to_string());
    }
}
