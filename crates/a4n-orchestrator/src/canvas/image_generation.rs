mod contracts;
mod execution;
mod input;

use a4n_api_contract::canvas::CanvasRunSummary;
use a4n_canvas_contract::CanvasImageNodeApplicationInput;
use a4n_error::AppError;
use async_trait::async_trait;

pub use contracts::{CanvasImageNodeExecutionInput, CanvasImageNodeInput};
pub use input::build_canvas_image_node_input_for_preview;

#[async_trait]
pub trait CanvasImageNodeApplicationPort: Send + Sync {
    async fn run_image(
        &self,
        input: CanvasImageNodeApplicationInput,
        host_port: &dyn crate::media::ImageGenerationHostPort,
        progress: &crate::media::ImageGenerationProgress<'_>,
    ) -> Result<CanvasRunSummary, AppError>;
}

#[cfg(test)]
mod tests;
