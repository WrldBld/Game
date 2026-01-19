// Expression sheet generation - methods for future sprite sheet features
#![allow(dead_code)]

//! Expression sheet generation use case.
//!
//! Handles generating expression sprite sheets for characters and slicing
//! them into individual expression sprites.
//!
//! ## Workflow
//!
//! 1. Queue an expression sheet generation request
//! 2. ComfyUI generates a grid of expressions (e.g., 4x4 = 16 expressions)
//! 3. Post-process: slice the grid into individual sprites
//! 4. Save each sprite with expression name in filename
//!
//! ## Expression Grid Layout
//!
//! The default layout is a 4x4 grid with expressions in this order:
//! ```text
//! | neutral   | happy     | sad       | angry     |
//! | surprised | afraid    | thoughtful| suspicious|
//! | curious   | confused  | worried   | excited   |
//! | confident | nervous   | amused    | calm      |
//! ```

use std::sync::Arc;
use uuid::Uuid;
use wrldbldr_domain::{AssetId, CharacterId};

use crate::infrastructure::ports::{
    AssetRepo, CharacterRepo, ClockPort, ImageGenPort, QueueError, QueuePort, RepoError,
};

// Type aliases for old names to maintain compatibility
type CharacterRepository = dyn CharacterRepo;
type QueueService = dyn QueuePort;
type ClockService = dyn ClockPort;

/// Standard expression order in a 4x4 grid
pub const STANDARD_EXPRESSION_ORDER: [&str; 16] = [
    "neutral",
    "happy",
    "sad",
    "angry",
    "surprised",
    "afraid",
    "thoughtful",
    "suspicious",
    "curious",
    "confused",
    "worried",
    "excited",
    "confident",
    "nervous",
    "amused",
    "calm",
];

/// Request for expression sheet generation
#[derive(Debug, Clone)]
pub struct ExpressionSheetRequest {
    /// Character to generate expressions for
    pub character_id: CharacterId,
    /// Source asset to use as base (e.g., character's portrait)
    /// If None, uses the character's current portrait_asset
    pub source_asset_id: Option<AssetId>,
    /// Expressions to generate (defaults to STANDARD_EXPRESSION_ORDER)
    pub expressions: Vec<String>,
    /// Grid layout: (columns, rows)
    /// Default is (4, 4) for 16 expressions
    pub grid_layout: (u32, u32),
    /// ComfyUI workflow to use for generation
    pub workflow: String,
    /// Additional prompt text for style guidance
    pub style_prompt: Option<String>,
}

impl ExpressionSheetRequest {
    /// Create a request with standard expressions
    pub fn standard(character_id: CharacterId, workflow: String) -> Self {
        Self {
            character_id,
            source_asset_id: None,
            expressions: STANDARD_EXPRESSION_ORDER
                .iter()
                .map(|s| s.to_string())
                .collect(),
            grid_layout: (4, 4),
            workflow,
            style_prompt: None,
        }
    }

    /// Create a request with custom expressions
    pub fn custom(character_id: CharacterId, expressions: Vec<String>, workflow: String) -> Self {
        let count = expressions.len();
        // Calculate grid layout (prefer square)
        let cols = (count as f32).sqrt().ceil() as u32;
        let rows = ((count as f32) / cols as f32).ceil() as u32;

        Self {
            character_id,
            source_asset_id: None,
            expressions,
            grid_layout: (cols, rows),
            workflow,
            style_prompt: None,
        }
    }
}

/// Result of expression sheet generation
#[derive(Debug, Clone)]
pub struct ExpressionSheetResult {
    /// Batch ID for tracking generation progress
    pub batch_id: Uuid,
    /// Character ID
    pub character_id: CharacterId,
    /// Expressions that will be generated
    pub expressions: Vec<String>,
}

/// Result of slicing an expression sheet
#[derive(Debug, Clone)]
pub struct SlicedExpression {
    /// Expression name (e.g., "happy", "sad")
    pub expression: String,
    /// Asset ID for the sliced sprite
    pub asset_id: AssetId,
    /// File path to the sprite
    pub file_path: String,
}

