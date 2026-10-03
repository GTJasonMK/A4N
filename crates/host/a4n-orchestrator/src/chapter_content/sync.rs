use a4n_db::workflow::{
    UpsertWorkflowCheckpointInput, WorkflowCheckpointSummary, WorkflowRunSummary,
};
use a4n_domain::chapter::ChapterRecord;
use a4n_domain::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_RUNNING};
use a4n_error::AppError;
use serde_json::{Value, json};

use crate::OrchestratorContext;

use a4n_chapter_content_core::{
    CHAPTER_CONTENT_SYNC_CLEANUP_STEP, CHAPTER_CONTENT_SYNC_EXTERNAL_STEP,
    CHAPTER_CONTENT_SYNC_RAG_STEP, CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
    chapter_content_sync_checkpoint_artifacts, chapter_content_sync_run_input_matches,
    chapter_content_sync_workflow_business_key,
};

use a4n_api_contract::chapter::ChapterDerivedCleanupHostPort;
const CHAPTER_CONTENT_SYNC_POSTPROCESS_STEP: &str = "chapter_content_sync.postprocess";

fn chapter_content_sync_completed_checkpoint_input(
    run: &WorkflowRunSummary,
    step_key: &str,
    step_order: i32,
) -> UpsertWorkflowCheckpointInput {
    UpsertWorkflowCheckpointInput {
        workflow_run_id: run.id,
        step_key: step_key.to_string(),
        step_order,
        status: WORKFLOW_STATUS_COMPLETED.to_string(),
        input_fingerprint_sha256: None,
        input_fingerprint_json: run.input_fingerprint_json.clone(),
        checkpoint_json: Some(json!({ "completed": true })),
        artifacts_json: None,
        generation_run_ids_json: None,
        error_message: None,
        completed: true,
    }
}

fn chapter_content_sync_checkpoint_completed(
    run: &WorkflowRunSummary,
    checkpoint: Option<&WorkflowCheckpointSummary>,
    step_key: &str,
) -> Result<bool, String> {
    let Some(checkpoint) = checkpoint else {
        return Ok(false);
    };
    if checkpoint.step_key != step_key {
        return Err(format!(
            "章节资料同步 checkpoint step_key 无效: expected={step_key}, actual={}",
            checkpoint.step_key
        ));
    }
    if checkpoint.status != WORKFLOW_STATUS_COMPLETED
        || checkpoint.input_fingerprint_json != run.input_fingerprint_json
        || checkpoint
            .checkpoint_json
            .as_ref()
            .and_then(|value| value.get("completed"))
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err(format!("章节资料同步 checkpoint {step_key} 合同无效"));
    }
    Ok(true)
}

impl OrchestratorContext {
    async fn chapter_content_sync_checkpoints(
        &self,
        workflow_run_id: i64,
    ) -> Result<Vec<WorkflowCheckpointSummary>, AppError> {
        a4n_db::workflow::workflow_checkpoint_summaries_for_runs(&self.database, &[workflow_run_id])
            .await
            .map_err(crate::workflow::workflow_read_app_error)
    }

    pub(crate) async fn settle_chapter_content_sync_run(
        &self,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        run: &WorkflowRunSummary,
    ) -> Result<ChapterRecord, AppError> {
        let result = self.execute_chapter_content_sync_run(host_port, run).await;
        self.finalize_workflow_result(run.id, result, "章节资料同步")
            .await
    }

    pub async fn resume_chapter_content_sync_workflow_run_validated(
        &self,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        requested_run: &WorkflowRunSummary,
    ) -> Result<ChapterRecord, AppError> {
        let (project_id, chapter_number) = chapter_content_sync_source_identity(requested_run)?;
        let lock = self.chapter_content_sync_lock(project_id, chapter_number);
        let _guard = lock.lock_owned().await;
        let run = self.workflow_run_summary(requested_run.id).await?;
        self.require_current_chapter_content_sync_run(&run).await?;
        crate::workflow::require_workflow_can_resume(
            &run.status,
            run.can_resume,
            run.resume_block_reason.as_deref(),
        )
        .map_err(|operation| AppError::InvalidState {
            current: run.status.clone(),
            operation,
        })?;
        let run = self.mark_workflow_running_validated(run.id).await?;
        let run = self.workflow_run_summary(run.id).await?;
        self.settle_chapter_content_sync_run(host_port, &run).await
    }

