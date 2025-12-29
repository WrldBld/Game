//! Event Notifier Port - Interface for in-process event notification
//!
//! This port abstracts the mechanism for notifying subscribers when new events
//! arrive, enabling instant wake-ups for in-process consumers without polling.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        PRODUCER                                          │
//! │                                                                          │
//! │  EventBus publishes domain event                                         │
//! │      │                                                                   │
//! │      └──> notifier.notify() - wake up all waiters                       │
//! │                                                                          │
//! └──────────────────────────────┬──────────────────────────────────────────┘
//!                                │
//!                   ┌────────────▼─────────────┐
//!                   │    EventNotifierPort     │ (trait defined here)
//!                   │  notify() / wait()       │
//!                   └────────────┬─────────────┘
//!                                │
//! ┌──────────────────────────────▼──────────────────────────────────────────┐
//! │                        CONSUMER                                          │
//! │                                                                          │
//! │  Queue worker waits for events                                           │
//! │      │                                                                   │
//! │      └──> notifier.wait().await - sleep until notified                  │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Design Notes
//!
//! - This provides instant wake-up semantics complementing polling mechanisms
//! - The `notify()` method is synchronous to allow calling from sync contexts
//! - The `wait()` method is async for efficient suspension
//! - Multiple waiters can be woken simultaneously
//! - This is a low-level coordination primitive; higher-level ports like
//!   `QueueNotificationPort` may build on this

use async_trait::async_trait;

/// Port for in-process event notification
///
/// This trait provides a simple notify/wait mechanism for waking up
/// event subscribers when new events arrive.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to allow sharing across tasks.
/// The underlying synchronization primitive should handle concurrent
/// notify/wait operations correctly.
///
/// # Usage
///
/// Producers call `notify()` after publishing events:
/// ```rust,ignore
/// event_bus.publish(event).await?;
/// notifier.notify(); // Wake up consumers
/// ```
///
/// Consumers wait for notifications:
/// ```rust,ignore
/// loop {
///     notifier.wait().await;
///     // Process new events
///     let events = event_repo.fetch_since(last_id).await?;
/// }
/// ```
///
/// # Testing
///
/// Enable the `testing` feature to get mock implementations via mockall.
#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
#[async_trait]
pub trait EventNotifierPort: Send + Sync {
    /// Wake up all waiters
    ///
    /// This method is synchronous to allow calling from any context.
    /// It signals all tasks currently waiting on `wait()` to wake up.
    ///
    /// If no tasks are waiting, the notification may be lost (no queuing).
    /// This is intentional - consumers should check for new events after
    /// waking and handle the case where no new events exist.
    fn notify(&self);

    /// Wait for a notification
    ///
    /// This method suspends the current task until `notify()` is called.
    /// Multiple tasks can wait simultaneously and will all be woken.
    ///
    /// # Cancellation Safety
    ///
    /// This method is cancellation-safe. If the future is dropped before
    /// completing, no notification is lost for other waiters.
    async fn wait(&self);
}
