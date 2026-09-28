use std::collections::HashMap;

use a4n_api_contract::manga::MangaPagePromptSourceSummary;
use a4n_error::AppError;
use serde_json::Value;

use crate::media_authoring::PreparedAdaptation;
use a4n_domain::manga::MangaPagePlanV4;
use a4n_domain::source_spans::SourceSpan;
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::{MangaPipelineStage, OrchestratorContext};
use super::super::state::manga_pipeline_stage_status;

pub(super) struct MangaRunContext<'a> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) chapter_id: i64,
    pub(super) project_id: i64,
    pub(super) adaptation: PreparedAdaptation,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) task_id: &'a str,
    pub(super) workflow_run_id: i64,
    pub(super) resume_from_checkpoint: bool,
    pub(super) stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
    pub(super) chapter_content: String,
    pub(super) source_spans: Vec<SourceSpan>,
    pub(super) variables: HashMap<String, Value>,
    pub(super) reference_assets: Value,
    pub(super) source_metadata: MangaPagePromptSourceSummary,
    pub(super) reference_assets_metadata: Value,
    pub(super) prompt_fingerprint: Value,
}

impl MangaRunContext<'_> {
    pub(super) async fn advance_stage(
        &self,
        next_stage: MangaPipelineStage,
    ) -> Result<(), AppError> {
        let (next_stage, next_current, next_message) = manga_pipeline_stage_status(next_stage);
        self.service
            .record_manga_pipeline_progress(
                self.workflow_run_id,
                self.task_id,
                next_stage,
                next_current,
                next_message,
            )
            .await
    }
}

pub(super) struct MangaEarlyRun {
    pub(super) events_json: Value,
    pub(super) characters_json: Value,
    pub(super) speech_json: Value,
}

pub(super) struct MangaExtractionRun {
    pub(super) extraction_summary: Value,
}

pub(super) struct MangaPlanningRun {
    pub(super) extraction: MangaExtractionRun,
    pub(super) plan: MangaPagePlanV4,
    pub(super) page_plan_ref: a4n_domain::media_authoring::AuthoringDocumentRef,
}
