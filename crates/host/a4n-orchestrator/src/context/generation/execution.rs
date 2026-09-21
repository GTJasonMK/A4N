//! Generation execution and terminal settlement.

use super::*;

/// 已取得 generation queue execution permit 的运行上下文。
pub(crate) struct GenerationTaskExecution {
    pub(crate) session: Arc<GenerationTaskSession>,
    pub(crate) publisher: SessionGenerationEventPublisher,
    pub(crate) checkpoint_writer: GenerationCheckpointWriter,
    pub(crate) terminal: GenerationTerminalCoordinator,
    pub(crate) permit: TaskPermit,
}

impl GenerationTaskExecution {
    pub(crate) async fn complete_with_result(
        self,
        operation_key: &str,
        result_ref: GenerationResultRef,
        summary: String,
        artifacts: Vec<PreparedArtifactEvent>,
        batch_summary: Option<BatchSummary>,
    ) -> Result<(), String> {
        let snapshot = self.session.snapshot();
        let final_text_hashes = snapshot_text_hashes(&snapshot.streams);
        let final_usage = snapshot.usage;
        let failure_batch_summary = batch_summary.clone();
        let failure_artifacts = artifacts.clone();
        let completion = self
            .terminal
            .complete_with_artifacts(&self.session, artifacts, move |_tx| {
                Box::pin(async move {
                    Ok(CompletionDescriptor {
                        result_ref: Some(result_ref),
                        final_text_hashes,
                        final_usage,
                        finish_reason: Some(GenerationFinishReason::Stop),
                        summary: Some(summary),
                        batch_summary,
                    })
                }) as BoxFuture<'_, Result<CompletionDescriptor, AppError>>
            })
            .await;
        if let Err(error) = completion {
            return Err(self
                .fail_with_batch_summary(
                    crate::generation::generation_public_error(error.code(), operation_key, error),
                    failure_batch_summary,
                    failure_artifacts,
                )
                .await);
        }
        self.finish();
        Ok(())
    }

    pub(crate) fn finish(self) {
        settle_generation_permit(
            self.permit,
            self.session.snapshot().status,
            "generation task execution 已结束".to_string(),
        );
    }

    pub(crate) async fn fail(self, error: GenerationError) -> String {
        self.fail_with_batch_summary(error, None, Vec::new()).await
    }

    pub(crate) async fn fail_with_batch_summary(
        self,
        error: GenerationError,
        batch_summary: Option<BatchSummary>,
        artifacts: Vec<PreparedArtifactEvent>,
    ) -> String {
        let Self {
            session,
            terminal,
            permit,
            ..
        } = self;
        let session_status = session.snapshot().status;
        let terminal_result = match session_status {
            GenerationTaskStatus::Cancelling => {
                terminal
                    .cancel(&session, None, batch_summary.clone(), artifacts)
                    .await
            }
            _ if generation_failure_is_cancelled_or_interrupted(session_status, &error) => {
                terminal
                    .interrupt(
                        &session,
                        InterruptionReason::SessionOwnerLost,
                        Some(error.message.clone()),
                        batch_summary.clone(),
                        artifacts,
                    )
                    .await
            }
            _ => {
                terminal
                    .fail(&session, error.clone(), batch_summary, artifacts)
                    .await
            }
        };
        let final_status = session.snapshot().status;
        settle_generation_permit(permit, final_status, error.message.clone());

        let mut message = error.message;
        if let Err(terminal_error) = terminal_result {
            message.push_str(&format!("; generation 终态写入失败: {terminal_error}"));
        }
        message
    }
}

pub(crate) async fn fail_generation_workflow_execution(
    orchestrator: &OrchestratorContext,
    execution: GenerationTaskExecution,
    workflow_run_id: Option<i64>,
    error: GenerationError,
) -> Result<(), String> {
    let cancelled_or_interrupted =
        generation_failure_is_cancelled_or_interrupted(execution.session.snapshot().status, &error);
    let workflow_error = if let Some(workflow_run_id) = workflow_run_id {
        let result = if cancelled_or_interrupted {
            orchestrator
                .cancel_workflow_run_status_validated(workflow_run_id, &error.message)
                .await
        } else {
            orchestrator
                .fail_workflow_run_validated(workflow_run_id, &error.message)
                .await
        };
        result.err().map(|error| (workflow_run_id, error))
    } else {
        None
    };
    let mut message = execution.fail(error).await;
    if let Some((workflow_run_id, workflow_error)) = workflow_error {
        message.push_str(&format!(
            "; generation workflow {workflow_run_id} 终态写入失败: {workflow_error}"
        ));
    }
    Err(message)
}

fn settle_generation_permit(permit: TaskPermit, status: GenerationTaskStatus, message: String) {
    match status {
        GenerationTaskStatus::Completed => permit.complete(),
        GenerationTaskStatus::Cancelled => permit.cancel(),
        GenerationTaskStatus::Interrupted => permit.interrupt(Some(message)),
        GenerationTaskStatus::Failed => permit.fail(message),
        status => permit.fail(format!(
            "{message}; generation lifecycle 未进入终态: {status:?}"
        )),
    }
}

