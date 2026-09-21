//! Generation task reads, snapshots, persistence and subscriptions.

use super::*;

impl GenerationApplication {
    async fn load_task_checkpoint(
        &self,
        task_id: &str,
    ) -> Result<
        a4n_db_models::models::generation::generation_task_checkpoint::Model,
        GenerationTaskAccessError,
    > {
        a4n_db::generation::load_generation_task_checkpoint(self.connection(), task_id)
            .await
            .map_err(|error| match error {
                GenerationTaskPersistenceError::NotFound(message) => {
                    GenerationTaskAccessError::NotFound(message)
                }
                other => GenerationTaskAccessError::Persistence(other),
            })
    }

    async fn list_active_task_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
    ) -> Result<Vec<GenerationTaskSummary>, GenerationTaskAccessError> {
        let page = a4n_db::generation::list_active_generation_task_checkpoints(
            self.connection(),
            user_id,
            project_id,
        )
        .await
        .map_err(GenerationTaskAccessError::Persistence)?;
        summarize_checkpoint_page(page)
    }

    async fn list_history_task_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        limit: u32,
        cursor: Option<GenerationTaskListCursor>,
    ) -> Result<GenerationTaskHistorySummaries, GenerationTaskAccessError> {
        let page = a4n_db::generation::list_generation_task_history_checkpoints(
            self.connection(),
            user_id,
            project_id,
            limit,
            cursor,
        )
        .await
        .map_err(GenerationTaskAccessError::Persistence)?;
        let next_cursor = if page.has_more {
            page.items
                .last()
                .map(|checkpoint| GenerationTaskListCursor {
                    updated_at_ms: DateTime::<Utc>::from_naive_utc_and_offset(
                        checkpoint.updated_at,
                        Utc,
                    )
                    .timestamp_millis(),
                    task_id: checkpoint.task_id.clone(),
                })
        } else {
            None
        };
        let items = summarize_checkpoint_page(page)?;
        Ok(GenerationTaskHistorySummaries { items, next_cursor })
    }

    pub(crate) async fn record_completed_generation_run(
        &self,
        input: RecordCompletedGenerationRunInput,
    ) -> Result<generation_run::Model, AppError> {
        a4n_db::generation::record_completed_generation_run(self.connection(), input)
            .await
            .map_err(generation_error)
    }

    pub(super) async fn record_completed_generation_run_with_checkpoint(
        &self,
        input: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<generation_run::Model, AppError> {
        a4n_db::generation::record_completed_run_with_checkpoint_transaction(
            self.connection(),
            input,
        )
        .await
        .map_err(generation_error)
    }

    pub(super) async fn attach_generation_runs_to_workflow_for_target(
        &self,
        input: AttachGenerationRunsToWorkflowInput,
    ) -> Result<usize, AppError> {
        a4n_db::generation::attach_runs_to_workflow_for_target(self.connection(), input)
            .await
            .map_err(generation_error)
    }

    pub(crate) async fn record_llm_call_trace(
        &self,
        input: RecordLlmCallTraceInput,
    ) -> Result<llm_call_trace::Model, AppError> {
        a4n_db::generation::record_llm_call_trace(self.connection(), input)
            .await
            .map_err(generation_error)
    }

    pub(in crate::context) async fn complete_generation_runs_for_target(
        &self,
        target_table: &str,
        target_id: i64,
        response_summary_json: Option<Value>,
    ) -> Result<usize, AppError> {
        a4n_db::generation::complete_runs_for_target(
            self.connection(),
            target_table,
            target_id,
            response_summary_json,
        )
        .await
        .map_err(generation_error)
    }

    pub(in crate::context) async fn fail_generation_runs_for_target(
        &self,
        target_table: &str,
        target_id: i64,
        error_message: String,
        response_summary_json: Option<Value>,
    ) -> Result<usize, AppError> {
        a4n_db::generation::fail_runs_for_target(
            self.connection(),
            target_table,
            target_id,
            error_message,
            response_summary_json,
        )
        .await
        .map_err(generation_error)
    }

    pub(in crate::context) async fn cancel_generation_runs_for_target(
        &self,
        target_table: &str,
        target_id: i64,
        message: String,
    ) -> Result<usize, AppError> {
        a4n_db::generation::cancel_runs_for_target(
            self.connection(),
            target_table,
            target_id,
            message,
        )
        .await
        .map_err(generation_error)
    }

    pub(in crate::context) async fn cancel_generation_runs_for_targets(
        &self,
        target_table: &str,
        target_ids: &[i64],
        message: String,
    ) -> Result<Vec<(i64, usize)>, AppError> {
        a4n_db::generation::cancel_runs_for_targets(
            self.connection(),
            target_table,
            target_ids,
            message,
        )
        .await
        .map_err(generation_error)
    }

    pub(super) async fn generation_run_exists_for_target_output(
        &self,
        target_table: &str,
        target_id: i64,
        step_key: &str,
        response_output: &str,
    ) -> Result<bool, AppError> {
        a4n_db::generation::generation_run_exists_for_target_output(
            self.connection(),
            target_table,
            target_id,
            step_key,
            response_output,
        )
        .await
        .map_err(generation_error)
    }

    pub(super) fn session(
        &self,
        task_id: &str,
        attempt: u32,
    ) -> Result<Option<Arc<GenerationTaskSession>>, GenerationTerminalError> {
        self.terminal
            .registry()
            .get(task_id, attempt)
            .map_err(Into::into)
    }

    pub(crate) async fn refresh_session_after_workflow_binding(
        &self,
        task_id: Option<&str>,
    ) -> Result<(), GenerationTaskAccessError> {
        let Some(task_id) = task_id else {
            return Ok(());
        };
        let loaded = a4n_db::generation::load_generation_task(self.connection(), task_id).await?;
        let Some(snapshot) = loaded.public_snapshot.as_ref() else {
            return Ok(());
        };
        let Some(session) = self.session(&snapshot.task_id, snapshot.attempt)? else {
            return Ok(());
        };
        let _guard = session.checkpoint_write_guard().await;
        let loaded = a4n_db::generation::load_generation_task(self.connection(), task_id).await?;
        let Some(snapshot) = loaded.public_snapshot else {
            return Ok(());
        };
        let covers_through_sequence = u64::try_from(loaded.checkpoint.covers_through_sequence)
            .map_err(|_| {
                GenerationSessionError::InvalidSession(
                    "数据库 generation covers 为负数".to_string(),
                )
            })?;
        let reserved_through_sequence = u64::try_from(loaded.checkpoint.reserved_through_sequence)
            .map_err(|_| {
                GenerationSessionError::InvalidSession(
                    "数据库 generation high-water 为负数".to_string(),
                )
            })?;
        session.rebuild_from_persisted_snapshot(
            snapshot,
            crate::generation::events::GenerationDurableCommit {
                checkpoint_revision: loaded.checkpoint.checkpoint_revision,
                covers_through_sequence,
                reserved_through_sequence,
                persisted_at_ms: loaded.checkpoint.updated_at.and_utc().timestamp_millis(),
            },
        )?;
        Ok(())
    }
}

