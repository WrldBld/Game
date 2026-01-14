//! Mock image generation for testing.
//!
//! Provides placeholder image generators for tests when ComfyUI is not available.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::test_fixtures::image_mocks::PlaceholderImageGen;
//!
//! #[tokio::test]
//! async fn test_image_flow() {
//!     let gen = PlaceholderImageGen::new();
//!     let result = gen.generate(request).await.unwrap();
//!     assert!(!result.image_data.is_empty());
//! }
//! ```

use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::infrastructure::ports::{ImageGenError, ImageGenPort, ImageRequest, ImageResult};

/// Mock image generator that returns placeholder images.
///
/// Uses pre-downloaded placeholder images from the test_data/images directory.
/// Falls back to generating minimal valid PNG data if placeholders are missing.
pub struct PlaceholderImageGen {
    images_dir: PathBuf,
    call_count: AtomicUsize,
}

impl PlaceholderImageGen {
    /// Create a new placeholder generator using the default test_data/images directory.
    pub fn new() -> Self {
        let images_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data")
            .join("images");
        Self {
            images_dir,
            call_count: AtomicUsize::new(0),
        }
    }

    /// Create with a custom images directory.
    pub fn with_dir(images_dir: PathBuf) -> Self {
        Self {
            images_dir,
            call_count: AtomicUsize::new(0),
        }
    }

    /// Get the number of generate calls made.
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::Relaxed)
    }

    /// Determine placeholder type from prompt keywords.
    fn placeholder_type_from_prompt(prompt: &str) -> PlaceholderType {
        let prompt_lower = prompt.to_lowercase();

        if prompt_lower.contains("portrait")
            || prompt_lower.contains("character")
            || prompt_lower.contains("face")
            || prompt_lower.contains("headshot")
        {
            PlaceholderType::Portrait
        } else if prompt_lower.contains("scene")
            || prompt_lower.contains("action")
            || prompt_lower.contains("battle")
            || prompt_lower.contains("combat")
        {
            PlaceholderType::Scene
        } else {
            PlaceholderType::Location
        }
    }

    /// Load placeholder image or generate minimal PNG.
    fn load_or_generate_placeholder(&self, placeholder_type: PlaceholderType) -> Vec<u8> {
        let filename = match placeholder_type {
            PlaceholderType::Portrait => "placeholder_portrait.png",
            PlaceholderType::Scene => "placeholder_scene.png",
            PlaceholderType::Location => "placeholder_location.png",
        };

        let path = self.images_dir.join(filename);

        if path.exists() {
            std::fs::read(&path).unwrap_or_else(|_| Self::generate_minimal_png())
        } else {
            // Generate minimal valid PNG if placeholder not found
            Self::generate_minimal_png()
        }
    }

    /// Generate a minimal valid PNG (1x1 transparent pixel).
    ///
    /// This is the smallest valid PNG file, useful when actual placeholders
    /// haven't been downloaded yet.
    fn generate_minimal_png() -> Vec<u8> {
        // Minimal 1x1 transparent PNG
        vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, // IHDR length
            0x49, 0x48, 0x44, 0x52, // IHDR
            0x00, 0x00, 0x00, 0x01, // width = 1
            0x00, 0x00, 0x00, 0x01, // height = 1
            0x08, 0x06, // bit depth = 8, color type = 6 (RGBA)
            0x00, 0x00, 0x00, // compression, filter, interlace
            0x1F, 0x15, 0xC4, 0x89, // IHDR CRC
            0x00, 0x00, 0x00, 0x0A, // IDAT length
            0x49, 0x44, 0x41, 0x54, // IDAT
            0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, // compressed data
            0x0D, 0x0A, 0x2D, 0xB4, // IDAT CRC
            0x00, 0x00, 0x00, 0x00, // IEND length
            0x49, 0x45, 0x4E, 0x44, // IEND
            0xAE, 0x42, 0x60, 0x82, // IEND CRC
        ]
    }
}

impl Default for PlaceholderImageGen {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
enum PlaceholderType {
    Portrait,
    Scene,
    Location,
}

#[async_trait]
impl ImageGenPort for PlaceholderImageGen {
    async fn generate(&self, request: ImageRequest) -> Result<ImageResult, ImageGenError> {
        self.call_count.fetch_add(1, Ordering::Relaxed);

        let placeholder_type = Self::placeholder_type_from_prompt(&request.prompt);
        let image_data = self.load_or_generate_placeholder(placeholder_type);

        Ok(ImageResult {
            image_data,
            format: "png".to_string(),
        })
    }

