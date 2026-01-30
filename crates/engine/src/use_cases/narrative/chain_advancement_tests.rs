//! Unit tests for event chain advancement when events are triggered.

use chrono::{TimeZone, Utc};
use std::sync::Arc;
use wrldbldr_domain::{
    EventChain, EventChainFavorite, EventChainId, EventChainName, NarrativeEvent, NarrativeEventId,
    NarrativeEventName, WorldId,
};

// =============================================================================
// Event Chain Ops Tests (world ownership validation)
// =============================================================================

mod chain_ops_world_ownership {
    use super::*;
    use crate::infrastructure::clock::FixedClock;
    use crate::infrastructure::ports::{ClockPort, MockNarrativeRepo};
    use crate::use_cases::narrative::chains::{EventChainError, EventChainOps};
    use crate::use_cases::narrative_operations::NarrativeOps;

    struct FixedClockWrapper(chrono::DateTime<Utc>);

    impl ClockPort for FixedClockWrapper {
        fn now(&self) -> chrono::DateTime<chrono::Utc> {
            self.0
        }
    }

    fn create_test_chain(world_id: WorldId) -> EventChain {
        let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        EventChain::new(world_id, EventChainName::new("Test Chain").unwrap(), now)
            .expect("create chain")
    }

    fn create_test_event(world_id: WorldId) -> NarrativeEvent {
        let now = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        NarrativeEvent::new(
            world_id,
            NarrativeEventName::new("Test Event").unwrap(),
            now,
        )
    }

    fn create_chain_ops(world_id: WorldId) -> (EventChainOps, Arc<MockNarrativeRepo>) {
        let clock = Arc::new(FixedClockWrapper(Utc::now())) as Arc<dyn ClockPort>;
        let narrative_repo = Arc::new(MockNarrativeRepo::new());
        let narrative_ops = NarrativeOps::new(
            narrative_repo.clone(),
            Arc::new(crate::infrastructure::ports::MockLocationRepo::new()),
            Arc::new(crate::infrastructure::ports::MockWorldRepo::new()),
            Arc::new(crate::infrastructure::ports::MockPlayerCharacterRepo::new()),
            Arc::new(crate::infrastructure::ports::MockCharacterRepo::new()),
            Arc::new(crate::infrastructure::ports::MockObservationRepo::new()),
            Arc::new(crate::infrastructure::ports::MockChallengeRepo::new()),
            Arc::new(crate::infrastructure::ports::MockFlagRepo::new()),
            Arc::new(crate::infrastructure::ports::MockSceneRepo::new()),
            clock,
        );

        let chain_ops = EventChainOps::new(Arc::new(narrative_ops));
        (chain_ops, narrative_repo)
    }

    #[tokio::test]
    async fn add_event_validates_world_ownership() {
        let world1 = WorldId::new();
        let world2 = WorldId::new();

        let (chain_ops, mut mock_repo) = create_chain_ops(world1);
        let chain = create_test_chain(world1);
        let chain_id = chain.id();

        // Setup: Save chain with world1
        mock_repo
            .expect_get_chain()
            .times(1)
            .returning(move |_| Ok(Some(chain.clone())));

        mock_repo.expect_save_chain().times(1).returning(|_| Ok(()));

        // Setup: Event from world2 (different world)
        let event_world2 = create_test_event(world2);
        let event_id = event_world2.id();

        mock_repo
            .expect_get_event()
            .times(1)
            .returning(move |_| Ok(Some(event_world2.clone())));

        // Execute: Try to add event from world2 to chain in world1
        let result = chain_ops.add_event(chain_id, event_id, None).await;

        // Assert: Should fail with WorldMismatch
        assert!(matches!(result, Err(EventChainError::WorldMismatch)));
    }

    #[tokio::test]
    async fn remove_event_validates_world_ownership() {
        let world1 = WorldId::new();
        let world2 = WorldId::new();

        let (chain_ops, mut mock_repo) = create_chain_ops(world1);
        let mut chain = create_test_chain(world1);
        let chain_id = chain.id();
        let event_world1 = create_test_event(world1);

        // Add event to chain
        chain.add_event(event_world1.id(), Utc::now());

        // Setup: Save chain with world1
        mock_repo
            .expect_get_chain()
            .times(1)
            .returning(move |_| Ok(Some(chain.clone())));

        mock_repo.expect_save_chain().times(1).returning(|_| Ok(()));

        // Setup: Event from world2 (different world)
        let event_world2 = create_test_event(world2);
        let event_id = event_world2.id();

        mock_repo
            .expect_get_event()
            .times(1)
            .returning(move |_| Ok(Some(event_world2.clone())));

        // Execute: Try to remove event from world2 from chain in world1
        let result = chain_ops.remove_event(chain_id, event_id).await;

        // Assert: Should fail with WorldMismatch
        assert!(matches!(result, Err(EventChainError::WorldMismatch)));
    }

