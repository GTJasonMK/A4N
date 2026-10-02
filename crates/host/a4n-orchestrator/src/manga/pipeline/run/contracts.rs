use std::collections::HashMap;

use a4n_api_contract::manga::MangaPagePromptSourceSummary;
use a4n_error::AppError;
use a4n_progress::{ProgressPhaseKind, ProgressSeverity};
use a4n_service_protocol::ProgressEvent;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::media_authoring::PreparedAdaptation;
use a4n_domain::source_spans::SourceSpan;
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::{MangaPipelineStage, OrchestratorContext};
use super::super::state::{MANGA_PIPELINE_TOTAL_STEPS, manga_pipeline_stage_status};

pub(super) struct MangaRunContext<'a> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) chapter_id: i64,
    pub(super) chapter_number: i32,
    pub(super) project_id: i64,
    pub(super) user_id: i64,
    pub(super) adaptation: PreparedAdaptation,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) task_id: &'a str,
    pub(super) workflow_run_id: i64,
    pub(super) resume_from_checkpoint: bool,
    pub(super) stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
    pub(super) image_size: Option<a4n_domain::ImageSize>,
    pub(super) image_generator:
        Option<&'a dyn a4n_api_contract::manga::MangaPipelinePageImageGenerator>,
    pub(super) cancel_token: Option<&'a CancellationToken>,
    pub(super) progress_reporter: Option<&'a (dyn Fn(ProgressEvent) + Send + Sync)>,
    pub(super) stream_reporter: Option<&'a (dyn Fn(&str, &str) + Send + Sync)>,
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
        let (stage_code, next_current, next_message) = manga_pipeline_stage_status(next_stage);
        self.service
            .record_manga_pipeline_progress(
                self.workflow_run_id,
                self.task_id,
                stage_code,
                next_current,
                next_message,
            )
            .await?;
        self.publish_progress(
            stage_code,
            ProgressPhaseKind::StructuredOutput,
            next_current,
            MANGA_PIPELINE_TOTAL_STEPS,
            next_message,
            "阶段",
            None,
            None,
            None,
        );
        Ok(())
    }

    pub(super) fn publish_progress(
        &self,
        stage_code: &str,
        phase_kind: ProgressPhaseKind,
        current: i32,
        total: i32,
        message: &str,
        unit_label: &str,
        item_key: Option<String>,
        item_index: Option<u64>,
        item_total: Option<u64>,
    ) {
        let Some(reporter) = self.progress_reporter else {
            return;
        };
        let Ok(current) = u64::try_from(current) else {
            return;
        };
        let Ok(total) = u64::try_from(total) else {
            return;
        };
        reporter(ProgressEvent {
            stage_code: format!("manga_{stage_code}"),
            phase_kind,
            current: Some(current),
            total: Some(total.max(1)),
            message: message.to_string(),
            unit_label: Some(unit_label.to_string()),
            item_key,
            item_index,
            item_total,
            severity: ProgressSeverity::Info,
            estimated: false,
        });
    }

    pub(super) fn check_cancelled(&self) -> Result<(), AppError> {
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(AppError::Cancelled);
        }
        Ok(())
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
