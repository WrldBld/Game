//! ComfyUI Configuration - Value object for ComfyUI retry and timeout settings
//!
//! # Architectural Note (ADR-001: Domain Serialization)
//!
//! `ComfyUIConfig` includes serde derives because:
//! 1. It is loaded from configuration files at startup
//! 2. The config file format IS the domain specification
//! 3. This is a pure data structure with no behavior
//!
//! This is an accepted exception to the "no serde in domain" rule.

use serde::{Deserialize, Serialize};

use crate::error::DomainError;

/// Configuration for ComfyUI retry behavior and timeouts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComfyUIConfig {
    /// Maximum number of retries (1-5)
    max_retries: u8,
    /// Base delay in seconds for exponential backoff (1-30)
    base_delay_seconds: u8,
    /// Timeout for queue_prompt operations in seconds
    queue_timeout_seconds: u16,
    /// Timeout for get_history operations in seconds
    history_timeout_seconds: u16,
    /// Timeout for get_image operations in seconds
    image_timeout_seconds: u16,
}

impl Default for ComfyUIConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_seconds: 5,
            queue_timeout_seconds: 30,
            history_timeout_seconds: 10,
            image_timeout_seconds: 60,
        }
    }
}

impl ComfyUIConfig {
    /// Create a new ComfyUIConfig with validation
    pub fn new(
        max_retries: u8,
        base_delay_seconds: u8,
        queue_timeout_seconds: u16,
        history_timeout_seconds: u16,
        image_timeout_seconds: u16,
    ) -> Result<Self, DomainError> {
        let config = Self {
            max_retries,
            base_delay_seconds,
            queue_timeout_seconds,
            history_timeout_seconds,
            image_timeout_seconds,
        };
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), DomainError> {
        if !(1..=5).contains(&self.max_retries) {
            return Err(DomainError::validation(
                "max_retries must be between 1 and 5",
            ));
        }
        if !(1..=30).contains(&self.base_delay_seconds) {
            return Err(DomainError::validation(
                "base_delay_seconds must be between 1 and 30",
            ));
        }
        if self.queue_timeout_seconds == 0 {
            return Err(DomainError::validation(
                "queue_timeout_seconds must be greater than 0",
            ));
        }
        if self.history_timeout_seconds == 0 {
            return Err(DomainError::validation(
                "history_timeout_seconds must be greater than 0",
            ));
        }
        if self.image_timeout_seconds == 0 {
            return Err(DomainError::validation(
                "image_timeout_seconds must be greater than 0",
            ));
        }
        Ok(())
    }

    // ============================================================================
    // Accessors
    // ============================================================================

    /// Maximum number of retries (1-5)
    pub fn max_retries(&self) -> u8 {
        self.max_retries
    }

    /// Base delay in seconds for exponential backoff (1-30)
    pub fn base_delay_seconds(&self) -> u8 {
        self.base_delay_seconds
    }

    /// Timeout for queue_prompt operations in seconds
    pub fn queue_timeout_seconds(&self) -> u16 {
        self.queue_timeout_seconds
    }

    /// Timeout for get_history operations in seconds
    pub fn history_timeout_seconds(&self) -> u16 {
        self.history_timeout_seconds
    }

    /// Timeout for get_image operations in seconds
    pub fn image_timeout_seconds(&self) -> u16 {
        self.image_timeout_seconds
    }

    // ============================================================================
    // Builder-style setters (consume self)
    // ============================================================================

    /// Set maximum retries
    pub fn with_max_retries(self, max_retries: u8) -> Self {
        Self {
            max_retries,
            ..self
        }
    }

    /// Set base delay in seconds
    pub fn with_base_delay_seconds(self, base_delay_seconds: u8) -> Self {
        Self {
            base_delay_seconds,
            ..self
        }
    }

    /// Set queue timeout in seconds
    pub fn with_queue_timeout_seconds(self, queue_timeout_seconds: u16) -> Self {
        Self {
            queue_timeout_seconds,
            ..self
        }
    }

    /// Set history timeout in seconds
    pub fn with_history_timeout_seconds(self, history_timeout_seconds: u16) -> Self {
        Self {
            history_timeout_seconds,
            ..self
        }
    }

    /// Set image timeout in seconds
    pub fn with_image_timeout_seconds(self, image_timeout_seconds: u16) -> Self {
        Self {
            image_timeout_seconds,
            ..self
        }
    }
}
