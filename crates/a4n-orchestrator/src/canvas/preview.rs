use a4n_canvas_contract::{
    CanvasPreviewNodeApplicationInput, GenerateCanvasImageInput, GenerateCanvasVideoInput,
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum CanvasNodeRequestPreview {
    Image(GenerateCanvasImageInput),
    Video(GenerateCanvasVideoInput),
    Llm(ChatRequest),
}

#[async_trait]
pub trait CanvasPreviewApplicationPort: Send + Sync {
    async fn prepare_node(
        &self,
        input: CanvasPreviewNodeApplicationInput,
    ) -> Result<CanvasNodeRequestPreview, AppError>;
}