    async fn execute_chapter_content_sync_run(
        &self,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        run: &WorkflowRunSummary,
    ) -> Result<ChapterRecord, AppError> {
        if run.status != WORKFLOW_STATUS_RUNNING {
            return Err(AppError::InvalidState {
                current: run.status.clone(),
                operation: "执行章节资料同步".to_string(),
            });
        }
        let (chapter_result, checkpoints_result) = futures::join!(
            self.require_current_chapter_content_sync_run(run),
            self.chapter_content_sync_checkpoints(run.id),
        );
        let chapter = chapter_result?;
        let checkpoints = checkpoints_result?;
        let cleanup_checkpoint = checkpoints
            .iter()
            .find(|checkpoint| checkpoint.step_key == CHAPTER_CONTENT_SYNC_CLEANUP_STEP)
            .ok_or_else(|| AppError::InvalidState {
                current: format!("chapter_content_sync_run_id={}", run.id),
                operation: "读取章节资料清理计划".to_string(),
            })?;
        let artifacts = chapter_content_sync_checkpoint_artifacts(run, cleanup_checkpoint)
            .map_err(|error| AppError::InvalidState {
                current: format!("chapter_content_sync_run_id={}", run.id),
                operation: error.to_string(),
            })?;

        if !content_sync_phase_completed(run, &checkpoints, CHAPTER_CONTENT_SYNC_EXTERNAL_STEP)? {
            self.mark_workflow_current_step(run.id, CHAPTER_CONTENT_SYNC_EXTERNAL_STEP)
                .await?;
            host_port
                .finish_chapter_derived_cleanup(
                    chapter.project_id,
                    chapter.id,
                    chapter.chapter_number,
                    artifacts,
                )
                .await?;
            self.require_current_chapter_content_sync_run(run).await?;
            self.upsert_workflow_checkpoint(chapter_content_sync_completed_checkpoint_input(
                run,
                CHAPTER_CONTENT_SYNC_EXTERNAL_STEP,
                20,
            ))
            .await?;
        }

        let (chapter_result, checkpoints_result) = futures::join!(
            self.require_current_chapter_content_sync_run(run),
            self.chapter_content_sync_checkpoints(run.id),
        );
        let chapter = chapter_result?;
        let checkpoints = checkpoints_result?;
        if !content_sync_phase_completed(run, &checkpoints, CHAPTER_CONTENT_SYNC_RAG_STEP)? {
            self.mark_workflow_current_step(run.id, CHAPTER_CONTENT_SYNC_RAG_STEP)
                .await?;
            host_port
                .sync_chapter_rag_records(chapter.project_id, chapter.id, run.id)
                .await?;
            self.require_current_chapter_content_sync_run(run).await?;
            self.upsert_workflow_checkpoint(chapter_content_sync_completed_checkpoint_input(
                run,
                CHAPTER_CONTENT_SYNC_RAG_STEP,
                30,
            ))
            .await?;
        }

        let (chapter_result, checkpoints_result) = futures::join!(
            self.require_current_chapter_content_sync_run(run),
            self.chapter_content_sync_checkpoints(run.id),
        );
        let chapter = chapter_result?;
        let checkpoints = checkpoints_result?;
        if !content_sync_phase_completed(run, &checkpoints, CHAPTER_CONTENT_SYNC_POSTPROCESS_STEP)?
        {
            self.mark_workflow_current_step(run.id, CHAPTER_CONTENT_SYNC_POSTPROCESS_STEP)
                .await?;
            if chapter.has_available_content() {
                self.process_chapter(chapter.clone()).await?;
            }
            self.refresh_project_current_chapter(chapter.project_id)
                .await?;
            self.sync_project_status_from_artifacts(chapter.project_id)
                .await?;
            self.require_current_chapter_content_sync_run(run).await?;
            self.upsert_workflow_checkpoint(chapter_content_sync_completed_checkpoint_input(
                run,
                CHAPTER_CONTENT_SYNC_POSTPROCESS_STEP,
                40,
            ))
            .await?;
        }

        self.require_current_chapter_content_sync_run(run).await
    }