    async fn check_health(&self) -> Result<bool, ImageGenError> {
        // Always healthy - we're just returning placeholder data
        Ok(true)
    }
}

/// Recording mock that captures all generate requests for verification.
pub struct RecordingImageGen {
    inner: PlaceholderImageGen,
    requests: std::sync::Mutex<Vec<ImageRequest>>,
}

impl RecordingImageGen {
    pub fn new() -> Self {
        Self {
            inner: PlaceholderImageGen::new(),
            requests: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Get all recorded requests.
    pub fn requests(&self) -> Vec<ImageRequest> {
        self.requests.lock().unwrap().clone()
    }

    /// Get the last request, if any.
    pub fn last_request(&self) -> Option<ImageRequest> {
        self.requests.lock().unwrap().last().cloned()
    }

    /// Clear recorded requests.
    pub fn clear(&self) {
        self.requests.lock().unwrap().clear();
    }
}

impl Default for RecordingImageGen {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ImageGenPort for RecordingImageGen {
    async fn generate(&self, request: ImageRequest) -> Result<ImageResult, ImageGenError> {
        self.requests.lock().unwrap().push(request.clone());
        self.inner.generate(request).await
    }

    async fn check_health(&self) -> Result<bool, ImageGenError> {
        Ok(true)
    }
}

/// Failing mock for testing error handling.
pub struct FailingImageGen {
    error_message: String,
}

impl FailingImageGen {
    pub fn new(error_message: &str) -> Self {
        Self {
            error_message: error_message.to_string(),
        }
    }

    pub fn unavailable() -> Self {
        Self {
            error_message: "Service unavailable".to_string(),
        }
    }
}

#[async_trait]
impl ImageGenPort for FailingImageGen {
    async fn generate(&self, _request: ImageRequest) -> Result<ImageResult, ImageGenError> {
        Err(ImageGenError::GenerationFailed(self.error_message.clone()))
    }

    async fn check_health(&self) -> Result<bool, ImageGenError> {
        Err(ImageGenError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_placeholder_generates_png() {
        let gen = PlaceholderImageGen::new();
        let request = ImageRequest {
            prompt: "A character portrait".to_string(),
            workflow: "test".to_string(),
            width: 512,
            height: 512,
        };

        let result = gen.generate(request).await.unwrap();
        assert!(!result.image_data.is_empty());
        assert_eq!(result.format, "png");

        // Check PNG signature
        assert_eq!(
            &result.image_data[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    #[tokio::test]
    async fn test_placeholder_detects_portrait() {
        let gen = PlaceholderImageGen::new();

        // Portrait keywords
        for keyword in ["portrait", "character", "face", "headshot"] {
            let request = ImageRequest {
                prompt: format!("A {} of a warrior", keyword),
                workflow: "test".to_string(),
                width: 512,
                height: 512,
            };
            let result = gen.generate(request).await.unwrap();
            assert!(!result.image_data.is_empty());
        }
    }

    #[tokio::test]
    async fn test_placeholder_detects_scene() {
        let gen = PlaceholderImageGen::new();

        for keyword in ["scene", "action", "battle", "combat"] {
            let request = ImageRequest {
                prompt: format!("A {} in the forest", keyword),
                workflow: "test".to_string(),
                width: 1024,
                height: 768,
            };
            let result = gen.generate(request).await.unwrap();
            assert!(!result.image_data.is_empty());
        }
    }

    #[tokio::test]
    async fn test_recording_captures_requests() {
        let gen = RecordingImageGen::new();

        let request = ImageRequest {
            prompt: "Test prompt".to_string(),
            workflow: "test".to_string(),
            width: 256,
            height: 256,
        };

        gen.generate(request.clone()).await.unwrap();
        gen.generate(request.clone()).await.unwrap();

        let requests = gen.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].prompt, "Test prompt");
    }

    #[tokio::test]
    async fn test_failing_returns_error() {
        let gen = FailingImageGen::new("Custom error");

        let request = ImageRequest {
            prompt: "Any prompt".to_string(),
            workflow: "test".to_string(),
            width: 256,
            height: 256,
        };

        let result = gen.generate(request).await;
        assert!(result.is_err());

        if let Err(ImageGenError::GenerationFailed(msg)) = result {
            assert_eq!(msg, "Custom error");
        } else {
            panic!("Expected GenerationFailed error");
        }
    }

    #[tokio::test]
    async fn test_call_count() {
        let gen = PlaceholderImageGen::new();

        assert_eq!(gen.call_count(), 0);

        let request = ImageRequest {
            prompt: "Test".to_string(),
            workflow: "test".to_string(),
            width: 256,
            height: 256,
        };

        gen.generate(request.clone()).await.unwrap();
        assert_eq!(gen.call_count(), 1);

        gen.generate(request.clone()).await.unwrap();
        assert_eq!(gen.call_count(), 2);
    }
}
