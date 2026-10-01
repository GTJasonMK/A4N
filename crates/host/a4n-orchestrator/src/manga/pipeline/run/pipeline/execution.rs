use std::collections::HashMap;

use a4n_api_contract::manga::MangaPagePromptSourceSummary;
use a4n_domain::manga::MangaPagePromptRecord;
use a4n_domain::source_spans::source_spans_for_content;
use a4n_error::AppError;

use crate::shooting_script::ChapterShootingScriptSourceInput;

use super::super::super::super::reference_assets::manga_reference_assets_json;
use super::super::super::super::{
    MangaGeneratePagePromptsInput, MangaPipelineStage, OrchestratorContext,
};
use super::super::super::prompt::manga_pipeline_prompt_fingerprint;
use super::super::super::state::manga_pipeline_stage_status;
use super::super::completion::complete_pipeline;
use super::super::contracts::MangaRunContext;
use super::super::early_extraction::run_early_extraction;
use super::super::late_extraction::run_late_extraction;
use super::super::planning::run_planning;

impl OrchestratorContext {
    pub(crate) async fn generate_page_prompts(
        &self,
        input: MangaGeneratePagePromptsInput<'_>,
    ) -> Result<Vec<MangaPagePromptRecord>, AppError> {
        let MangaGeneratePagePromptsInput {
            chapter_id,
            project_id,
            prompt_renderer,
            character_portraits,
            task_id,
            workflow_run_id,
            resume_from_checkpoint,
        } = input;
        let script_context = self
            .require_current_chapter_shooting_script(ChapterShootingScriptSourceInput {
                project_id,
                source_chapter_id: chapter_id,
                visual_plan_id: None,
            })
            .await?;
        if script_context.source.source_chapter_id != chapter_id {
            return Err(AppError::InvalidArgument(format!(
                "漫画章节 ID 与统一来源解析结果不一致: requested={chapter_id}, resolved={}",
                script_context.source.source_chapter_id
            )));
        }
        let chapter_content = script_context.source.content.clone();
        let source_spans = source_spans_for_content(
            chapter_id,
            script_context.source.chapter_number,
            &chapter_content,
        );
        let source_metadata = MangaPagePromptSourceSummary {
            root_project_id: script_context.source.root_project_id,
            source_chapter_id: script_context.source.source_chapter_id,
            source_snapshot_id: script_context.source.source_snapshot_id,
            source_snapshot_chapter_id: script_context.source.source_snapshot_chapter_id,
            source_revision_key: script_context.source.source_revision_key.clone(),
            content_sha256: script_context.source.content_sha256.clone(),
            shooting_script_id: script_context.shooting_script.record.id,
            shooting_script_revision: script_context.shooting_script.record.revision,
            shooting_script_sha256: script_context.shooting_script.record.script_sha256.clone(),
        };
        let (initial_stage, initial_current, initial_message) =
            manga_pipeline_stage_status(MangaPipelineStage::CharacterBinding);
        self.record_manga_pipeline_progress(
            workflow_run_id,
            task_id,
            initial_stage,
            initial_current,
            initial_message,
        )
        .await?;

        let variables = HashMap::new();
        let reference_assets = manga_reference_assets_json(&character_portraits);
        let reference_assets_metadata = reference_assets.clone();
        let prompt_fingerprint =
            manga_pipeline_prompt_fingerprint(prompt_renderer, project_id, &variables).await?;
        let context = MangaRunContext {
            service: self,
            chapter_id,
            project_id,
            shooting_script: script_context.shooting_script,
            prompt_renderer,
            task_id,
            workflow_run_id,
            resume_from_checkpoint,
            chapter_content,
            source_spans,
            variables,
            reference_assets,
            source_metadata,
            reference_assets_metadata,
            prompt_fingerprint,
        };
        let early = run_early_extraction(&context).await?;
        let extraction = run_late_extraction(&context, early).await?;
        let planning = run_planning(&context, extraction).await?;
        complete_pipeline(&context, planning).await
    }
}
