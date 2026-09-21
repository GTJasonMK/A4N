use std::{future::Future, sync::Arc, time::Duration};

use a4n_db::{
    SharedDatabaseConnection,
    generation::{
        GENERATION_SEQUENCE_LEASE_SIZE, GenerationTaskPersistenceError,
        PersistGenerationCheckpointInput, PersistedGenerationCheckpoint, PersistedGenerationTask,
        extend_generation_sequence_lease, load_generation_task, persist_generation_checkpoint,
    },
};
use a4n_service_protocol::{
    CheckpointEvent, GenerationEvent, GenerationTaskStatus, MAX_TEXT_DELTA_BYTES,
};
use async_trait::async_trait;
use serde_json::Value;

use super::events::{
    GenerationDurableCommit, GenerationSequenceLeaseReplenisher, GenerationSequenceLeaseUpdate,
    GenerationSessionError, GenerationTaskSession, SessionCheckpointCandidate, now_ms,
};

const CHECKPOINT_RETRY_DELAYS: [Duration; 3] = [
    Duration::from_millis(250),
    Duration::from_secs(1),
    Duration::from_secs(3),
];

#[derive(Debug, Clone, Default)]
pub(crate) struct GenerationCheckpointMetadata {
    pub(crate) step_key: Option<String>,
    pub(crate) call_key: Option<String>,
    pub(crate) provider_kind: Option<String>,
    pub(crate) provider_model: Option<String>,
    pub(crate) provider_job_id: Option<String>,
    pub(crate) provider_idempotency_key_sha256: Option<String>,
    pub(crate) resume_metadata_version: Option<i32>,
    pub(crate) execution_checkpoint_json: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationCheckpointWriteOutcome {
    Skipped,
    Persisted {
        checkpoint_revision: i64,
        covers_through_sequence: u64,
    },
    Superseded,
    RebuiltFromPersistedSnapshot,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum GenerationCheckpointError {
    #[error(transparent)]
    Session(#[from] GenerationSessionError),
    #[error(transparent)]
    Persistence(#[from] GenerationTaskPersistenceError),
}

impl GenerationCheckpointError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Session(error) => error.code(),
            Self::Persistence(error) => error.code(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct GenerationCheckpointWriter {
    db: SharedDatabaseConnection,
}

pub(crate) struct DatabaseGenerationSequenceLeaseReplenisher {
    db: SharedDatabaseConnection,
}

impl DatabaseGenerationSequenceLeaseReplenisher {
    pub(crate) fn new(db: SharedDatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl GenerationSequenceLeaseReplenisher for DatabaseGenerationSequenceLeaseReplenisher {
    async fn replenish(
        &self,
        task_id: &str,
        _attempt: u32,
        expected_checkpoint_revision: i64,
        current_reserved_through_sequence: u64,
        block_size: u64,
    ) -> Result<GenerationSequenceLeaseUpdate, GenerationSessionError> {
        if block_size != GENERATION_SEQUENCE_LEASE_SIZE as u64 {
            return Err(GenerationSessionError::InvalidSession(format!(
                "sequence lease block 必须是 {GENERATION_SEQUENCE_LEASE_SIZE}"
            )));
        }
        let current_reserved_through_sequence = i64::try_from(current_reserved_through_sequence)
            .map_err(|_| GenerationSessionError::InvalidSequenceLease {
                current: current_reserved_through_sequence,
                next: u64::MAX,
            })?;
        let lease = extend_generation_sequence_lease(
            self.db.as_ref(),
            task_id,
            expected_checkpoint_revision,
            current_reserved_through_sequence,
        )
        .await
        .map_err(|error| GenerationSessionError::SequenceLeaseUnavailable(error.to_string()))?;
        Ok(GenerationSequenceLeaseUpdate {
            checkpoint_revision: lease.checkpoint_revision,
            reserved_through_sequence: u64::try_from(lease.reserved_through_sequence).map_err(
                |_| GenerationSessionError::InvalidSequenceLease {
                    current: current_reserved_through_sequence as u64,
                    next: u64::MAX,
                },
            )?,
        })
    }
}

impl GenerationCheckpointWriter {
    pub(crate) fn new(db: SharedDatabaseConnection) -> Self {
        Self { db }
    }

    pub(crate) async fn persist_if_due(
        &self,
        session: &Arc<GenerationTaskSession>,
        metadata: GenerationCheckpointMetadata,
        force: bool,
    ) -> Result<GenerationCheckpointWriteOutcome, GenerationCheckpointError> {
        let _guard = session.checkpoint_write_guard().await;
        let mut candidate = session.checkpoint_candidate();
        if !session.checkpoint_due(&candidate, now_ms()?, force) {
            return Ok(GenerationCheckpointWriteOutcome::Skipped);
        }
        if !ordinary_checkpoint_status(candidate.snapshot.status) {
            return Ok(GenerationCheckpointWriteOutcome::Superseded);
        }

        let mut expected_revision = candidate.checkpoint_revision;
        loop {
            let input = checkpoint_input(&candidate, expected_revision, metadata.clone());
            match retry_database(session, || {
                persist_generation_checkpoint(self.db.as_ref(), input.clone())
            })
            .await
            {
                Ok(persisted) => {
                    let commit = persisted_commit(&persisted)?;
                    session.mark_snapshot_persisted(commit)?;
                    return Ok(GenerationCheckpointWriteOutcome::Persisted {
                        checkpoint_revision: commit.checkpoint_revision,
                        covers_through_sequence: commit.covers_through_sequence,
                    });
                }
                Err(GenerationCheckpointError::Persistence(
                    GenerationTaskPersistenceError::CheckpointConflict,
                )) => {
                    let loaded = retry_database(session, || {
                        load_generation_task(self.db.as_ref(), &candidate.snapshot.task_id)
                    })
                    .await?;
                    validate_loaded(&candidate, &loaded)?;
                    let snapshot = loaded.public_snapshot.as_ref().ok_or_else(|| {
                        GenerationSessionError::InvalidSession(
                            "active generation checkpoint 缺少 public snapshot".to_string(),
                        )
                    })?;
                    if !ordinary_checkpoint_status(snapshot.status)
                        || snapshot.covers_through_sequence
                            >= candidate.snapshot.covers_through_sequence
                    {
                        return Ok(GenerationCheckpointWriteOutcome::Superseded);
                    }
                    if snapshot.status != candidate.snapshot.status {
                        return Err(GenerationSessionError::InvalidSession(
                            "数据库与 session 的 generation status 不一致".to_string(),
                        )
                        .into());
                    }
                    expected_revision = loaded.checkpoint.checkpoint_revision;
                    candidate = session.checkpoint_candidate();
                }
                Err(error) => return Err(error),
            }
        }
    }

    pub(crate) async fn persist_checkpoint_event(
        &self,
        session: &Arc<GenerationTaskSession>,
        metadata: GenerationCheckpointMetadata,
        checkpoint_key: String,
        resumable: bool,
        summary: Option<String>,
        text_delta: Option<(String, String)>,
    ) -> Result<GenerationCheckpointWriteOutcome, GenerationCheckpointError> {
        let _guard = session.checkpoint_write_guard().await;
        let text_delta = match text_delta {
            Some((stream_id, delta)) if delta.is_empty() => {
                return Err(GenerationSessionError::InvalidSession(format!(
                    "generation checkpoint stream {stream_id} delta 不能为空"
                ))
                .into());
            }
            Some((stream_id, delta)) => Some((stream_id, split_text_delta(&delta))),
            None => None,
        };
        let checkpoint = CheckpointEvent {
            checkpoint_key,
            resumable,
            summary,
        };
        let mut expected_revision = None;

        loop {
            let mut events = text_delta
                .as_ref()
                .map(|(stream_id, deltas)| {
                    session.next_committed_text_delta_events(stream_id, deltas)
                })
                .unwrap_or_default();
            events.push(GenerationEvent::Checkpoint(checkpoint.clone()));
            let permit = session.prepare_committed_events(events)?;
            if !ordinary_checkpoint_status(permit.snapshot().status) {
                return Ok(GenerationCheckpointWriteOutcome::Superseded);
            }
            let mut candidate = session.checkpoint_candidate();
            candidate.snapshot = permit.snapshot().clone();
            let input = checkpoint_input(
                &candidate,
                expected_revision.unwrap_or(candidate.checkpoint_revision),
                metadata.clone(),
            );
            match retry_database(session, || {
                persist_generation_checkpoint(self.db.as_ref(), input.clone())
            })
            .await
            {
                Ok(persisted) => {
                    let commit = persisted_commit(&persisted)?;
                    if permit.commit(commit).is_ok() {
                        return Ok(GenerationCheckpointWriteOutcome::Persisted {
                            checkpoint_revision: commit.checkpoint_revision,
                            covers_through_sequence: commit.covers_through_sequence,
                        });
                    }
                    let loaded = retry_database(session, || {
                        load_generation_task(self.db.as_ref(), &candidate.snapshot.task_id)
                    })
                    .await?;
                    rebuild_from_loaded(session, loaded)?;
                    return Ok(GenerationCheckpointWriteOutcome::RebuiltFromPersistedSnapshot);
                }
                Err(GenerationCheckpointError::Persistence(
                    GenerationTaskPersistenceError::CheckpointConflict,
                )) => {
                    drop(permit);
                    let loaded = retry_database(session, || {
                        load_generation_task(self.db.as_ref(), &candidate.snapshot.task_id)
                    })
                    .await?;
                    validate_loaded(&candidate, &loaded)?;
                    let snapshot = loaded.public_snapshot.as_ref().ok_or_else(|| {
                        GenerationSessionError::InvalidSession(
                            "active generation checkpoint 缺少 public snapshot".to_string(),
                        )
                    })?;
                    if !ordinary_checkpoint_status(snapshot.status)
                        || snapshot.latest_checkpoint.as_ref() == Some(&checkpoint)
                    {
                        return Ok(GenerationCheckpointWriteOutcome::Superseded);
                    }
                    if snapshot.covers_through_sequence > session.latest_sequence() {
                        return Err(GenerationSessionError::InvalidSession(
                            "数据库 checkpoint covers 超过当前 session".to_string(),
                        )
                        .into());
                    }
                    expected_revision = Some(loaded.checkpoint.checkpoint_revision);
                }
                Err(error) => return Err(error),
            }
        }
    }
}

pub(crate) fn split_text_delta(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut end = (start + MAX_TEXT_DELTA_BYTES).min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        parts.push(text[start..end].to_string());
        start = end;
    }
    parts
}

fn checkpoint_input(
    candidate: &SessionCheckpointCandidate,
    expected_checkpoint_revision: i64,
    metadata: GenerationCheckpointMetadata,
) -> PersistGenerationCheckpointInput {
    PersistGenerationCheckpointInput {
        task_id: candidate.snapshot.task_id.clone(),
        attempt: candidate.snapshot.attempt,
        expected_checkpoint_revision,
        public_snapshot: candidate.snapshot.clone(),
        step_key: metadata.step_key,
        call_key: metadata.call_key,
        provider_kind: metadata.provider_kind,
        provider_model: metadata.provider_model,
        provider_job_id: metadata.provider_job_id,
        provider_idempotency_key_sha256: metadata.provider_idempotency_key_sha256,
        resume_metadata_version: metadata.resume_metadata_version,
        execution_checkpoint_json: metadata.execution_checkpoint_json,
    }
}

fn ordinary_checkpoint_status(status: GenerationTaskStatus) -> bool {
    matches!(
        status,
        GenerationTaskStatus::Reserved
            | GenerationTaskStatus::Running
            | GenerationTaskStatus::Cancelling
    )
}

fn validate_loaded(
    candidate: &SessionCheckpointCandidate,
    loaded: &PersistedGenerationTask,
) -> Result<(), GenerationCheckpointError> {
    let attempt = u32::try_from(loaded.checkpoint.attempt).map_err(|_| {
        GenerationSessionError::InvalidSession("数据库 generation attempt 为负数".to_string())
    })?;
    let Some(snapshot) = loaded.public_snapshot.as_ref() else {
        return Ok(());
    };
    if loaded.checkpoint.task_id != candidate.snapshot.task_id
        || attempt != candidate.snapshot.attempt
        || snapshot.started != candidate.snapshot.started
    {
        return Err(GenerationSessionError::InvalidSession(
            "数据库 checkpoint 与当前 session identity 不一致".to_string(),
        )
        .into());
    }
    Ok(())
}

fn persisted_commit(
    persisted: &PersistedGenerationCheckpoint,
) -> Result<GenerationDurableCommit, GenerationCheckpointError> {
    Ok(GenerationDurableCommit {
        checkpoint_revision: persisted.checkpoint_revision,
        covers_through_sequence: u64::try_from(persisted.covers_through_sequence).map_err(
            |_| {
                GenerationSessionError::InvalidSession(
                    "数据库 checkpoint covers 为负数".to_string(),
                )
            },
        )?,
        reserved_through_sequence: u64::try_from(persisted.reserved_through_sequence).map_err(
            |_| {
                GenerationSessionError::InvalidSession(
                    "数据库 checkpoint high-water 为负数".to_string(),
                )
            },
        )?,
        persisted_at_ms: persisted.updated_at.and_utc().timestamp_millis(),
    })
}

fn rebuild_from_loaded(
    session: &GenerationTaskSession,
    loaded: PersistedGenerationTask,
) -> Result<(), GenerationCheckpointError> {
    let snapshot = loaded.public_snapshot.ok_or_else(|| {
        GenerationSessionError::InvalidSession(
            "已提交 generation checkpoint 缺少 public snapshot".to_string(),
        )
    })?;
    let commit = GenerationDurableCommit {
        checkpoint_revision: loaded.checkpoint.checkpoint_revision,
        covers_through_sequence: u64::try_from(loaded.checkpoint.covers_through_sequence).map_err(
            |_| {
                GenerationSessionError::InvalidSession(
                    "数据库 checkpoint covers 为负数".to_string(),
                )
            },
        )?,
        reserved_through_sequence: u64::try_from(loaded.checkpoint.reserved_through_sequence)
            .map_err(|_| {
                GenerationSessionError::InvalidSession(
                    "数据库 checkpoint high-water 为负数".to_string(),
                )
            })?,
        persisted_at_ms: loaded.checkpoint.updated_at.and_utc().timestamp_millis(),
    };
    session.rebuild_from_persisted_snapshot(snapshot, commit)?;
    Ok(())
}

async fn retry_database<T, F, Fut>(
    session: &GenerationTaskSession,
    mut operation: F,
) -> Result<T, GenerationCheckpointError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, GenerationTaskPersistenceError>>,
{
    let mut retries = CHECKPOINT_RETRY_DELAYS.into_iter();
    loop {
        match operation().await {
            Ok(value) => return Ok(value),
            Err(error @ GenerationTaskPersistenceError::Database(_)) => {
                let Some(delay) = retries.next() else {
                    let reason = format!("generation checkpoint database unavailable: {error}");
                    session.seal_ordinary_publisher(reason.clone());
                    return Err(GenerationSessionError::PublisherSealed(reason).into());
                };
                tokio::time::sleep(delay).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_schedule_and_active_statuses_are_fixed() {
        assert_eq!(
            CHECKPOINT_RETRY_DELAYS,
            [
                Duration::from_millis(250),
                Duration::from_secs(1),
                Duration::from_secs(3),
            ]
        );
        assert!(ordinary_checkpoint_status(GenerationTaskStatus::Reserved));
        assert!(ordinary_checkpoint_status(GenerationTaskStatus::Running));
        assert!(ordinary_checkpoint_status(GenerationTaskStatus::Cancelling));
        assert!(!ordinary_checkpoint_status(
            GenerationTaskStatus::Finalizing
        ));
        assert!(!ordinary_checkpoint_status(GenerationTaskStatus::Completed));
    }

    #[test]
    fn checkpoint_text_delta_splits_without_breaking_utf8() {
        let text = "镜".repeat(MAX_TEXT_DELTA_BYTES / "镜".len() + 2);
        let parts = split_text_delta(&text);

        assert!(parts.iter().all(|part| part.len() <= MAX_TEXT_DELTA_BYTES));
        assert_eq!(parts.concat(), text);
    }
}
