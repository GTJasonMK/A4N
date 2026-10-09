use std::collections::HashMap;

use a4n_domain::{
    manga::{
        MangaPipelineArtifactDraft, MangaPipelineArtifactStage, MangaPreprocessingV3,
        manga_preprocessing_input_fingerprint,
    },
    source_spans::source_spans_for_content,
    source_text_hash,
};
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::ProgressEvent;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{OrchestratorContext, source_markers::SourceMarkerMap};

use super::super::prompt::{
    MangaRenderTraceInput, manga_preprocessing_user_message, validate_manga_preprocessing,
};

pub(super) struct MangaPreprocessingInput<'a> {
    pub project_id: i64,
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub chapter_content: &'a str,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub reference_assets: Value,
    pub task_id: &'a str,
    pub workflow_run_id: i64,
    pub resume_from_checkpoint: bool,
    pub cancel_token: Option<&'a CancellationToken>,
    pub progress_reporter: Option<&'a (dyn Fn(ProgressEvent) + Send + Sync)>,
    pub stream_reporter: Option<&'a (dyn Fn(&str, &str) + Send + Sync)>,
}

pub(super) async fn run_manga_preprocessing(
    context: &OrchestratorContext,
    input: MangaPreprocessingInput<'_>,
) -> Result<MangaPreprocessingV3, AppError> {
    let source_spans = source_spans_for_content(
        input.chapter_id,
        input.chapter_number,
        input.chapter_content,
    );
    let source_markers = SourceMarkerMap::from_source_spans(&source_spans)?;
    let input_fingerprint = manga_preprocessing_input_fingerprint(
        &source_text_hash(input.chapter_content),
        input.chapter_number,
        &input.reference_assets,
    );
    let user_content = manga_preprocessing_user_message(
        input.chapter_content,
        &source_spans,
        &source_markers,
        input.reference_assets,
    )?;

    context
        .record_manga_pipeline_progress_with_total(
            input.workflow_run_id,
            input.task_id,
            "preprocessing",
            0,
            1,
            "正在提取漫画改编前章节资料",
        )
        .await?;
    if let Some(reporter) = input.progress_reporter {
        reporter(ProgressEvent {
            stage_code: "manga_preprocessing".into(),
            phase_kind: a4n_progress::ProgressPhaseKind::StructuredOutput,
            current: Some(0),
            total: Some(1),
            message: "正在提取漫画改编前章节资料".into(),
            unit_label: Some("阶段".into()),
            item_key: None,
            item_index: None,
            item_total: None,
            severity: a4n_progress::ProgressSeverity::Info,
            estimated: false,
        });
    }

    let (_, preprocessing) = context
        .render_and_validate_manga_call_with_markers(
            MangaRenderTraceInput {
                prompt_renderer: input.prompt_renderer,
                prompt_name: "manga_preprocessing",
                project_id: input.project_id,
                workflow_run_id: input.workflow_run_id,
                call_key: format!("manga_pipeline:{}:manga_preprocessing", input.chapter_id),
                call_order: 0,
                restore_from_checkpoint: input.resume_from_checkpoint,
                variables: &HashMap::new(),
                user_content,
                temperature: 0.3,
                cancel_token: input.cancel_token,
                stream_id: "raw_json".into(),
                stream_reporter: input.stream_reporter,
            },
            &source_markers,
            validate_manga_preprocessing,
        )
        .await?;

    context
        .upsert_manga_pipeline_artifact(MangaPipelineArtifactDraft {
            project_id: input.project_id,
            chapter_id: input.chapter_id,
            stage: MangaPipelineArtifactStage::Preprocessing,
            page_number: None,
            input_fingerprint,
            payload: serde_json::to_value(&preprocessing)?,
            stale: false,
        })
        .await?;
    context
        .report_manga_preprocessing_completed(
            input.workflow_run_id,
            input.task_id,
            input.progress_reporter,
        )
        .await?;
    Ok(preprocessing)
}

impl OrchestratorContext {
    async fn report_manga_preprocessing_completed(
        &self,
        workflow_run_id: i64,
        task_id: &str,
        reporter: Option<&(dyn Fn(ProgressEvent) + Send + Sync)>,
    ) -> Result<(), AppError> {
        let message = "漫画预处理完成".to_string();
        self.record_manga_pipeline_progress_with_total(
            workflow_run_id,
            task_id,
            "preprocessing",
            1,
            1,
            &message,
        )
        .await?;
        if let Some(reporter) = reporter {
            reporter(ProgressEvent {
                stage_code: "manga_preprocessing".into(),
                phase_kind: a4n_progress::ProgressPhaseKind::StructuredOutput,
                current: Some(1),
                total: Some(1),
                message,
                unit_label: Some("阶段".into()),
                item_key: None,
                item_index: None,
                item_total: None,
                severity: a4n_progress::ProgressSeverity::Info,
                estimated: false,
            });
        }
        Ok(())
    }
}
