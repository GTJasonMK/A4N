use std::collections::HashMap;

use a4n_domain::{manga::MangaPreprocessingV3, source_spans::source_spans_for_content};
use a4n_error::AppError;

use super::super::super::reference_assets::manga_reference_assets_json;
use super::super::super::{MangaPipelineRequestPreviewInput, OrchestratorContext};
use super::super::prompt::{MangaPreprocessingMessages, manga_preprocessing_summary};
use super::contracts::{MangaPipelineRequestPreviewStep, MangaPreviewContext};
use super::planning::preview_planning;
use crate::source_markers::SourceMarkerMap;

impl OrchestratorContext {
    pub(crate) async fn preview_generate_page_prompt_requests(
        &self,
        input: MangaPipelineRequestPreviewInput<'_>,
    ) -> Result<Vec<MangaPipelineRequestPreviewStep>, AppError> {
        let MangaPipelineRequestPreviewInput {
            chapter_id,
            project_id,
            prompt_renderer,
            character_portraits,
            available_reference_images,
            workflow_run_id,
            resume_from_checkpoint,
            stop_after,
            run_stage,
            page_number,
        } = input;
        let source = a4n_db::project::resolve_chapter_source(
            self.database.executor(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(crate::shooting_script::errors::resolve_source_error)?;
        let reference_assets = manga_reference_assets_json(&character_portraits);
        if run_stage == Some(a4n_domain::manga::MangaPipelineArtifactStage::Preprocessing) {
            let source_spans =
                source_spans_for_content(chapter_id, source.chapter_number, &source.content);
            let source_markers = SourceMarkerMap::from_source_spans(&source_spans)?;
            return super::preprocessing::preview_preprocessing(
                self,
                project_id,
                chapter_id,
                prompt_renderer,
                workflow_run_id,
                resume_from_checkpoint,
                &MangaPreprocessingMessages {
                    chapter_content: &source.content,
                    source_spans: &source_spans,
                    source_markers: &source_markers,
                    reference_assets: &reference_assets,
                },
            )
            .await;
        }
        let preprocessing_input_fingerprint =
            a4n_domain::manga::manga_preprocessing_input_fingerprint(
                &source.content_sha256,
                source.chapter_number,
                &reference_assets,
            );
        let preprocessing = self
            .manga_pipeline_artifact(
                project_id,
                chapter_id,
                a4n_domain::manga::MangaPipelineArtifactStage::Preprocessing,
                None,
            )
            .await?
            .filter(|artifact| {
                !artifact.stale && artifact.input_fingerprint == preprocessing_input_fingerprint
            })
            .ok_or_else(|| AppError::InvalidState {
                current: "manga_preprocessing_stale".into(),
                operation: "请先重新运行漫画预处理".into(),
            })?;
        let preprocessing_fingerprint = preprocessing.output_fingerprint.clone();
        let preprocessing: MangaPreprocessingV3 = serde_json::from_value(preprocessing.payload)?;
        preprocessing
            .validate()
            .map_err(AppError::InvalidArgument)?;
        let adaptation = self
            .require_adopted_manga_adaptation(&source, &preprocessing_fingerprint)
            .await?;
        let chapter_content = source.content.clone();
        let source_spans =
            source_spans_for_content(chapter_id, source.chapter_number, &chapter_content);
        let source_markers = SourceMarkerMap::from_source_spans(&source_spans)?;
        let context = MangaPreviewContext {
            service: self,
            chapter_id,
            project_id,
            adaptation,
            prompt_renderer,
            workflow_run_id,
            resume_from_checkpoint,
            stop_after,
            run_stage,
            page_number,
            chapter_content,
            source_spans,
            source_markers,
            variables: HashMap::new(),
            reference_assets,
            available_reference_images,
        };

        let mut steps = Vec::new();
        let extraction_summary = manga_preprocessing_summary(&preprocessing)?;
        preview_planning(&context, &mut steps, extraction_summary).await?;
        Ok(steps)
    }
}
