//! ComfyUI workflow-backed image generation E2E tests.
//!
//! These tests validate the ComfyUI image generation integration using real workflows:
//! - `backdrop_v1`: Landscape-oriented backdrop generation (1024x768)
//! - `portrait_v1`: Portrait-oriented character generation (768x1024)
//!
//! # Prerequisites
//!
//! ComfyUI must be running locally at `http://127.0.0.1:8188` (or the URL specified
//! by the `COMFYUI_URL` environment variable).
//!
//! The following model file must be available in ComfyUI's checkpoints directory:
//! - `bridgeToonsComicMix_v60.safetensors`
//!
//! # Running Tests
//!
//! ```bash
//! # Run with local ComfyUI instance
//! COMFYUI_E2E=1 cargo test -p wrldbldr-engine --lib comfyui -- --ignored --test-threads=1
//!
//! # Run with custom ComfyUI URL
//! COMFYUI_E2E=1 COMFYUI_URL=http://192.168.1.100:8188 cargo test -p wrldbldr-engine --lib comfyui -- --ignored
//! ```
//!
//! # Environment Variables
//!
//! - `COMFYUI_E2E=1`: Required to enable e2e tests (tests are skipped otherwise)
//! - `COMFYUI_URL`: Optional URL override (default: `http://127.0.0.1:8188`)
//! - `COMFYUI_POLL_TIMEOUT_SECS`: Optional poll timeout in seconds (default: 120)

use std::sync::Once;

use crate::infrastructure::comfyui::ComfyUIClient;
use crate::infrastructure::ports::{ImageGenPort, ImageRequest};

/// Available checkpoint for model override tests.
const AVAILABLE_CHECKPOINT: &str = "bridgeToonsComicMix_v60.safetensors";

/// Default poll timeout for e2e tests (5 minutes).
const E2E_POLL_TIMEOUT_SECS: &str = "300";

static INIT: Once = Once::new();

/// Initialize e2e test environment.
///
/// Sets up environment variables needed for e2e tests:
/// - Increases poll timeout to accommodate slower generation times
fn init_e2e_env() {
    INIT.call_once(|| {
        std::env::set_var("COMFYUI_POLL_TIMEOUT_SECS", E2E_POLL_TIMEOUT_SECS);
    });
}

/// Check if ComfyUI e2e tests should run.
///
/// Tests are gated by the `COMFYUI_E2E` environment variable.
/// Returns `true` only if `COMFYUI_E2E=1` is set.
fn should_run_comfyui_e2e() -> bool {
    std::env::var("COMFYUI_E2E").unwrap_or_default() == "1"
}

/// Setup function called at the start of each e2e test.
///
/// Initializes the test environment if needed.
fn setup_e2e_test() {
    init_e2e_env();
}

/// Get the ComfyUI base URL from environment or use default.
///
/// Default: `http://127.0.0.1:8188`
fn get_comfyui_url() -> String {
    std::env::var("COMFYUI_URL").unwrap_or_else(|_| "http://127.0.0.1:8188".to_string())
}

/// Skip macro for ComfyUI e2e tests.
///
/// Returns a test result that indicates the test was skipped.
/// Also initializes the e2e test environment (sets up timeout, etc.).
macro_rules! skip_if_not_enabled {
    () => {
        setup_e2e_test();
        if !should_run_comfyui_e2e() {
            eprintln!("Skipping ComfyUI e2e test. Set COMFYUI_E2E=1 to enable.");
            return;
        }
    };
}

