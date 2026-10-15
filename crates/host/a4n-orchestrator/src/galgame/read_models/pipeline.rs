use a4n_api_contract::galgame::{
    GalgamePipelineStatusSummary, GalgameWorkflowCheckpointSummary, GalgameWorkflowRunSummary,
};
use a4n_db::{galgame::GalgamePipelinePromptContextRecord, media_authoring as authoring_records};
use a4n_domain::workflow::{WorkflowCheckpointSummary, WorkflowRunSummary};
use a4n_error::AppError;

use crate::galgame::{
    GalgameChapterAdaptation, GalgamePipelinePromptContext, OrchestratorContext,
    galgame_pipeline_source_spans_from_prompt_chapters, galgame_prompt_chapter_from_snapshot,
    galgame_speaker_profiles_from_prompt_records, validate_positive_id,
};
use crate::media_authoring::{authoring_error, authoring_source, parse_adaptation};
use crate::shooting_script::errors::resolve_source_error;

pub(in crate::galgame) fn galgame_pipeline_status_summary(
    run: Option<WorkflowRunSummary>,
    checkpoints: Vec<WorkflowCheckpointSummary>,
) -> GalgamePipelineStatusSummary {
    GalgamePipelineStatusSummary {
        run: run.map(|run| GalgameWorkflowRunSummary {
            id: run.id,
            status: run.status,
            mode: run.mode,
            current_step_key: run.current_step_key,
            progress_json: run.progress_json,
            active_task_id: run.active_task_id,
            can_resume: run.can_resume,
            resume_block_reason: run.resume_block_reason,
            error_message: run.error_message,
            created_at: run.created_at,
            updated_at: run.updated_at,
            completed_at: run.completed_at,
        }),
        checkpoints: checkpoints
            .into_iter()
            .map(|checkpoint| GalgameWorkflowCheckpointSummary {
                id: checkpoint.id,
                step_key: checkpoint.step_key,
                step_order: checkpoint.step_order,
                status: checkpoint.status,
                checkpoint_json: checkpoint.checkpoint_json,
                artifacts_json: checkpoint.artifacts_json,
                error_message: checkpoint.error_message,
                created_at: checkpoint.created_at,
                updated_at: checkpoint.updated_at,
                completed_at: checkpoint.completed_at,
            })
            .collect(),
    }
}

impl OrchestratorContext {
    pub async fn galgame_pipeline_prompt_context(
        &self,
        project_id: i64,
    ) -> Result<GalgamePipelinePromptContext, AppError> {
        validate_positive_id(project_id, "project_id")?;
        let context = self.load_galgame_context(project_id).await?;
        let record = a4n_db::galgame::galgame_pipeline_prompt_context_record(
            self.database.executor(),
            context.root_project.id,
            context.source_snapshot.id,
        )
        .await
        .map_err(AppError::from)?;
        let root_project_id = context.root_project.id;
        let source_snapshot_id = context.source_snapshot.id;
        let mut context = galgame_pipeline_prompt_context_from_record(
            context.project.id,
            root_project_id,
            source_snapshot_id,
            record,
        )?;
        let source_chapter_ids = context
            .chapters
            .iter()
            .map(|chapter| chapter.source_chapter_id)
            .collect::<Vec<_>>();
        let sources = a4n_db::project::resolve_snapshot_sources(
            self.database.executor(),
            project_id,
            root_project_id,
            source_snapshot_id,
            &source_chapter_ids,
        )
        .await
        .map_err(resolve_source_error)?;
        for (chapter_batch, source_batch) in context
            .chapters
            .chunks(authoring_records::AUTHORING_READ_BATCH_SIZE)
            .zip(sources.chunks(authoring_records::AUTHORING_READ_BATCH_SIZE))
        {
            let source_chapter_ids = source_batch
                .iter()
                .map(|source| source.source_chapter_id)
                .collect::<Vec<_>>();
            let rows = authoring_records::current_authoring_document_rows_by_source_chapters(
                self.database.executor(),
                project_id,
                source_snapshot_id,
                a4n_domain::media_authoring::AuthoringDocumentKind::AdaptationPlan,
                &source_chapter_ids,
            )
            .await
            .map_err(authoring_error)?;
            for ((chapter, source), row) in chapter_batch.iter().zip(source_batch).zip(rows) {
                authoring_source(source)?;
                let Some(row) = row.map_err(authoring_error)? else {
                    continue;
                };
                let adaptation = parse_adaptation(
                    row.parse().map_err(authoring_error)?,
                    source,
                    a4n_domain::media_authoring::AdaptationTarget::Galgame,
                )?;
                context.adaptations.push(GalgameChapterAdaptation {
                    source_chapter_id: chapter.source_chapter_id,
                    source_snapshot_chapter_id: chapter.source_snapshot_chapter_id,
                    chapter_number: chapter.chapter_number,
                    adaptation_plan_id: adaptation.record.id,
                    adaptation_plan_revision: adaptation.record.revision,
                    plan: adaptation.plan,
                });
            }
        }
        Ok(context)
    }
}

/// 将 DB 层管线提示词上下文 read model 装配为 service 侧 DTO。
pub(crate) fn galgame_pipeline_prompt_context_from_record(
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: i64,
    record: GalgamePipelinePromptContextRecord,
) -> Result<GalgamePipelinePromptContext, AppError> {
    if record.chapters.is_empty() {
        return Err(AppError::InvalidState {
            current: "source_snapshot_has_no_chapters".to_string(),
            operation: "load_galgame_pipeline_prompt_context".to_string(),
        });
    }
    let mut prompt_chapters = Vec::with_capacity(record.chapters.len());
    for chapter in record.chapters {
        prompt_chapters.push(galgame_prompt_chapter_from_snapshot(
            &chapter.chapter,
            &chapter.text_anchors,
        )?);
    }
    let source_spans = galgame_pipeline_source_spans_from_prompt_chapters(&prompt_chapters);
    Ok(GalgamePipelinePromptContext {
        project_id,
        root_project_id,
        source_snapshot_id,
        speaker_profiles: galgame_speaker_profiles_from_prompt_records(&record.speaker_profiles),
        chapters: prompt_chapters,
        source_spans,
        adaptations: Vec::new(),
    })
}
