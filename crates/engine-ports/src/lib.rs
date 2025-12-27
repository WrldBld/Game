pub mod inbound;
pub mod outbound;

// Re-export mocks for test builds
#[cfg(any(test, feature = "testing"))]
pub use outbound::MockChallengeRepositoryPort;
