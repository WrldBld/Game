//! Integration tests for Observation system (P1)
//!
//! Tests:
//! - ObservationSnapshot emission on PC selection/spectate
//! - ObservationUpdated emission on movement (record_visit)
//! - ObservationUpdated emission on CreateObservation
//! - ObservationDeleted emission on DeleteObservation

use crate::api::websocket::test_support::{TestAppBuilder, TestConnectionInfo};
use wrldbldr_domain::{CharacterId, LocationId, PlayerCharacterId, RegionId, UserId};
use wrldbldr_shared::{
    ClientMessage, ObservationRequest, ServerMessage,
};
use wrldbldr_shared::requests::observation::CreateObservationData;

#[tokio::test]
async fn observation_snapshot_emitted_on_select_player_character() {
    let mut builder = TestAppBuilder::new();
    let pc_id = PlayerCharacterId::new();
    let user_id = UserId::new("player1").unwrap();

    // Create world and PC
    let world_id = builder.create_world("Test World").await;
    let location_id = builder.add_location(world_id, "Tavern").await;
    let region_id = builder.add_region(location_id, "Common Room").await;
    let _npc_id = builder.add_npc(world_id, "Barkeep").await;
    builder.add_pc(pc_id, "Hero", user_id.clone()).await;
    builder.set_pc_location(pc_id, region_id).await;

    // Connect and select PC
    let (addr, mut client) = builder
        .connect_user(user_id.clone(), Some(world_id), Some(pc_id))
        .await;

    // Select the PC
    let select_msg = ClientMessage::SelectPlayerCharacter {
        pc_id: pc_id.to_string(),
    };
    let _ = client.send_message(select_msg).await;

    // Wait for ObservationSnapshot
    let snapshot_msg = client
        .wait_for_message_timeout(Duration::from_secs(2), |msg| {
            matches!(msg, ServerMessage::ObservationSnapshot { .. })
        })
        .await;

    assert!(
        snapshot_msg.is_some(),
        "Should receive ObservationSnapshot after selecting PC"
    );

    if let Some(ServerMessage::ObservationSnapshot { pc_id, observations }) = snapshot_msg {
        assert_eq!(pc_id, pc_id.to_string());
        // Initially should be empty
        assert!(observations.is_empty());
    } else {
        panic!("Expected ObservationSnapshot message");
    }

    drop(client);
    drop(addr);
}

#[tokio::test]
async fn observation_snapshot_emitted_on_spectate() {
    let mut builder = TestAppBuilder::new();
    let pc_id = PlayerCharacterId::new();
    let spectator_user_id = UserId::new("spectator1").unwrap();
    let player_user_id = UserId::new("player1").unwrap();

    // Create world and PC
    let world_id = builder.create_world("Test World").await;
    let location_id = builder.add_location(world_id, "Tavern").await;
    let region_id = builder.add_region(location_id, "Common Room").await;
    builder.add_pc(pc_id, "Hero", player_user_id.clone()).await;
    builder.set_pc_location(pc_id, region_id).await;

    // Connect as spectator
    let (addr, mut client) = builder
        .connect_user(spectator_user_id.clone(), Some(world_id), None)
        .await;

    // Set spectate target
    let spectate_msg = ClientMessage::SetSpectateTarget {
        pc_id: pc_id,
    };
    let _ = client.send_message(spectate_msg).await;

    // Wait for ObservationSnapshot
    let snapshot_msg = client
        .wait_for_message_timeout(Duration::from_secs(2), |msg| {
            matches!(msg, ServerMessage::ObservationSnapshot { .. })
        })
        .await;

    assert!(
        snapshot_msg.is_some(),
        "Should receive ObservationSnapshot after setting spectate target"
    );

    if let Some(ServerMessage::ObservationSnapshot { pc_id, observations }) = snapshot_msg {
        assert_eq!(pc_id, pc_id.to_string());
        assert!(observations.is_empty());
    } else {
        panic!("Expected ObservationSnapshot message");
    }

    drop(client);
    drop(addr);
}

#[tokio::test]
async fn observation_updated_on_movement() {
    let mut builder = TestAppBuilder::new();
    let pc_id = PlayerCharacterId::new();
    let user_id = UserId::new("player1").unwrap();

    // Create world, PC, and NPC
    let world_id = builder.create_world("Test World").await;
    let location_id = builder.add_location(world_id, "Tavern").await;
    let region1_id = builder.add_region(location_id, "Common Room").await;
    let region2_id = builder.add_region(location_id, "Private Room").await;
    let npc_id = builder.add_npc(world_id, "Barkeep").await;

    builder.add_pc(pc_id, "Hero", user_id.clone()).await;
    builder.set_pc_location(pc_id, region1_id).await;

    // Staging for region2 with the NPC present
    builder.stage_region(region2_id, world_id, vec![(npc_id, true)]).await;

    // Connect as player
    let (addr, mut client) = builder
        .connect_user(user_id.clone(), Some(world_id), Some(pc_id))
        .await;

    // Move to region with NPC (should create observation)
    let move_msg = ClientMessage::MoveToRegion {
        pc_id: pc_id.to_string(),
        region_id: region2_id.to_string(),
    };
    let _ = client.send_message(move_msg).await;

    // Wait for ObservationUpdated
    let updated_msg = client
        .wait_for_message_timeout(Duration::from_secs(2), |msg| {
            matches!(msg, ServerMessage::ObservationUpdated { .. })
        })
        .await;

    assert!(
        updated_msg.is_some(),
        "Should receive ObservationUpdated after moving to region with NPC"
    );

    if let Some(ServerMessage::ObservationUpdated { pc_id, observation }) = updated_msg {
        assert_eq!(pc_id, pc_id.to_string());
        assert_eq!(observation.npc_id, npc_id.to_string());
        assert_eq!(observation.observation_type, "direct");
    } else {
        panic!("Expected ObservationUpdated message");
    }

    drop(client);
    drop(addr);
}