    fn chapter_content_sync_chapter_id(&self, run: &WorkflowRunSummary) -> Result<i64, AppError> {
        if run.workflow_kind != CHAPTER_CONTENT_SYNC_WORKFLOW_KIND
            || run.scope_table.as_deref() != Some("chapter")
            || run.scope_id != run.chapter_id
        {
            return Err(AppError::InvalidState {
                current: format!("workflow_kind={}", run.workflow_kind),
                operation: "读取章节资料同步目标".to_string(),
            });
        }
        run.chapter_id.ok_or_else(|| AppError::InvalidState {
            current: "missing_chapter_id".to_string(),
            operation: "读取章节资料同步目标".to_string(),
        })
    }

    async fn require_current_chapter_content_sync_run(
        &self,
        run: &WorkflowRunSummary,
    ) -> Result<ChapterRecord, AppError> {
        let chapter = self
            .chapter_by_id_for_analysis(self.chapter_content_sync_chapter_id(run)?)
            .await?;
        let latest = self
            .latest_workflow_run_by_business_key(&chapter_content_sync_workflow_business_key(
                chapter.project_id,
                chapter.id,
            ))
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_chapter_content_sync_run".to_string(),
                operation: "执行章节资料同步".to_string(),
            })?;
        if latest.id != run.id
            || run.project_id != Some(chapter.project_id)
            || !chapter_content_sync_run_input_matches(&run.input_fingerprint_json, &chapter)
        {
            return Err(AppError::InvalidState {
                current: "chapter_content_sync_replaced".to_string(),
                operation: "执行已被新正文取代的章节资料同步".to_string(),
            });
        }
        Ok(chapter)
    }
}

fn chapter_content_sync_source_identity(run: &WorkflowRunSummary) -> Result<(i64, i32), AppError> {
    let source = run
        .input_fingerprint_json
        .as_ref()
        .and_then(|fingerprint| fingerprint.get("source_fingerprint"));
    let project_id = source
        .and_then(|source| source.get("project_id"))
        .and_then(Value::as_i64);
    let chapter_number = source
        .and_then(|source| source.get("chapter_number"))
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok());
    project_id
        .zip(chapter_number)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("chapter_content_sync_run_id={}", run.id),
            operation: "读取章节资料同步锁键".to_string(),
        })
}