impl OrchestratorContext {
    pub(crate) async fn authorize_generation_task(
        &self,
        user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<AuthorizedGenerationTask, GenerationTaskAccessError> {
        let checkpoint = self.generation.load_task_checkpoint(task_id).await?;
        authorize_checkpoint(&checkpoint, user_id, attempt, self).await?;
        let persisted = a4n_db::generation::persisted_generation_task_from_checkpoint(checkpoint)
            .map_err(GenerationTaskAccessError::Persistence)?;
        Ok(AuthorizedGenerationTask { persisted })
    }

    /// 读取 summary 时只校验 checkpoint 规范列；损坏/过期 snapshot 仍返回可展示条目。
    pub(crate) async fn generation_task_summary_for_user(
        &self,
        user_id: i64,
        task_id: &str,
        attempt: u32,
    ) -> Result<GenerationTaskSummary, GenerationTaskAccessError> {
        let checkpoint = self.generation.load_task_checkpoint(task_id).await?;
        authorize_checkpoint(&checkpoint, user_id, attempt, self).await?;
        generation_task_summary_from_checkpoint(
            &checkpoint,
            a4n_db::generation::generation_public_snapshot_for_summary(&checkpoint),
        )
    }

    pub(crate) async fn list_active_generation_task_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
    ) -> Result<Vec<GenerationTaskSummary>, GenerationTaskAccessError> {
        if let Some(project_id) = project_id {
            self.verify_project_permission(
                project_id,
                user_id,
                a4n_domain::collaboration::ProjectAccessRole::Viewer,
            )
            .await
            .map_err(GenerationTaskAccessError::Project)?;
        }
        self.generation
            .list_active_task_summaries(user_id, project_id)
            .await
    }

    pub(crate) async fn list_generation_task_history_summaries(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        limit: u32,
        cursor: Option<GenerationTaskListCursor>,
    ) -> Result<GenerationTaskHistorySummaries, GenerationTaskAccessError> {
        if let Some(project_id) = project_id {
            self.verify_project_permission(
                project_id,
                user_id,
                a4n_domain::collaboration::ProjectAccessRole::Viewer,
            )
            .await
            .map_err(GenerationTaskAccessError::Project)?;
        }
        self.generation
            .list_history_task_summaries(user_id, project_id, limit, cursor)
            .await
    }

    /// 只从已授权 task 取得进程内 session；不存在 live session 时显式要求 snapshot。
    pub(crate) fn subscribe_generation_task(
        &self,
        task: &AuthorizedGenerationTask,
        after_sequence: Option<u64>,
    ) -> Result<OwnerGenerationSubscription, GenerationTaskAccessError> {
        let session = self
            .generation
            .session_for_authorized_task(&task.persisted)?
            .ok_or(GenerationTaskAccessError::SnapshotRequired)?;
        session.subscribe(after_sequence).map_err(Into::into)
    }
}

