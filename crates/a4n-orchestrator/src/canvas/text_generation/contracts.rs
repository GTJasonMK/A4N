use a4n_core::{error::AppError, models::canvas::canvas_node};
use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_llm::provider::{ChatRequest, ChatResponse};
use async_trait::async_trait;

use crate::canvas::CanvasNodeRunHandle;

#[derive(Debug, Clone)]
pub(super) struct CanvasRenderedPrompt {
    pub prompt_kind: String,
    pub prompt_text: String,
    pub canvas_text_asset_id: Option<i64>,
    pub source_node_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub(super) struct CanvasLlmTextInput {
    pub system_prompt: String,
    pub user_prompt: String,
    pub instruction: String,
    pub source_context: String,
    pub output_kind: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<usize>,
    pub config_id: Option<i64>,
    pub canvas_text_asset_id: Option<i64>,
    pub source_node_ids: Vec<i64>,
}

pub struct CanvasTextNodeExecutionInput<'a> {
    pub project_id: i64,
    pub graph_id: i64,
    pub node: &'a canvas_node::Model,
    pub document: &'a CanvasGraphRecordDocument,
    pub shared_run: Option<CanvasNodeRunHandle>,
    pub mode: &'a str,
}

#[derive(Debug)]
pub struct CanvasTextGenerationLlmCall {
    pub project_id: i64,
    pub workflow_run_id: i64,
    pub config_id: Option<i64>,
    pub request: ChatRequest,
}

#[async_trait]
pub trait CanvasTextGenerationLlmPort: Send + Sync {
    /// 通过统一 stream 收集完整文本结果（禁止完整响应 chat）。
    async fn stream_complete(
        &self,
        input: CanvasTextGenerationLlmCall,
    ) -> Result<ChatResponse, AppError>;
}