fn snapshot_text_hashes(streams: &[TextStreamSnapshot]) -> Vec<StreamTextHash> {
    streams
        .iter()
        .map(|stream| StreamTextHash {
            stream_id: stream.stream_id.clone(),
            utf8_bytes: stream.next_offset,
            sha256: stream.sha256.clone(),
        })
        .collect()
}

impl GenerationApplication {
    pub(crate) async fn acquire_task_execution(
        &self,
        handle: &GenerationTaskHandle,
    ) -> Result<Option<GenerationTaskExecution>, GenerationTaskAccessError> {
        let session = self
            .session(&handle.task_id, handle.attempt)?
            .ok_or(GenerationTaskAccessError::SnapshotRequired)?;
        let Some(permit) = self
            .task_queue
            .acquire_generation_task(&handle.task_id)
            .await
            .map_err(GenerationTaskAccessError::Queue)?
        else {
            return Ok(None);
        };
        match self.terminal.begin_running(&session).await {
            Ok(GenerationTerminalWriteOutcome::Persisted)
            | Ok(GenerationTerminalWriteOutcome::RebuiltFromPersistedSnapshot) => {
                let publisher = SessionGenerationEventPublisher::new(
                    session.clone(),
                    Arc::new(DatabaseGenerationSequenceLeaseReplenisher::new(
                        self.database.shared_connection(),
                    )),
                );
                Ok(Some(GenerationTaskExecution {
                    session,
                    publisher,
                    checkpoint_writer: GenerationCheckpointWriter::new(
                        self.database.shared_connection(),
                    ),
                    terminal: self.terminal.clone(),
                    permit,
                }))
            }
            Ok(GenerationTerminalWriteOutcome::AlreadyTerminal)
            | Ok(GenerationTerminalWriteOutcome::NoLiveSession) => {
                settle_generation_permit(
                    permit,
                    session.snapshot().status,
                    "generation task 在取得执行权前已结束".to_string(),
                );
                Ok(None)
            }
            Err(error) => {
                permit.fail(error.to_string());
                Err(GenerationTaskAccessError::Terminal(error))
            }
        }
    }
}

impl OrchestratorContext {
    pub(crate) async fn acquire_generation_task_execution(
        &self,
        handle: &GenerationTaskHandle,
    ) -> Result<Option<GenerationTaskExecution>, GenerationTaskAccessError> {
        self.generation.acquire_task_execution(handle).await
    }
}
#[cfg(test)]
mod completion_hash_tests {
    use super::*;

    fn settled_permit_status(status: GenerationTaskStatus) -> TaskStatus {
        let settled = Arc::new(std::sync::Mutex::new(None));
        let observed = Arc::clone(&settled);
        let permit = TaskPermit::new(
            "00000000-0000-4000-8000-000000000001".to_string(),
            TaskKind::LlmCall,
            TaskPriority::High,
            Box::new(move |status| {
                *observed.lock().expect("permit status mutex poisoned") = Some(status);
            }),
        );

        settle_generation_permit(permit, status, "terminal summary".to_string());

        Arc::try_unwrap(settled)
            .expect("permit status observer still shared")
            .into_inner()
            .expect("permit status mutex poisoned")
            .expect("permit was not settled")
    }

    #[test]
    fn completion_hashes_cover_every_snapshot_stream() {
        let streams = vec![
            TextStreamSnapshot {
                stream_id: "raw_json".to_string(),
                text: "{}".to_string(),
                next_offset: 2,
                sha256: "a".repeat(64),
            },
            TextStreamSnapshot {
                stream_id: "notes".to_string(),
                text: "ok".to_string(),
                next_offset: 2,
                sha256: "b".repeat(64),
            },
        ];

        let hashes = snapshot_text_hashes(&streams);

        assert_eq!(hashes.len(), streams.len());
        for (hash, stream) in hashes.iter().zip(&streams) {
            assert_eq!(hash.stream_id, stream.stream_id);
            assert_eq!(hash.utf8_bytes, stream.next_offset);
            assert_eq!(hash.sha256, stream.sha256);
        }
    }

    #[test]
    fn generation_permit_settlement_mirrors_terminal_status() {
        assert!(matches!(
            settled_permit_status(GenerationTaskStatus::Completed),
            TaskStatus::Completed
        ));
        assert!(matches!(
            settled_permit_status(GenerationTaskStatus::Failed),
            TaskStatus::Failed { error } if error == "terminal summary"
        ));
        assert!(matches!(
            settled_permit_status(GenerationTaskStatus::Cancelled),
            TaskStatus::Cancelled
        ));
        assert!(matches!(
            settled_permit_status(GenerationTaskStatus::Interrupted),
            TaskStatus::Interrupted { reason: Some(reason) } if reason == "terminal summary"
        ));
        assert!(matches!(
            settled_permit_status(GenerationTaskStatus::Running),
            TaskStatus::Failed { error } if error.contains("未进入终态: Running")
        ));
    }
}