    #[tokio::test]
    async fn complete_event_validates_event_exists() {
        let world1 = WorldId::new();

        let (chain_ops, mut mock_repo) = create_chain_ops(world1);
        let mut chain = create_test_chain(world1);
        let chain_id = chain.id();
        let event_world1 = create_test_event(world1);

        // Add event to chain
        chain.add_event(event_world1.id(), Utc::now());

        // Setup: Save chain with world1
        mock_repo
            .expect_get_chain()
            .times(1)
            .returning(move |_| Ok(Some(chain.clone())));

        mock_repo.expect_save_chain().times(1).returning(|_| Ok(()));

        // Setup: Event doesn't exist (returns None)
        let nonexistent_id = NarrativeEventId::new();

        mock_repo
            .expect_get_event()
            .times(1)
            .returning(move |_| Ok(None));

        // Execute: Try to complete nonexistent event
        let result = chain_ops.complete_event(chain_id, nonexistent_id).await;

        // Assert: Should fail with EventNotFound
        assert!(matches!(result, Err(EventChainError::EventNotFound(_))));
    }

    #[tokio::test]
    async fn complete_event_validates_event_in_chain() {
        let world1 = WorldId::new();

        let (chain_ops, mut mock_repo) = create_chain_ops(world1);
        let mut chain = create_test_chain(world1);
        let chain_id = chain.id();
        let event_world1 = create_test_event(world1);
        let event_id = event_world1.id();

        // DON'T add event to chain - chain is empty

        // Setup: Save chain with world1
        mock_repo
            .expect_get_chain()
            .times(1)
            .returning(move |_| Ok(Some(chain.clone())));

        mock_repo.expect_save_chain().times(0); // Should not save

        // Setup: Event exists
        mock_repo
            .expect_get_event()
            .times(1)
            .returning(move |_| Ok(Some(event_world1.clone())));

        // Execute: Try to complete event that's not in the chain
        let result = chain_ops.complete_event(chain_id, event_id).await;

        // Assert: Should fail with EventNotInChain
        assert!(matches!(result, Err(EventChainError::EventNotInChain)));
    }

    #[tokio::test]
    async fn complete_event_validates_world_ownership() {
        let world1 = WorldId::new();
        let world2 = WorldId::new();

        let (chain_ops, mut mock_repo) = create_chain_ops(world1);
        let mut chain = create_test_chain(world1);
        let chain_id = chain.id();
        let event_world1 = create_test_event(world1);
        let event_id = event_world1.id();

        // Add event to chain
        chain.add_event(event_id, Utc::now());

        // Setup: Save chain with world1
        mock_repo
            .expect_get_chain()
            .times(1)
            .returning(move |_| Ok(Some(chain.clone())));

        mock_repo.expect_save_chain().times(0); // Should not save

        // Setup: Event from world2 (different world)
        let event_world2 = create_test_event(world2);

        mock_repo
            .expect_get_event()
            .times(1)
            .returning(move |_| Ok(Some(event_world2.clone())));

        // Execute: Try to complete event from world2 in chain in world1
        let result = chain_ops.complete_event(chain_id, event_id).await;

        // Assert: Should fail with WorldMismatch
        assert!(matches!(result, Err(EventChainError::WorldMismatch)));
    }

    #[tokio::test]
    async fn complete_event_succeeds_when_valid() {
        let world1 = WorldId::new();

        let (chain_ops, mut mock_repo) = create_chain_ops(world1);
        let mut chain = create_test_chain(world1);
        let chain_id = chain.id();
        let event_world1 = create_test_event(world1);
        let event_id = event_world1.id();

        // Add event to chain
        chain.add_event(event_id, Utc::now());

        let chain_clone_for_save = chain.clone();
        // Setup: Save chain with world1
        mock_repo
            .expect_get_chain()
            .times(1)
            .returning(move |_| Ok(Some(chain_clone_for_save.clone())));

        mock_repo.expect_save_chain().times(1).returning(|_| Ok(()));

        // Setup: Event exists in same world
        mock_repo
            .expect_get_event()
            .times(1)
            .returning(move |_| Ok(Some(event_world1.clone())));

        // Execute: Complete event
        let result = chain_ops.complete_event(chain_id, event_id).await;

        // Assert: Should succeed
        assert!(result.is_ok());
    }
}