fn content_sync_phase_completed(
    run: &WorkflowRunSummary,
    checkpoints: &[WorkflowCheckpointSummary],
    step_key: &str,
) -> Result<bool, AppError> {
    chapter_content_sync_checkpoint_completed(
        run,
        checkpoints
            .iter()
            .find(|checkpoint| checkpoint.step_key == step_key),
        step_key,
    )
    .map_err(|operation| AppError::InvalidState {
        current: format!("chapter_content_sync_run_id={}", run.id),
        operation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_chapter_content_core::{
        chapter_content_sync_checkpoint_input, chapter_content_sync_workflow_fingerprint,
        chapter_content_sync_workflow_run_input,
    };
    use a4n_domain::project::ChapterDerivedCleanupArtifacts;
    use chrono::Utc;

    fn chapter() -> ChapterRecord {
        let now = Utc::now().naive_utc();
        ChapterRecord {
            id: 7,
            project_id: 3,
            outline_id: None,
            chapter_number: 5,
            title: Some("回声".to_string()),
            content: "正文".to_string(),
            word_count: 2,
            status: "draft".to_string(),
            summary: None,
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn fingerprint_tracks_formal_content_but_not_derived_fields() {
        let mut chapter = chapter();
        let fingerprint = chapter_content_sync_workflow_run_input(&chapter)
            .input_fingerprint_json
            .expect("workflow fingerprint should exist");
        assert!(chapter_content_sync_run_input_matches(
            &Some(fingerprint.clone()),
            &chapter
        ));
        chapter.style_tags = Some(json!(["叙事为主"]));
        chapter.updated_at = Utc::now().naive_utc();
        assert_eq!(
            fingerprint.get("source_fingerprint"),
            Some(&chapter_content_sync_workflow_fingerprint(&chapter))
        );

        chapter.content = "新正文".to_string();
        assert_ne!(
            fingerprint.get("source_fingerprint"),
            Some(&chapter_content_sync_workflow_fingerprint(&chapter))
        );
    }

    #[test]
    fn checkpoint_round_trips_cleanup_artifacts() {
        let artifacts = ChapterDerivedCleanupArtifacts {
            stale_attribute_event_ids: vec![1],
            scene_records_changed: true,
            ..Default::default()
        };
        let chapter = chapter();
        let fingerprint = chapter_content_sync_workflow_run_input(&chapter)
            .input_fingerprint_json
            .expect("workflow fingerprint should exist");
        let checkpoint =
            chapter_content_sync_checkpoint_input(9, fingerprint, &chapter, &artifacts)
                .expect("checkpoint should serialize");
        let restored: ChapterDerivedCleanupArtifacts = serde_json::from_value(
            checkpoint
                .artifacts_json
                .expect("checkpoint artifacts should exist"),
        )
        .expect("checkpoint should parse");
        assert_eq!(restored, artifacts);
    }

    #[test]
    fn repeated_saves_get_distinct_runs_for_the_same_formal_content() {
        let chapter = chapter();
        let first = chapter_content_sync_workflow_run_input(&chapter)
            .input_fingerprint_json
            .expect("first workflow fingerprint should exist");
        let second = chapter_content_sync_workflow_run_input(&chapter)
            .input_fingerprint_json
            .expect("second workflow fingerprint should exist");

        assert_ne!(first, second);
        assert!(chapter_content_sync_run_input_matches(
            &Some(first),
            &chapter
        ));
        assert!(chapter_content_sync_run_input_matches(
            &Some(second),
            &chapter
        ));

        let malformed = json!({
            "schema_version": 2,
            "workflow_kind": CHAPTER_CONTENT_SYNC_WORKFLOW_KIND,
            "source_fingerprint": chapter_content_sync_workflow_fingerprint(&chapter),
        });
        assert!(!chapter_content_sync_run_input_matches(
            &Some(malformed),
            &chapter
        ));
    }

    #[test]
    fn source_identity_uses_the_current_workflow_contract() {
        let run_input = chapter_content_sync_workflow_run_input(&chapter());
        let now = Utc::now();
        let run = WorkflowRunSummary {
            id: 9,
            business_key: run_input.business_key,
            workflow_kind: run_input.workflow_kind,
            project_id: run_input.project_id,
            root_project_id: run_input.root_project_id,
            source_snapshot_id: run_input.source_snapshot_id,
            scope_table: run_input.scope_table,
            scope_id: run_input.scope_id,
            chapter_id: run_input.chapter_id,
            mode: run_input.mode,
            input_fingerprint_json: run_input.input_fingerprint_json,
            status: run_input.status,
            current_step_key: run_input.current_step_key,
            progress: None,
            progress_json: None,
            active_task_id: run_input.active_task_id,
            can_resume: run_input.can_resume,
            resume_block_reason: run_input.resume_block_reason,
            error_message: run_input.error_message,
            created_at: now.to_rfc3339(),
            updated_at: now.to_rfc3339(),
            completed_at: None,
        };

        assert_eq!(chapter_content_sync_source_identity(&run).unwrap(), (3, 5));
    }

    #[test]
    fn retry_plan_carries_forward_and_deduplicates_pending_cleanup() {
        let mut carried = ChapterDerivedCleanupArtifacts {
            stale_attribute_event_ids: vec![3, 1],
            ..Default::default()
        };
        carried.merge(ChapterDerivedCleanupArtifacts {
            stale_attribute_event_ids: vec![3, 4],
            stale_character_state_ids: vec![8],
            scene_records_changed: true,
            ..Default::default()
        });

        assert_eq!(carried.stale_attribute_event_ids, vec![1, 3, 4]);
        assert_eq!(carried.stale_character_state_ids, vec![8]);
        assert!(carried.scene_records_changed);
    }
}
