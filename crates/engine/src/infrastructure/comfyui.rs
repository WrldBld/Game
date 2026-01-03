//! ComfyUI image generation client - stub implementation.

use async_trait::async_trait;
use crate::infrastructure::ports::{ImageGenPort, ImageRequest, ImageResult, ImageGenError};

pub struct ComfyUIClient {
    #[allow(dead_code)]
    base_url: String,
}

impl ComfyUIClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
}

#[async_trait]
impl ImageGenPort for ComfyUIClient {
    async fn generate(&self, _request: ImageRequest) -> Result<ImageResult, ImageGenError> {
        todo!("ComfyUI: generate")
    }

    async fn check_health(&self) -> Result<bool, ImageGenError> {
        todo!("ComfyUI: check_health")
    }
}
