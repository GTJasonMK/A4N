mod execution;
mod orchestration;
mod request;

use a4n_progress::ProgressSpan;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use super::{progress::SceneProfileSyncProgress, types::SceneProfileSyncRequestContext};
use crate::OrchestratorContext;

pub(super) use orchestration::run_scene_profile_sync_stages;
pub(super) use request::build_scene_profile_candidates_request;
#[cfg(test)]
pub(super) use request::build_scene_profile_stage_request;

pub(super) struct SceneProfileStageContext<'a, 'progress>
where
    'progress: 'a,
{
    pub(super) state: &'a OrchestratorContext,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) workflow_run_id: i64,
    pub(super) request_context: &'a SceneProfileSyncRequestContext,
    pub(super) progress: Option<&'a SceneProfileSyncProgress<'progress>>,
    pub(super) model_span: ProgressSpan,
    pub(super) chapter_index: usize,
    pub(super) response_chars: Vec<Value>,
}