// =============================================================================
// Backdrop Generation Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires running ComfyUI instance (set COMFYUI_E2E=1 to run)"]
async fn comfyui_backdrop_v1_generates_non_empty_png() {
    skip_if_not_enabled!();

    let base_url = get_comfyui_url();
    let client = ComfyUIClient::new(&base_url);

    // Verify ComfyUI is reachable
    let health = client
        .check_health()
        .await
        .expect("Failed to check ComfyUI health");
    assert!(health, "ComfyUI is not healthy at {}", base_url);

    // Generate a backdrop using the backdrop_v1 workflow
    let request = ImageRequest::new(
        "A cozy medieval tavern interior at dusk, warm candlelight, wooden beams",
        "backdrop_v1",
        0,  // Use workflow default (1024)
        0, // Use workflow default (768)
    );

    let result = client
        .generate(request)
        .await
        .expect("Image generation failed");

    // Assert image data is non-empty
    assert!(
        !result.image_data.is_empty(),
        "Generated image data should not be empty"
    );
    assert!(
        result.image_data.len() > 1000,
        "Generated image should be larger than 1KB (got {} bytes)",
        result.image_data.len()
    );

    // Assert format is png or jpeg
    assert!(
        result.format == "png" || result.format == "jpeg",
        "Expected format 'png' or 'jpeg', got '{}'",
        result.format
    );

    // Verify PNG magic bytes if it's a PNG
    if result.format == "png" {
        assert_eq!(
            &result.image_data[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "PNG file should have correct magic bytes"
        );
    }

    eprintln!(
        "Successfully generated backdrop: {} bytes, format: {}",
        result.image_data.len(),
        result.format
    );
}

#[tokio::test]
#[ignore = "requires running ComfyUI instance (set COMFYUI_E2E=1 to run)"]
async fn comfyui_backdrop_v1_with_custom_dimensions() {
    skip_if_not_enabled!();

    let base_url = get_comfyui_url();
    let client = ComfyUIClient::new(&base_url);

    // Verify ComfyUI is reachable
    let health = client
        .check_health()
        .await
        .expect("Failed to check ComfyUI health");
    assert!(health, "ComfyUI is not healthy at {}", base_url);

    // Generate a backdrop with custom dimensions
    let request = ImageRequest::new(
        "A misty forest path at dawn, tall trees, atmospheric fog",
        "backdrop_v1",
        512,  // Custom width
        384, // Custom height
    );

    let result = client
        .generate(request)
        .await
        .expect("Image generation with custom dimensions failed");

    // Assert image data is non-empty
    assert!(
        !result.image_data.is_empty(),
        "Generated image data should not be empty"
    );
    assert!(
        result.image_data.len() > 1000,
        "Generated image should be larger than 1KB (got {} bytes)",
        result.image_data.len()
    );

    // Assert format
    assert!(
        result.format == "png" || result.format == "jpeg",
        "Expected format 'png' or 'jpeg', got '{}'",
        result.format
    );

    eprintln!(
        "Successfully generated custom backdrop: {} bytes, format: {}",
        result.image_data.len(),
        result.format
    );
}

// =============================================================================
// Portrait Generation Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires running ComfyUI instance (set COMFYUI_E2E=1 to run)"]
async fn comfyui_portrait_v1_generates_non_empty_png() {
    skip_if_not_enabled!();

    let base_url = get_comfyui_url();
    let client = ComfyUIClient::new(&base_url);

    // Verify ComfyUI is reachable
    let health = client
        .check_health()
        .await
        .expect("Failed to check ComfyUI health");
    assert!(health, "ComfyUI is not healthy at {}", base_url);

    // Generate a portrait using the portrait_v1 workflow
    let request = ImageRequest::new(
        "A wise elderly wizard with a long white beard, wearing purple robes",
        "portrait_v1",
        0,  // Use workflow default (768)
        0, // Use workflow default (1024)
    );

    let result = client
        .generate(request)
        .await
        .expect("Portrait generation failed");

    // Assert image data is non-empty
    assert!(
        !result.image_data.is_empty(),
        "Generated portrait data should not be empty"
    );
    assert!(
        result.image_data.len() > 1000,
        "Generated portrait should be larger than 1KB (got {} bytes)",
        result.image_data.len()
    );

    // Assert format is png or jpeg
    assert!(
        result.format == "png" || result.format == "jpeg",
        "Expected format 'png' or 'jpeg', got '{}'",
        result.format
    );

    // Verify PNG magic bytes if it's a PNG
    if result.format == "png" {
        assert_eq!(
            &result.image_data[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "PNG file should have correct magic bytes"
        );
    }

    eprintln!(
        "Successfully generated portrait: {} bytes, format: {}",
        result.image_data.len(),
        result.format
    );
}

#[tokio::test]
#[ignore = "requires running ComfyUI instance (set COMFYUI_E2E=1 to run)"]
async fn comfyui_portrait_v1_with_score_tokens_in_prompt() {
    skip_if_not_enabled!();

    let base_url = get_comfyui_url();
    let client = ComfyUIClient::new(&base_url);

    // Verify ComfyUI is reachable
    let health = client
        .check_health()
        .await
        .expect("Failed to check ComfyUI health");
    assert!(health, "ComfyUI is not healthy at {}", base_url);

    // Generate a portrait with SCORE tokens already in the prompt
    // The client should not duplicate them
    let request = ImageRequest::new(
        "SCORE_9, SCORE_8, a fierce female warrior with red hair and leather armor",
        "portrait_v1",
        0,
        0,
    );

    let result = client
        .generate(request)
        .await
        .expect("Portrait generation with SCORE tokens failed");

    // Assert image data is non-empty
    assert!(
        !result.image_data.is_empty(),
        "Generated portrait data should not be empty"
    );
    assert!(
        result.image_data.len() > 1000,
        "Generated portrait should be larger than 1KB (got {} bytes)",
        result.image_data.len()
    );

    eprintln!(
        "Successfully generated portrait with SCORE tokens: {} bytes, format: {}",
        result.image_data.len(),
        result.format
    );
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires running ComfyUI instance (set COMFYUI_E2E=1 to run)"]
async fn comfyui_unknown_workflow_returns_error() {
    skip_if_not_enabled!();

    let base_url = get_comfyui_url();
    let client = ComfyUIClient::new(&base_url);

    // Verify ComfyUI is reachable
    let health = client
        .check_health()
        .await
        .expect("Failed to check ComfyUI health");
    assert!(health, "ComfyUI is not healthy at {}", base_url);

    // Try to generate with an unknown workflow
    let request = ImageRequest::new(
        "A fantasy landscape",
        "nonexistent_workflow",
        0,
        0,
    );

    let result = client.generate(request).await;

    // Should return an error
    assert!(
        result.is_err(),
        "Unknown workflow should return an error"
    );

    let err = result.unwrap_err();
    let err_string = format!("{}", err);
    assert!(
        err_string.contains("Unknown workflow ID"),
        "Error should mention unknown workflow ID, got: {}",
        err_string
    );

    eprintln!("Correctly rejected unknown workflow with error: {}", err_string);
}

// =============================================================================
// Health Check Tests
// =============================================================================

#[tokio::test]
#[ignore = "requires running ComfyUI instance (set COMFYUI_E2E=1 to run)"]
async fn comfyui_health_check_returns_true_when_healthy() {
    skip_if_not_enabled!();

    let base_url = get_comfyui_url();
    let client = ComfyUIClient::new(&base_url);

    let health = client
        .check_health()
        .await
        .expect("Health check should not fail");

    assert!(health, "ComfyUI should be healthy at {}", base_url);
}

#[tokio::test]
async fn comfyui_health_check_returns_false_for_invalid_url() {
    // This test doesn't require COMFYUI_E2E=1 - it tests error handling
    // Use an invalid URL that should not have ComfyUI running
    let client = ComfyUIClient::new("http://127.0.0.1:59999");

    let health_result = client.check_health().await;

    // For invalid URLs, check_health returns Err(ImageGenError::Unavailable)
    // because the connection fails. This is the expected behavior.
    match health_result {
        Ok(healthy) => assert!(!healthy, "Health check should return false for invalid URL"),
        Err(_) => {
            // Connection errors are expected for invalid URLs - this is acceptable
            // as it indicates the service is not available
        }
    }
}

// =============================================================================
// New ImageRequest Builder Tests (Per-Workflow Settings)
// =============================================================================

#[tokio::test]
#[ignore = "requires running ComfyUI instance (set COMFYUI_E2E=1 to run)"]
async fn comfyui_portrait_v1_with_model_override() {
    skip_if_not_enabled!();

    let base_url = get_comfyui_url();
    let client = ComfyUIClient::new(&base_url);

    // Verify ComfyUI is reachable
    let health = client
        .check_health()
        .await
        .expect("Failed to check ComfyUI health");
    assert!(health, "ComfyUI is not healthy at {}", base_url);

    // Generate a portrait with model override using builder pattern
    let request = ImageRequest::new(
        "A wise elderly wizard with a long white beard",
        "portrait_v1",
        0,
        0,
    )
    .with_model_id(AVAILABLE_CHECKPOINT)
    .with_negative_prompt("blurry, low quality")
    .with_prompt_prefix("masterpiece, best quality")
    .with_prompt_suffix("detailed, 8k")
    .with_score_tokens_enabled(true);

    let result = client
        .generate(request)
        .await
        .expect("Portrait generation with model override failed");

    // Assert image data is non-empty
    assert!(
        !result.image_data.is_empty(),
        "Generated portrait data should not be empty"
    );
    assert!(
        result.image_data.len() > 1000,
        "Generated portrait should be larger than 1KB (got {} bytes)",
        result.image_data.len()
    );

    eprintln!(
        "Successfully generated portrait with model override: {} bytes, format: {}",
        result.image_data.len(),
        result.format
    );
}

#[tokio::test]
#[ignore = "requires running ComfyUI instance (set COMFYUI_E2E=1 to run)"]
async fn comfyui_backdrop_v1_with_full_settings() {
    skip_if_not_enabled!();

    let base_url = get_comfyui_url();
    let client = ComfyUIClient::new(&base_url);

    // Verify ComfyUI is reachable
    let health = client
        .check_health()
        .await
        .expect("Failed to check ComfyUI health");
    assert!(health, "ComfyUI is not healthy at {}", base_url);

    // Generate a backdrop with all new settings using builder pattern
    let request = ImageRequest::new(
        "A mystical castle on a hill at sunset",
        "backdrop_v1",
        1024,
        768,
    )
    .with_model_id(AVAILABLE_CHECKPOINT)
    .with_negative_prompt("people, text, watermark")
    .with_prompt_prefix("fantasy art, digital painting")
    .with_prompt_suffix("highly detailed, cinematic lighting")
    .with_score_tokens_enabled(true)
    .with_pose_asset_id("castle_pose_001")
    .with_controlnet_mappings(vec![
        crate::infrastructure::ports::types::ControlNetMapping {
            node_id: "15".to_string(),
            input_key: "image".to_string(),
            enabled: false, // Disabled for this test since we don't have actual pose files
        },
    ]);

    let result = client
        .generate(request)
        .await
        .expect("Backdrop generation with full settings failed");

    // Assert image data is non-empty
    assert!(
        !result.image_data.is_empty(),
        "Generated backdrop data should not be empty"
    );
    assert!(
        result.image_data.len() > 1000,
        "Generated backdrop should be larger than 1KB (got {} bytes)",
        result.image_data.len()
    );

    eprintln!(
        "Successfully generated backdrop with full settings: {} bytes, format: {}",
        result.image_data.len(),
        result.format
    );
}

#[test]
fn image_request_builder_creates_correct_request() {
    // Unit test for the builder pattern (no ComfyUI required)
    let request = ImageRequest::new("test prompt", "test_workflow", 512, 768)
        .with_model_id("model.safetensors")
        .with_negative_prompt("negative prompt")
        .with_prompt_prefix("prefix")
        .with_prompt_suffix("suffix")
        .with_score_tokens_enabled(false)
        .with_pose_asset_id("pose_123")
        .with_controlnet_mappings(vec![crate::infrastructure::ports::types::ControlNetMapping {
            node_id: "node_1".to_string(),
            input_key: "image".to_string(),
            enabled: true,
        }]);

    assert_eq!(request.prompt, "test prompt");
    assert_eq!(request.workflow, "test_workflow");
    assert_eq!(request.width, 512);
    assert_eq!(request.height, 768);
    assert_eq!(request.model_id, Some("model.safetensors".to_string()));
    assert_eq!(request.negative_prompt, Some("negative prompt".to_string()));
    assert_eq!(request.prompt_prefix, Some("prefix".to_string()));
    assert_eq!(request.prompt_suffix, Some("suffix".to_string()));
    assert_eq!(request.score_tokens_enabled, false);
    assert_eq!(request.pose_asset_id, Some("pose_123".to_string()));
    assert!(request.controlnet_mappings.is_some());
    assert_eq!(request.controlnet_mappings.as_ref().unwrap().len(), 1);
}

#[test]
fn image_request_effective_prompt_combines_prefix_and_suffix() {
    let request = ImageRequest::new("main prompt", "workflow", 0, 0)
        .with_prompt_prefix("prefix")
        .with_prompt_suffix("suffix");

    let effective = request.effective_prompt();
    assert_eq!(effective, "prefix, main prompt, suffix");
}

#[test]
fn image_request_effective_prompt_without_prefix_suffix() {
    let request = ImageRequest::new("main prompt", "workflow", 0, 0);

    let effective = request.effective_prompt();
    assert_eq!(effective, "main prompt");
}

#[test]
fn image_request_effective_prompt_with_only_prefix() {
    let request = ImageRequest::new("main prompt", "workflow", 0, 0)
        .with_prompt_prefix("prefix");

    let effective = request.effective_prompt();
    assert_eq!(effective, "prefix, main prompt");
}

#[test]
fn image_request_effective_prompt_with_only_suffix() {
    let request = ImageRequest::new("main prompt", "workflow", 0, 0)
        .with_prompt_suffix("suffix");

    let effective = request.effective_prompt();
    assert_eq!(effective, "main prompt, suffix");
}
