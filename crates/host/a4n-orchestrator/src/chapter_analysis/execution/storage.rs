use a4n_chapter_analysis::ensure_chapter_has_analysis_content;
use a4n_db::chapter_content::{lock_chapter_analysis_source, store_chapter_analysis_record};
use a4n_db::workflow::UpsertWorkflowCheckpointInput;
use a4n_db_models::models::{chapter::chapter, generation::workflow_run};
use a4n_domain::{
    chapter::StoreChapterAnalysisRecordInput,
    workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_RUNNING},
};
use a4n_error::AppError;
use sea_orm::TransactionTrait;
use serde_json::json;

use super::super::owner::chapter_analysis_storage_projection;
use super::super::{
    CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP, ChapterAnalysisInput, ChapterAnalysisProgress,
    OrchestratorContext, chapter_analysis_phase_fingerprint, chapter_analysis_run_input_matches,
    chapter_analysis_workflow_business_key, chapter_analysis_workflow_run_input,
};
use crate::chapter_content_edit_error_to_app_error;
use crate::generation::generation_error;
use crate::workflow::{workflow_read_app_error, workflow_write_app_error};
use a4n_chapter_content_core::cleanup_chapter_derived_data_in;

impl OrchestratorContext {
    pub(crate) async fn begin_chapter_analysis_workflow_run(
        &self,
        chapter: &chapter::Model,
        current_step_key: &str,
        active_task_id: Option<&str>,
    ) -> Result<workflow_run::Model, AppError> {
        let source = a4n_db::project::chapter_source_snapshot_from_model(chapter);
        let input = chapter_analysis_workflow_run_input(chapter, current_step_key, active_task_id);
        crate::workflow::validate_begin_workflow_run_input(&input)
            .map_err(AppError::InvalidArgument)?;
        let tx = self
            .database
            .connection()
            .begin()
            .await
            .map_err(AppError::Database)?;
        let started = async {
            lock_chapter_analysis_source(&tx, &source)
                .await
                .map_err(chapter_content_edit_error_to_app_error)?;
            a4n_db::workflow::upsert_workflow_run_in_transaction(&tx, input)
                .await
                .map_err(workflow_write_app_error)
        }
        .await;
        match started {
            Ok(run) => {
                tx.commit().await.map_err(AppError::Database)?;
                Ok(run)
            }
            Err(error) => {
                tx.rollback().await.map_err(|rollback_error| {
                    AppError::Internal(anyhow::anyhow!(
                        "启动章节分析失败：{error}；事务回滚失败：{rollback_error}"
                    ))
                })?;
                Err(error)
            }
        }
    }

    pub(crate) async fn analyze_and_store_chapter(
        &self,
        input: ChapterAnalysisInput<'_>,
    ) -> Result<chapter::Model, AppError> {
        let ChapterAnalysisInput {
            chapter,
            prompt_renderer,
            progress,
            stream_reporter,
            cancel_token,
            workflow_run_id,
        } = input;
        ensure_chapter_has_analysis_content(&chapter)?;

        let workflow_run_id = workflow_run_id
            .ok_or_else(|| AppError::InvalidArgument("章节分析缺少 workflow_run_id".to_string()))?;
        let source = a4n_db::project::chapter_source_snapshot_from_model(&chapter);
        let mut progress = ChapterAnalysisProgress::new(progress, cancel_token)
            .with_stream_reporter(stream_reporter);
        let run = self
            .analyze_chapter(
                &chapter,
                prompt_renderer,
                &mut progress,
                Some(workflow_run_id),
            )
            .await?;
        let generation_run_input = self.chapter_analysis_generation_run_input(&chapter, &run)?;
        let analysis = run.analysis;
        let projection = chapter_analysis_storage_projection(&chapter, &analysis);

        let tx = self
            .database
            .connection()
            .begin()
            .await
            .map_err(AppError::Database)?;
        let replacement = async {
            let locked = lock_chapter_analysis_source(&tx, &source)
                .await
                .map_err(chapter_content_edit_error_to_app_error)?;
            let business_key = chapter_analysis_workflow_business_key(locked.project_id, locked.id);
            let latest = a4n_db::workflow::latest_workflow_run_by_business_key_for_update(
                &tx,
                &business_key,
            )
            .await
            .map_err(workflow_read_app_error)?
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_chapter_analysis_workflow".to_string(),
                operation: "保存章节结构化分析".to_string(),
            })?;
            if latest.id != workflow_run_id
                || latest.workflow_kind != "chapter_analysis"
                || latest.project_id != Some(locked.project_id)
                || latest.chapter_id != Some(locked.id)
                || latest.status != WORKFLOW_STATUS_RUNNING
                || !chapter_analysis_run_input_matches(&latest.input_fingerprint_json, &locked)
            {
                return Err(AppError::InvalidState {
                    current: "chapter_analysis_workflow_replaced".to_string(),
                    operation: "保存已被更新运行取代的章节分析".to_string(),
                });
            }

