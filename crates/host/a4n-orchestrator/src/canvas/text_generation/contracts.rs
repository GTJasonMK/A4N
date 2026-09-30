use crate::canvas::CanvasNodeRunHandle;
use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_domain::canvas::CanvasNodeRecord;

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
    pub node: &'a CanvasNodeRecord,
    pub document: &'a CanvasGraphRecordDocument,
    pub shared_run: Option<CanvasNodeRunHandle>,
    pub mode: &'a str,
}

pub(crate) use a4n_canvas_host_contract::{
    CanvasTextGenerationLlmCall, CanvasTextGenerationLlmPort,
};