#[tokio::test]
async fn observation_updated_on_dm_create_observation() {
    let mut builder = TestAppBuilder::new();
    let pc_id = PlayerCharacterId::new();
    let dm_user_id = UserId::new("dm").unwrap();
    let player_user_id = UserId::new("player1").unwrap();

    // Create world, PC, and NPC
    let world_id = builder.create_world("Test World").await;
    let location_id = builder.add_location(world_id, "Market").await;
    let region_id = builder.add_region(location_id, "Market Square").await;
    let npc_id = builder.add_npc(world_id, "Merchant").await;

    builder.add_pc(pc_id, "Hero", player_user_id.clone()).await;
    builder.set_pc_location(pc_id, region_id).await;

    // Connect as DM
    let (addr, mut client) = builder
        .connect_user(dm_user_id.clone(), Some(world_id), None)
        .await;

    // Create observation (HeardAbout type)
    let create_obs_data = CreateObservationData {
        npc_id: npc_id.to_string(),
        observation_type: "heard_about".to_string(),
        location_id: Some(location_id.to_string()),
        region_id: Some(region_id.to_string()),
        notes: Some("Rumor has it".to_string()),
    };

    let create_msg = ClientMessage::ObservationRequest(
        ObservationRequest::CreateObservation {
            pc_id: pc_id.to_string(),
            data: create_obs_data,
        },
    );
    let _ = client.send_message(create_msg).await;

    // Player should receive ObservationUpdated
    let player_client = builder.connect_user(player_user_id.clone(), Some(world_id), Some(pc_id)).await.1;

    let updated_msg = player_client
        .wait_for_message_timeout(Duration::from_secs(2), |msg| {
            matches!(msg, ServerMessage::ObservationUpdated { .. })
        })
        .await;

    assert!(
        updated_msg.is_some(),
        "Player should receive ObservationUpdated when DM creates observation"
    );

    if let Some(ServerMessage::ObservationUpdated { pc_id, observation }) = updated_msg {
        assert_eq!(pc_id, pc_id.to_string());
        assert_eq!(observation.npc_id, npc_id.to_string());
        assert_eq!(observation.observation_type, "heard_about");
        assert_eq!(observation.notes, Some("Rumor has it".to_string()));
    } else {
        panic!("Expected ObservationUpdated message");
    }

    drop(player_client);
    drop(client);
    drop(addr);
}

#[tokio::test]
async fn observation_deleted_on_dm_delete_observation() {
    let mut builder = TestAppBuilder::new();
    let pc_id = PlayerCharacterId::new();
    let dm_user_id = UserId::new("dm").unwrap();
    let player_user_id = UserId::new("player1").unwrap();

    // Create world, PC, and NPC
    let world_id = builder.create_world("Test World").await;
    let location_id = builder.add_location(world_id, "Market").await;
    let region_id = builder.add_region(location_id, "Market Square").await;
    let npc_id = builder.add_npc(world_id, "Merchant").await;

    builder.add_pc(pc_id, "Hero", player_user_id.clone()).await;
    builder.set_pc_location(pc_id, region_id).await;

    // Pre-create an observation
    let create_obs_data = CreateObservationData {
        npc_id: npc_id.to_string(),
        observation_type: "heard_about".to_string(),
        location_id: Some(location_id.to_string()),
        region_id: Some(region_id.to_string()),
        notes: None,
    };
    builder.create_observation(pc_id, create_obs_data).await;

    // Connect as DM
    let (addr, mut client) = builder
        .connect_user(dm_user_id.clone(), Some(world_id), None)
        .await;

    // Delete observation
    let delete_msg = ClientMessage::ObservationRequest(
        ObservationRequest::DeleteObservation {
            pc_id: pc_id.to_string(),
            npc_id: npc_id.to_string(),
        },
    );
    let _ = client.send_message(delete_msg).await;

    // Player should receive ObservationDeleted
    let player_client = builder.connect_user(player_user_id.clone(), Some(world_id), Some(pc_id)).await.1;

    let deleted_msg = player_client
        .wait_for_message_timeout(Duration::from_secs(2), |msg| {
            matches!(msg, ServerMessage::ObservationDeleted { .. })
        })
        .await;

    assert!(
        deleted_msg.is_some(),
        "Player should receive ObservationDeleted when DM deletes observation"
    );

    if let Some(ServerMessage::ObservationDeleted { pc_id, npc_id }) = deleted_msg {
        assert_eq!(pc_id, pc_id.to_string());
        assert_eq!(npc_id, npc_id.to_string());
    } else {
        panic!("Expected ObservationDeleted message");
    }

    drop(player_client);
    drop(client);
    drop(addr);
}
