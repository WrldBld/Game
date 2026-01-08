//! Game Connection Ports - ISP-compliant split traits for Engine WebSocket operations
//!
//! This module provides Interface Segregation Principle (ISP) compliant traits
//! that split the monolithic `GameConnectionPort` into focused, single-responsibility
//! interfaces. This allows consumers to depend only on the specific capabilities
//! they need.
//!
//! # Traits
//!
//! - [`ConnectionLifecyclePort`] - Connection state and lifecycle management (5 methods)
//! - [`SessionCommandPort`] - Session joining (1 method; callbacks remain on main trait)
//! - [`PlayerActionPort`] - Player gameplay actions (7 methods: combat, inventory)
//! - [`DmControlPort`] - Dungeon Master operations (12 methods: scenes, approvals, NPCs)
//! - [`NavigationPort`] - Player movement between regions/locations (2 methods)
//! - [`GameRequestPort`] - Request-response operations (3 methods)
//!
//! # Design Philosophy
//!
//! Unlike traditional ISP implementations that create a super-trait combining all
//! sub-traits, this design keeps traits completely independent. Implementations
//! that need all capabilities simply implement all traits. This provides:
//!
//! - Maximum flexibility for partial implementations
//! - Clear boundaries for mocking in tests
//! - Better documentation of what capabilities each consumer needs
//!
//! # Callback Methods
//!
//! The `on_state_change` and `on_message` callback registration methods remain
//! on the main `GameConnectionPort` trait because mockall doesn't support mocking
//! `Fn` objects. These methods are typically only needed by top-level session
//! management code anyway.
//!
//! # Example
//!
//! ```ignore
//! // A service that only needs navigation
//! fn move_player(nav: &dyn NavigationPort, pc_id: &str, region_id: &str) -> anyhow::Result<()> {
//!     nav.move_to_region(pc_id, region_id)
//! }
//!
//! // A DM-only service
//! fn approve_action(dm: &dyn DmControlPort, request_id: &str) -> anyhow::Result<()> {
//!     dm.send_approval_decision(request_id, ApprovalDecision::Accept)
//! }
//! ```

pub mod dm_control_port;
pub mod lifecycle_port;
pub mod navigation_port;
pub mod player_action_port;
pub mod request_port;
pub mod session_port;

// Re-export traits for convenient access
pub use dm_control_port::DmControlPort;
pub use lifecycle_port::ConnectionLifecyclePort;
pub use navigation_port::NavigationPort;
pub use player_action_port::PlayerActionPort;
pub use request_port::GameRequestPort;
pub use session_port::SessionCommandPort;

// Re-export mock types when testing feature is enabled
#[cfg(any(test, feature = "testing"))]
pub use dm_control_port::MockDmControlPort;
#[cfg(any(test, feature = "testing"))]
pub use lifecycle_port::MockConnectionLifecyclePort;
#[cfg(any(test, feature = "testing"))]
pub use navigation_port::MockNavigationPort;
#[cfg(any(test, feature = "testing"))]
pub use player_action_port::MockPlayerActionPort;
#[cfg(any(test, feature = "testing"))]
pub use request_port::MockGameRequestPort;
#[cfg(any(test, feature = "testing"))]
pub use session_port::MockSessionCommandPort;

/// Combined mock module for testing scenarios that need all traits
///
/// This module provides a `CombinedGameConnectionMock` struct that wraps
/// all individual mocks, useful for integration tests that need complete
/// game connection functionality.
#[cfg(any(test, feature = "testing"))]
pub mod mock {
    use super::*;

    /// A combined mock that holds all individual port mocks
    ///
    /// Use this when you need to mock complete game connection functionality
    /// in integration tests. For unit tests, prefer using individual mocks
    /// to test only the specific traits your code depends on.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut mocks = CombinedGameConnectionMock::new();
    ///
    /// // Configure individual mocks
    /// mocks.lifecycle.expect_connect().returning(|| Ok(()));
    /// mocks.session.expect_join_world().returning(|_, _, _| Ok(()));
    ///
    /// // Use the mocks
    /// mocks.lifecycle.connect()?;
    /// ```
    #[derive(Default)]
    pub struct CombinedGameConnectionMock {
        /// Connection lifecycle operations mock
        pub lifecycle: MockConnectionLifecyclePort,
        /// Session management operations mock
        pub session: MockSessionCommandPort,
        /// Player action operations mock
        pub player_action: MockPlayerActionPort,
        /// DM control operations mock
        pub dm_control: MockDmControlPort,
        /// Navigation operations mock
        pub navigation: MockNavigationPort,
        /// Request-response operations mock
        pub request: MockGameRequestPort,
    }

    impl CombinedGameConnectionMock {
        /// Create a new combined mock with default (unconfigured) mocks
        pub fn new() -> Self {
            Self::default()
        }
    }
}