            let mut next_source = locked.clone();
            next_source.analysis_data = Some(analysis.clone());
            let next_revision = a4n_db::fact_feedback::source_revision(&next_source);
            let facts_unchanged = next_revision.is_some()
                && a4n_db::fact_feedback::source_revision(&locked) == next_revision
                && a4n_db::fact_feedback::current_feedback_sources(&tx, locked.project_id)
                    .await
                    .map_err(crate::fact_feedback::feedback_error)?
                    .iter()
                    .any(|source| Some(&source.revision) == next_revision.as_ref());
            // Repeated analysis with the same facts must not retract later chapters or create material versions.
            let artifacts = if facts_unchanged {
                Default::default()
            } else {
                cleanup_chapter_derived_data_in(
                    &tx,
                    locked.project_id,
                    locked.id,
                    locked.chapter_number,
                    Some(workflow_run_id),
                )
                .await?
            };

            let updated = store_chapter_analysis_record(
                &tx,
                locked,
                StoreChapterAnalysisRecordInput {
                    analysis_data: analysis,
                    summary: projection.summary,
                    style_tags: projection.style_tags,
                },
            )
            .await
            .map_err(chapter_content_edit_error_to_app_error)?;
            a4n_db::generation::delete_generation_run_targets_for(
                &tx,
                "chapter_analysis",
                &[updated.id],
            )
            .await
            .map_err(generation_error)?;
            a4n_db::generation::record_completed_generation_run_in_transaction(
                &tx,
                generation_run_input,
            )
            .await
            .map_err(generation_error)?;

            let fingerprint = chapter_analysis_phase_fingerprint(
                &updated,
                CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP,
            );
            let fingerprint_sha256 =
                a4n_db::generation::generation_checkpoint_fingerprint_sha256(&fingerprint)
                    .map_err(AppError::Serialization)?;
            a4n_db::workflow::upsert_workflow_checkpoint_in_transaction(
                &tx,
                UpsertWorkflowCheckpointInput {
                    workflow_run_id,
                    step_key: CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP.to_string(),
                    step_order: 6,
                    status: WORKFLOW_STATUS_COMPLETED.to_string(),
                    input_fingerprint_sha256: Some(fingerprint_sha256),
                    input_fingerprint_json: Some(fingerprint),
                    checkpoint_json: Some(json!({
                        "completed": true,
                        "facts_unchanged": facts_unchanged,
                        "cleanup_artifacts": artifacts,
                    })),
                    artifacts_json: None,
                    generation_run_ids_json: None,
                    error_message: None,
                    completed: true,
                },
            )
            .await
            .map_err(workflow_write_app_error)?;
            Ok(updated)
        }
        .await;

        match replacement {
            Ok(updated) => {
                tx.commit().await.map_err(AppError::Database)?;
                Ok(updated)
            }
            Err(error) => {
                tx.rollback().await.map_err(|rollback_error| {
                    AppError::Internal(anyhow::anyhow!(
                        "章节分析替换失败：{error}；事务回滚失败：{rollback_error}"
                    ))
                })?;
                Err(error)
            }
        }
    }
}
