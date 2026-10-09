use std::collections::HashMap;

use a4n_api_contract::manga::MangaPagePromptSourceSummary;
use a4n_domain::manga::{MangaPagePromptRecord, MangaPreprocessingV3};
use a4n_domain::source_spans::source_spans_for_content;
use a4n_error::AppError;

use super::super::super::super::reference_assets::manga_reference_assets_json;
use super::super::super::super::{MangaGeneratePagePromptsInput, OrchestratorContext};
use super::super::super::prompt::manga_pipeline_prompt_fingerprint;
use super::super::completion::complete_pipeline;
use super::super::contracts::MangaRunContext;
use super::super::page_slots::run_page_slots;
use super::super::preprocessing::{MangaPreprocessingInput, run_manga_preprocessing};
use crate::source_markers::SourceMarkerMap;

impl OrchestratorContext {
    pub(crate) async fn generate_page_prompts(
        &self,
        input: MangaGeneratePagePromptsInput<'_>,
    ) -> Result<Vec<MangaPagePromptRecord>, AppError> {
        let MangaGeneratePagePromptsInput {
            user_id,
            chapter_id,
            chapter_number,
            page_number,
            project_id,
            prompt_renderer,
            character_portraits,
            available_reference_images,
            task_id,
            workflow_run_id,
            resume_from_checkpoint,
            stop_after,
            run_stage,
            image_size,
            image_generator,
            cancel_token,
            progress_reporter,
            stream_reporter,
        } = input;
        let source = a4n_db::project::resolve_chapter_source(
            self.database.executor(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(crate::shooting_script::errors::resolve_source_error)?;
        let reference_assets = manga_reference_assets_json(&character_portraits);
        let preprocessing_input_fingerprint =
            a4n_domain::manga::manga_preprocessing_input_fingerprint(
                &source.content_sha256,
                source.chapter_number,
                &reference_assets,
            );
        if run_stage == Some(a4n_domain::manga::MangaPipelineArtifactStage::Preprocessing) {
            run_manga_preprocessing(
                self,
                MangaPreprocessingInput {
                    project_id,
                    chapter_id,
                    chapter_number,
                    chapter_content: &source.content,
                    prompt_renderer,
                    reference_assets,
                    task_id,
                    workflow_run_id,
                    resume_from_checkpoint,
                    cancel_token,
                    progress_reporter,
                    stream_reporter,
                },
            )
            .await?;
            return self
                .manga_page_prompts_by_chapter(project_id, chapter_id)
                .await;
        }
        let preprocessing = self
            .manga_pipeline_artifact(
                project_id,
                chapter_id,
                a4n_domain::manga::MangaPipelineArtifactStage::Preprocessing,
                None,
            )
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "manga_preprocessing_missing".into(),
                operation: "请先完成漫画预处理".into(),
            })?;
        if preprocessing.stale || preprocessing.input_fingerprint != preprocessing_input_fingerprint
        {
            return Err(AppError::InvalidState {
                current: "manga_preprocessing_stale".into(),
                operation: "原文章节已变化，请重新运行漫画预处理".into(),
            });
        }
        let preprocessing_fingerprint = preprocessing.output_fingerprint.clone();
        let preprocessing: MangaPreprocessingV3 = serde_json::from_value(preprocessing.payload)
            .map_err(|error| AppError::InvalidArgument(format!("漫画预处理产物无效: {error}")))?;
        preprocessing
            .validate()
            .map_err(AppError::InvalidArgument)?;
        let adaptation = self
            .require_adopted_manga_adaptation(&source, &preprocessing_fingerprint)
            .await?;
        if source.source_chapter_id != chapter_id {
            return Err(AppError::InvalidArgument(format!(
                "漫画章节 ID 与统一来源解析结果不一致: requested={chapter_id}, resolved={}",
                source.source_chapter_id
            )));
        }
        let chapter_content = source.content.clone();
        let source_spans =
            source_spans_for_content(chapter_id, source.chapter_number, &chapter_content);
        let source_markers = SourceMarkerMap::from_source_spans(&source_spans)?;
        let source_metadata = MangaPagePromptSourceSummary {
            root_project_id: source.root_project_id,
            source_chapter_id: source.source_chapter_id,
            source_snapshot_id: source.source_snapshot_id,
            source_snapshot_chapter_id: source.source_snapshot_chapter_id,
            source_revision_key: source.source_revision_key.clone(),
            content_sha256: source.content_sha256.clone(),
            adaptation_plan_id: adaptation.record.id,
            adaptation_plan_revision: adaptation.record.revision,
            adaptation_plan_sha256: adaptation.record.sha256.clone(),
        };
        let variables = HashMap::new();
        let reference_assets_metadata = reference_assets.clone();
        let prompt_fingerprint = serde_json::json!({
            "pipeline_contract": "manga_page_pipeline_v2",
            "prompts": manga_pipeline_prompt_fingerprint(
                prompt_renderer,
                project_id,
                &variables,
            )
            .await?,
        });
        let context = MangaRunContext {
            service: self,
            chapter_id,
            chapter_number,
            page_number,
            project_id,
            user_id,
            adaptation,
            prompt_renderer,
            task_id,
            workflow_run_id,
            resume_from_checkpoint,
            stop_after,
            run_stage,
            image_size,
            image_generator,
            cancel_token,
            progress_reporter,
            stream_reporter,
            chapter_content,
            source_spans,
            source_markers,
            variables,
            reference_assets,
            source_metadata,
            reference_assets_metadata,
            available_reference_images,
            prompt_fingerprint,
        };
        if let Some(stage) = run_stage {
            return super::super::single_stage::run_single_stage(&context, stage).await;
        }
        let extraction = super::super::contracts::MangaExtractionRun {
            extraction_summary: super::super::single_stage::manga_preprocessing_summary(
                &preprocessing,
            )?,
        };
        let page_slots = run_page_slots(&context, &extraction.extraction_summary).await?;
        complete_pipeline(&context, extraction, page_slots).await
    }
}
