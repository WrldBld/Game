//! Live LLM runtime configuration with ArcSwap.
//!
//! This module provides runtime-swappable LLM configuration, allowing
//! base_url and model to be changed without restarting the server.

use std::sync::Arc;

use arc_swap::ArcSwap;

use crate::infrastructure::ports::{LlmModelsPort, LlmPort};
use crate::infrastructure::resilient_llm::ResilientLlmClient;

/// Runtime LLM configuration that can be swapped atomically.
///
/// This struct holds both the LLM client (for generation) and the models port
/// (for discovery), ensuring they always use the same base_url.
#[derive(Clone)]
#[allow(dead_code)]
pub struct LlmRuntime {
    /// The LLM client for generation requests
    llm: Arc<dyn LlmPort>,
    /// The models port for discovery
    models: Arc<dyn LlmModelsPort>,
    /// The effective base URL
    base_url: String,
    /// The effective model (used as fallback when request doesn't specify)
    model: String,
    /// The API key for authenticated requests (if configured)
    api_key: Option<String>,
}

impl LlmRuntime {
    /// Create a new LLM runtime.
    pub fn new(
        llm: Arc<dyn LlmPort>,
        models: Arc<dyn LlmModelsPort>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            llm,
            models,
            base_url: base_url.into(),
            model: model.into(),
            api_key: None,
        }
    }

    /// Create a new LLM runtime with API key.
    #[allow(dead_code)]
    pub fn with_api_key(
        llm: Arc<dyn LlmPort>,
        models: Arc<dyn LlmModelsPort>,
        base_url: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
    ) -> Self {
        Self {
            llm,
            models,
            base_url: base_url.into(),
            model: model.into(),
            api_key,
        }
    }

    /// Create a new runtime from an OpenAI-compatible client.
    ///
    /// The client is wrapped with ResilientLlmClient for retry/circuit breaker.
    #[allow(dead_code)]
    pub fn from_openai_client(
        client: Arc<crate::infrastructure::openai_compatible::OpenAICompatibleClient>,
        retry_config: crate::infrastructure::resilient_llm::RetryConfig,
    ) -> Self {
        let base_url = client.base_url().to_string();
        let model = client.model().to_string();
        let api_key = client.api_key().map(String::from);
        let llm: Arc<dyn LlmPort> = Arc::new(ResilientLlmClient::new(client.clone(), retry_config));
        let models: Arc<dyn LlmModelsPort> = client;

        Self {
            llm,
            models,
            base_url,
            model,
            api_key,
        }
    }

    /// Get the LLM port for generation.
    #[allow(dead_code)]
    pub fn llm(&self) -> Arc<dyn LlmPort> {
        self.llm.clone()
    }

    /// Get the models port for discovery.
    #[allow(dead_code)]
    pub fn models(&self) -> Arc<dyn LlmModelsPort> {
        self.models.clone()
    }

    /// Get the effective base URL.
    #[allow(dead_code)]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the effective model.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the API key (if configured).
    #[allow(dead_code)]
    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }
}

/// Atomic holder for the LLM runtime.
///
/// This uses ArcSwap to allow lock-free reads and atomic swaps.
pub struct LlmRuntimeHolder {
    runtime: ArcSwap<LlmRuntime>,
}

impl LlmRuntimeHolder {
    /// Create a new holder with the given runtime.
    pub fn new(runtime: LlmRuntime) -> Self {
        Self {
            runtime: ArcSwap::new(Arc::new(runtime)),
        }
    }

    /// Get a snapshot of the current runtime.
    ///
    /// This is cheap - just an atomic pointer read.
    pub fn current(&self) -> Arc<LlmRuntime> {
        self.runtime.load_full()
    }

    /// Swap the runtime atomically.
    ///
    /// In-flight requests using the old runtime will continue with the old config.
    pub fn swap(&self, runtime: LlmRuntime) {
        let old = self.runtime.swap(Arc::new(runtime));
        tracing::info!(
            base_url = %old.base_url,
            model = %old.model,
            "Swapped LLM runtime (old config)"
        );
    }

    /// Get the current base URL.
    #[allow(dead_code)]
    pub fn base_url(&self) -> String {
        self.runtime.load().base_url().to_string()
    }

    /// Get the current model.
    pub fn model(&self) -> String {
        self.runtime.load().model().to_string()
    }

    /// Get the current API key (if configured).
    #[allow(dead_code)]
    pub fn api_key(&self) -> Option<String> {
        self.runtime.load().api_key().map(String::from)
    }
}