/// Generate expression sheet use case.
#[allow(dead_code)]
pub struct GenerateExpressionSheet {
    asset_repo: Arc<dyn AssetRepo>,
    image_gen: Arc<dyn ImageGenPort>,
    character: Arc<CharacterRepository>,
    queue: Arc<QueueService>,
    clock: Arc<ClockService>,
}

impl GenerateExpressionSheet {
    pub fn new(
        asset_repo: Arc<dyn AssetRepo>,
        image_gen: Arc<dyn ImageGenPort>,
        character: Arc<CharacterRepository>,
        queue: Arc<QueueService>,
        clock: Arc<ClockService>,
    ) -> Self {
        Self {
            asset_repo,
            image_gen,
            character,
            queue,
            clock,
        }
    }

    /// Queue an expression sheet generation request.
    ///
    /// This returns immediately with a batch ID that can be used to track progress.
    /// When generation completes, the sheet will be sliced into individual expressions.
    pub async fn queue(
        &self,
        request: ExpressionSheetRequest,
    ) -> Result<ExpressionSheetResult, ExpressionSheetError> {
        // Get the character to validate it exists and get their name
        let character = self
            .character
            .get(request.character_id)
            .await?
            .ok_or(ExpressionSheetError::CharacterNotFound)?;

        // Build the generation prompt
        let expressions_list = request.expressions.join(", ");
        let base_prompt = format!(
            "Expression sheet for character '{}'. Grid layout {}x{}. Expressions: {}",
            character.name(),
            request.grid_layout.0,
            request.grid_layout.1,
            expressions_list
        );

        let prompt = match request.style_prompt {
            Some(style) => format!("{} Style: {}", base_prompt, style),
            None => base_prompt,
        };

        // Queue the generation
        let batch_id = self
            .queue
            .enqueue_asset_generation(&crate::queue_types::AssetGenerationData {
                world_id: Some(character.world_id()),
                entity_type: "character".to_string(),
                entity_id: request.character_id.to_string(),
                workflow_id: request.workflow,
                prompt,
                count: 1, // Expression sheet is a single image
            })
            .await?;

        Ok(ExpressionSheetResult {
            batch_id,
            character_id: request.character_id,
            expressions: request.expressions,
        })
    }

    /// Slice a generated expression sheet into individual sprites.
    ///
    /// This is called after the generation completes. It takes the generated
    /// grid image and slices it into individual expression sprites.
    ///
    /// # Arguments
    /// * `sheet_data` - The raw image data of the expression sheet
    /// * `expressions` - List of expression names in grid order (left-to-right, top-to-bottom)
    /// * `grid_layout` - (columns, rows) of the grid
    /// * `character_id` - Character to save sprites for
    ///
    /// # Returns
    /// List of sliced expressions with their asset IDs
    pub async fn slice_sheet(
        &self,
        _sheet_data: &[u8],
        expressions: &[String],
        _grid_layout: (u32, u32),
        character_id: CharacterId,
    ) -> Result<Vec<SlicedExpression>, ExpressionSheetError> {
        // TODO: Implement actual image slicing using the `image` crate.
        //
        // IMPORTANT: Do not return placeholder assets or mutate character state here.
        // Returning “fake” asset IDs/paths makes downstream systems think sprites exist
        // when they don't.

        let now = self.clock.now();
        tracing::warn!(
            character_id = %character_id,
            expression_count = expressions.len(),
            now = ?now,
            "Expression sheet slicing is not implemented"
        );

        Err(ExpressionSheetError::SliceFailed(
            "Expression sheet slicing is not implemented yet".to_string(),
        ))
    }
}

/// Errors that can occur during expression sheet generation
#[derive(Debug, thiserror::Error)]
pub enum ExpressionSheetError {
    #[error("Character not found")]
    CharacterNotFound,

    #[error("Failed to queue generation: {0}")]
    Queue(#[from] QueueError),

    #[error("Failed to slice image: {0}")]
    SliceFailed(String),

    #[error("Failed to save asset: {0}")]
    SaveFailed(String),

    #[error("Repository error: {0}")]
    RepoError(#[from] RepoError),
}