async fn authorize_checkpoint(
    checkpoint: &a4n_db_models::models::generation::generation_task_checkpoint::Model,
    user_id: i64,
    attempt: u32,
    context: &OrchestratorContext,
) -> Result<(), GenerationTaskAccessError> {
    if checkpoint.requested_by_user_id != user_id {
        return Err(GenerationTaskAccessError::Forbidden(
            checkpoint.task_id.clone(),
        ));
    }
    let actual_attempt = u32::try_from(checkpoint.attempt).map_err(|_| {
        GenerationTaskAccessError::Persistence(GenerationTaskPersistenceError::Corrupt(format!(
            "generation task {} attempt 超出 u32 范围",
            checkpoint.task_id
        )))
    })?;
    if actual_attempt != attempt {
        return Err(GenerationTaskAccessError::AttemptMismatch {
            expected: attempt,
            actual: actual_attempt,
        });
    }
    if let Some(project_id) = checkpoint.project_id.or(checkpoint.root_project_id) {
        context
            .verify_project_permission(
                project_id,
                user_id,
                a4n_domain::collaboration::ProjectAccessRole::Viewer,
            )
            .await
            .map_err(GenerationTaskAccessError::Project)?;
    }
    Ok(())
}

fn summarize_checkpoint_page(
    page: GenerationTaskCheckpointPage,
) -> Result<Vec<GenerationTaskSummary>, GenerationTaskAccessError> {
    page.items
        .iter()
        .map(|checkpoint| {
            generation_task_summary_from_checkpoint(
                checkpoint,
                a4n_db::generation::generation_public_snapshot_for_summary(checkpoint),
            )
        })
        .collect()
}

fn generation_task_summary_from_checkpoint(
    checkpoint: &a4n_db_models::models::generation::generation_task_checkpoint::Model,
    public_snapshot: GenerationPublicSnapshot,
) -> Result<GenerationTaskSummary, GenerationTaskAccessError> {
    let started = a4n_db::generation::started_event_from_checkpoint(checkpoint)
        .map_err(GenerationTaskAccessError::Persistence)?;
    let status: GenerationTaskStatus =
        serde_json::from_value(Value::String(checkpoint.status.clone())).map_err(|error| {
            GenerationTaskAccessError::Persistence(GenerationTaskPersistenceError::Corrupt(
                format!(
                    "generation task {} status 无效: {error}",
                    checkpoint.task_id
                ),
            ))
        })?;
    let attempt = u32::try_from(checkpoint.attempt).map_err(|_| {
        GenerationTaskAccessError::Persistence(GenerationTaskPersistenceError::Corrupt(format!(
            "generation task {} attempt 超出 u32 范围",
            checkpoint.task_id
        )))
    })?;
    if attempt == 0 {
        return Err(GenerationTaskAccessError::Persistence(
            GenerationTaskPersistenceError::Corrupt(format!(
                "generation task {} attempt 必须大于 0",
                checkpoint.task_id
            )),
        ));
    }
    let result_ref = match (
        checkpoint.result_kind.as_ref(),
        checkpoint.result_id.as_ref(),
        checkpoint.result_revision.as_ref(),
    ) {
        (None, None, None) => None,
        (Some(kind), Some(id), Some(revision)) if status == GenerationTaskStatus::Completed => {
            Some(a4n_service_protocol::GenerationResultRef {
                kind: kind.clone(),
                id: id.clone(),
                revision: revision.clone(),
            })
        }
        (Some(_), Some(_), Some(_)) => None,
        _ => {
            return Err(GenerationTaskAccessError::Persistence(
                GenerationTaskPersistenceError::Corrupt(format!(
                    "generation task {} result ref 规范列不成组",
                    checkpoint.task_id
                )),
            ));
        }
    };
    let (snapshot_state, terminal_event) = match public_snapshot {
        GenerationPublicSnapshot::Available(snapshot) => (
            GenerationSnapshotState::Available,
            snapshot
                .status
                .is_terminal()
                .then_some(snapshot.terminal_event)
                .flatten(),
        ),
        GenerationPublicSnapshot::Expired => (GenerationSnapshotState::Expired, None),
        GenerationPublicSnapshot::Corrupt => (GenerationSnapshotState::Corrupt, None),
    };
    let updated_at_ms =
        DateTime::<Utc>::from_naive_utc_and_offset(checkpoint.updated_at, Utc).timestamp_millis();
    if !(0..=a4n_service_protocol::JS_SAFE_INTEGER_I64).contains(&updated_at_ms) {
        return Err(GenerationTaskAccessError::Persistence(
            GenerationTaskPersistenceError::Corrupt(format!(
                "generation task {} updated_at 超出 JavaScript safe integer",
                checkpoint.task_id
            )),
        ));
    }
    Ok(GenerationTaskSummary {
        handle: GenerationTaskHandle {
            event_schema_version: GENERATION_EVENT_SCHEMA_VERSION,
            task_id: checkpoint.task_id.clone(),
            attempt,
            status,
        },
        started,
        updated_at_ms,
        result_ref,
        snapshot_state,
        terminal_event,
    })
}

pub(super) fn generation_task_handle(snapshot: &GenerationTaskSnapshot) -> GenerationTaskHandle {
    GenerationTaskHandle {
        event_schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: snapshot.task_id.clone(),
        attempt: snapshot.attempt,
        status: snapshot.status,
    }
}
