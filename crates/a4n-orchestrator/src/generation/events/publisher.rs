use super::*;

#[async_trait]
pub(crate) trait GenerationSequenceLeaseReplenisher: Send + Sync {
    async fn replenish(
        &self,
        task_id: &str,
        attempt: u32,
        expected_checkpoint_revision: i64,
        current_reserved_through_sequence: u64,
        block_size: u64,
    ) -> Result<GenerationSequenceLeaseUpdate, GenerationSessionError>;
}

#[derive(Debug, Clone)]
pub(super) enum OrdinaryInput {
    Progress(ProgressEvent),
    Text { stream_id: String, delta: String },
    Usage(UsageEvent),
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ApplyMode {
    Ordinary,
    Committed,
}

pub(super) enum OrdinaryAction {
    Published,
    Wait,
    Replenish,
}

pub(super) fn append_ordinary_locked(
    state: &mut SessionState,
    config: &GenerationSessionConfig,
    capacity: &SessionCapacityLease,
    latest_sequence: &watch::Sender<u64>,
    input: OrdinaryInput,
) -> Result<(), GenerationSessionError> {
    let sequence = state
        .snapshot
        .covers_through_sequence
        .checked_add(1)
        .ok_or(GenerationSessionError::InvalidSequenceLease {
            current: state.snapshot.covers_through_sequence,
            next: u64::MAX,
        })?;
    let event = match input {
        OrdinaryInput::Progress(event) => GenerationEvent::Progress(event),
        OrdinaryInput::Usage(event) => GenerationEvent::Usage(event),
        OrdinaryInput::Text { stream_id, delta } => {
            let offset = state
                .snapshot
                .streams
                .iter()
                .find(|stream| stream.stream_id == stream_id)
                .map(|stream| stream.next_offset)
                .unwrap_or(0);
            let chunk_index = state
                .next_chunk_indexes
                .get(&stream_id)
                .copied()
                .unwrap_or(0);
            GenerationEvent::TextDelta(TextDeltaEvent {
                stream_id,
                offset,
                delta,
                chunk_index,
            })
        }
    };
    let envelope = GenerationTaskEvent {
        schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: state.snapshot.task_id.clone(),
        attempt: state.snapshot.attempt,
        sequence,
        emitted_at_ms: now_ms()?,
        event,
    };
    let pending_memory_bytes = capacity
        .current_memory()
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(MAX_GENERATION_EVENT_BYTES * 2))
        .ok_or(GenerationSessionError::SessionCapacityExceeded)?;
    let memory = capacity.reserve_memory(pending_memory_bytes)?;
    let mut next_snapshot = state.snapshot.clone();
    let mut next_chunk_indexes = state.next_chunk_indexes.clone();
    apply_event(
        &mut next_snapshot,
        &mut next_chunk_indexes,
        &envelope,
        ApplyMode::Ordinary,
    )?;

    let encoded_bytes = encoded_event_bytes(&envelope)?;
    let mut next_replay = state.replay.clone();
    let mut next_replay_bytes = state
        .replay_bytes
        .checked_add(encoded_bytes)
        .ok_or(GenerationSessionError::SessionCapacityExceeded)?;
    let next_unpersisted_event_bytes = state
        .unpersisted_event_bytes
        .checked_add(encoded_bytes)
        .ok_or(GenerationSessionError::SessionCapacityExceeded)?;
    next_replay.push_back(envelope.clone());
    trim_replay(
        &mut next_replay,
        &mut next_replay_bytes,
        state.persisted_covers_through_sequence,
        config,
    )?;
    let memory_bytes = session_memory_bytes(&next_snapshot, next_replay_bytes, config)?;
    if memory_bytes > pending_memory_bytes {
        return Err(GenerationSessionError::SessionCapacityExceeded);
    }

    state.snapshot = next_snapshot;
    state.next_chunk_indexes = next_chunk_indexes;
    state.replay = next_replay;
    state.replay_bytes = next_replay_bytes;
    state.unpersisted_event_bytes = next_unpersisted_event_bytes;
    memory.commit(memory_bytes);
    latest_sequence.send_replace(sequence);
    Ok(())
}

pub(super) fn apply_event(
    snapshot: &mut GenerationTaskSnapshot,
    next_chunk_indexes: &mut HashMap<String, u64>,
    envelope: &GenerationTaskEvent,
    mode: ApplyMode,
) -> Result<(), GenerationSessionError> {
    envelope
        .validate()
        .map_err(|error| GenerationSessionError::Contract(error.to_string()))?;
    if envelope.task_id != snapshot.task_id || envelope.attempt != snapshot.attempt {
        return Err(GenerationSessionError::InvalidSession(
            "事件 task_id/attempt 与 snapshot 不一致".to_string(),
        ));
    }
    let expected_sequence = snapshot.covers_through_sequence.checked_add(1).ok_or(
        GenerationSessionError::InvalidSequenceLease {
            current: snapshot.covers_through_sequence,
            next: u64::MAX,
        },
    )?;
    if envelope.sequence != expected_sequence {
        return Err(GenerationSessionError::InvalidSession(format!(
            "事件 sequence={}，预期={expected_sequence}",
            envelope.sequence
        )));
    }

    let status = snapshot.status;
    match &envelope.event {
        GenerationEvent::Started(_) => {
            return Err(GenerationSessionError::InvalidState {
                status,
                operation: "started",
            });
        }
        GenerationEvent::StatusChanged(event) => {
            let expected = matches!(
                (status, event.status),
                (
                    GenerationTaskStatus::Reserved,
                    GenerationTaskStatus::Running
                ) | (
                    GenerationTaskStatus::Running,
                    GenerationTaskStatus::Finalizing
                )
            );
            if !expected {
                return Err(GenerationSessionError::InvalidState {
                    status,
                    operation: "status_changed",
                });
            }
            snapshot.status = event.status;
        }
        GenerationEvent::Progress(event) => {
            if matches!(mode, ApplyMode::Ordinary)
                && !matches!(
                    status,
                    GenerationTaskStatus::Reserved | GenerationTaskStatus::Running
                )
            {
                return Err(GenerationSessionError::InvalidState {
                    status,
                    operation: "progress",
                });
            }
            let mut progress = event.clone();
            // Model-connect/stream updates intentionally omit counters. Keep the
            // last known pair in the durable snapshot so reconnect/resume does not
            // turn a counted workflow into an apparent 0/1 task.
            if let Some(previous) = snapshot.progress.as_ref() {
                progress.current = progress.current.or(previous.current);
                progress.total = progress.total.or(previous.total);
            }
            snapshot.progress = Some(progress);
        }
        GenerationEvent::TextDelta(event) => {
            if matches!(mode, ApplyMode::Ordinary)
                && !matches!(
                    status,
                    GenerationTaskStatus::Reserved | GenerationTaskStatus::Running
                )
            {
                return Err(GenerationSessionError::InvalidState {
                    status,
                    operation: "text_delta",
                });
            }
            if matches!(mode, ApplyMode::Committed)
                && !matches!(
                    status,
                    GenerationTaskStatus::Reserved
                        | GenerationTaskStatus::Running
                        | GenerationTaskStatus::Finalizing
                        | GenerationTaskStatus::Cancelling
                )
            {
                return Err(GenerationSessionError::InvalidState {
                    status,
                    operation: "text_delta",
                });
            }
            apply_text_delta(snapshot, next_chunk_indexes, event)?;
        }
        GenerationEvent::Usage(event) => {
            if matches!(mode, ApplyMode::Ordinary)
                && !matches!(
                    status,
                    GenerationTaskStatus::Reserved | GenerationTaskStatus::Running
                )
            {
                return Err(GenerationSessionError::InvalidState {
                    status,
                    operation: "usage",
                });
            }
            if matches!(mode, ApplyMode::Committed)
                && !matches!(
                    status,
                    GenerationTaskStatus::Reserved
                        | GenerationTaskStatus::Running
                        | GenerationTaskStatus::Finalizing
                )
            {
                return Err(GenerationSessionError::InvalidState {
                    status,
                    operation: "usage",
                });
            }
            apply_usage(snapshot, &event.usage)?;
        }
        GenerationEvent::Checkpoint(event) => {
            if status.is_terminal() {
                return Err(GenerationSessionError::TerminalAlreadySet);
            }
            snapshot.latest_checkpoint = Some(event.clone());
        }
        GenerationEvent::Artifact(event) => {
            if status.is_terminal() {
                return Err(GenerationSessionError::TerminalAlreadySet);
            }
            apply_artifact(snapshot, event)?;
        }
        GenerationEvent::CancelRequested(CancelRequestedEvent { reason }) => {
            if !matches!(
                status,
                GenerationTaskStatus::Reserved | GenerationTaskStatus::Running
            ) {
                return Err(GenerationSessionError::InvalidState {
                    status,
                    operation: "cancel_requested",
                });
            }
            snapshot.status = GenerationTaskStatus::Cancelling;
            snapshot.cancel_reason = Some(*reason);
        }
        GenerationEvent::Completed(_) => {
            if status != GenerationTaskStatus::Finalizing {
                return Err(GenerationSessionError::InvalidState {
                    status,
                    operation: "completed",
                });
            }
            snapshot.status = GenerationTaskStatus::Completed;
            snapshot.cancel_reason = None;
            snapshot.terminal_event = Some(envelope.clone());
        }
        GenerationEvent::Failed(_) => {
            if status.is_terminal() {
                return Err(GenerationSessionError::TerminalAlreadySet);
            }
            snapshot.status = GenerationTaskStatus::Failed;
            snapshot.cancel_reason = None;
            snapshot.terminal_event = Some(envelope.clone());
        }
        GenerationEvent::Cancelled(_) => {
            if status != GenerationTaskStatus::Cancelling {
                return Err(GenerationSessionError::InvalidState {
                    status,
                    operation: "cancelled",
                });
            }
            snapshot.status = GenerationTaskStatus::Cancelled;
            snapshot.cancel_reason = None;
            snapshot.terminal_event = Some(envelope.clone());
        }
        GenerationEvent::Interrupted(_) => {
            if status.is_terminal() {
                return Err(GenerationSessionError::TerminalAlreadySet);
            }
            snapshot.status = GenerationTaskStatus::Interrupted;
            snapshot.cancel_reason = None;
            snapshot.terminal_event = Some(envelope.clone());
        }
    }
    snapshot.covers_through_sequence = envelope.sequence;
    snapshot.updated_at_ms = envelope.emitted_at_ms;
    snapshot
        .validate()
        .map_err(|error| GenerationSessionError::Contract(error.to_string()))
}

pub(crate) fn apply_committed_envelope_to_snapshot(
    snapshot: &mut GenerationTaskSnapshot,
    envelope: &GenerationTaskEvent,
) -> Result<(), GenerationSessionError> {
    let mut next_chunk_indexes = HashMap::new();
    apply_event(
        snapshot,
        &mut next_chunk_indexes,
        envelope,
        ApplyMode::Committed,
    )
}

fn apply_text_delta(
    snapshot: &mut GenerationTaskSnapshot,
    next_chunk_indexes: &mut HashMap<String, u64>,
    event: &TextDeltaEvent,
) -> Result<(), GenerationSessionError> {
    let stream = snapshot
        .streams
        .iter_mut()
        .find(|stream| stream.stream_id == event.stream_id);
    let stream = if let Some(stream) = stream {
        stream
    } else {
        if event.offset != 0 {
            return Err(GenerationSessionError::OffsetMismatch {
                stream_id: event.stream_id.clone(),
                expected: 0,
                actual: event.offset,
            });
        }
        snapshot.streams.push(TextStreamSnapshot {
            stream_id: event.stream_id.clone(),
            text: String::new(),
            next_offset: 0,
            sha256: format!("{:x}", Sha256::digest([])),
        });
        snapshot.streams.last_mut().expect("stream inserted")
    };
    if stream.next_offset != event.offset {
        return Err(GenerationSessionError::OffsetMismatch {
            stream_id: event.stream_id.clone(),
            expected: stream.next_offset,
            actual: event.offset,
        });
    }
    stream.text.push_str(&event.delta);
    stream.next_offset = stream.text.len() as u64;
    stream.sha256 = format!("{:x}", Sha256::digest(stream.text.as_bytes()));
    next_chunk_indexes.insert(event.stream_id.clone(), event.chunk_index.saturating_add(1));
    Ok(())
}

fn apply_usage(
    snapshot: &mut GenerationTaskSnapshot,
    usage: &GenerationUsage,
) -> Result<(), GenerationSessionError> {
    if let Some(previous) = &snapshot.usage
        && (usage.prompt_tokens < previous.prompt_tokens
            || usage.completion_tokens < previous.completion_tokens
            || usage.total_tokens < previous.total_tokens
            || optional_value_decreased(
                previous.prompt_cache_hit_tokens,
                usage.prompt_cache_hit_tokens,
            )
            || optional_value_decreased(
                previous.prompt_cache_miss_tokens,
                usage.prompt_cache_miss_tokens,
            )
            || optional_value_decreased(
                previous.prompt_cache_write_tokens,
                usage.prompt_cache_write_tokens,
            ))
    {
        return Err(GenerationSessionError::UsageRegression);
    }
    snapshot.usage = Some(usage.clone());
    Ok(())
}

fn optional_value_decreased(previous: Option<u64>, next: Option<u64>) -> bool {
    match (previous, next) {
        (Some(previous), Some(next)) => next < previous,
        (Some(_), None) => true,
        _ => false,
    }
}

fn apply_artifact(
    snapshot: &mut GenerationTaskSnapshot,
    event: &ArtifactEvent,
) -> Result<(), GenerationSessionError> {
    let artifact_id = event.artifact.artifact_id.as_str();
    match event.operation {
        ArtifactOperation::PreviewAvailable | ArtifactOperation::Available => {
            if snapshot
                .artifacts
                .iter()
                .any(|artifact| artifact.artifact_id == artifact_id)
            {
                return Err(GenerationSessionError::InvalidSession(format!(
                    "artifact_id 重复: {artifact_id}"
                )));
            }
            snapshot.artifacts.push(event.artifact.clone());
        }
        ArtifactOperation::Updated => {
            let replaced = event
                .replaces_artifact_id
                .as_deref()
                .expect("updated artifact contract validated");
            let Some(index) = snapshot
                .artifacts
                .iter()
                .position(|artifact| artifact.artifact_id == replaced)
            else {
                return Err(GenerationSessionError::InvalidSession(format!(
                    "updated artifact 不存在: {replaced}"
                )));
            };
            if snapshot
                .artifacts
                .iter()
                .any(|artifact| artifact.artifact_id == artifact_id)
            {
                return Err(GenerationSessionError::InvalidSession(format!(
                    "updated artifact_id 已存在: {artifact_id}"
                )));
            }
            snapshot.artifacts.remove(index);
            snapshot.artifacts.push(event.artifact.clone());
        }
        ArtifactOperation::Removed => {
            let Some(index) = snapshot
                .artifacts
                .iter()
                .position(|artifact| artifact.artifact_id == artifact_id)
            else {
                return Err(GenerationSessionError::InvalidSession(format!(
                    "removed artifact 不存在: {artifact_id}"
                )));
            };
            snapshot.artifacts.remove(index);
        }
    }
    Ok(())
}

pub(crate) struct CommittedEventPermit {
    pub(super) session: Arc<GenerationTaskSession>,
    pub(super) events: Vec<GenerationTaskEvent>,
    pub(super) snapshot: Option<GenerationTaskSnapshot>,
    pub(super) replay: Option<VecDeque<GenerationTaskEvent>>,
    pub(super) replay_bytes: usize,
    pub(super) next_chunk_indexes: Option<HashMap<String, u64>>,
    pub(super) first_sequence: u64,
    pub(super) last_sequence: u64,
    pub(super) committed_memory_bytes: usize,
    pub(super) memory: Option<MemoryReservation>,
    pub(super) committed: bool,
}

impl CommittedEventPermit {
    #[cfg(test)]
    pub(super) fn events(&self) -> &[GenerationTaskEvent] {
        &self.events
    }

    pub(crate) fn snapshot(&self) -> &GenerationTaskSnapshot {
        self.snapshot
            .as_ref()
            .expect("committed event permit snapshot consumed")
    }

    #[cfg(test)]
    pub(super) fn first_sequence(&self) -> u64 {
        self.first_sequence
    }

    #[cfg(test)]
    pub(super) fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    pub(crate) fn commit(
        mut self,
        commit: GenerationDurableCommit,
    ) -> Result<(), GenerationSessionError> {
        validate_durable_commit(commit)?;
        if commit.covers_through_sequence != self.last_sequence {
            return Err(GenerationSessionError::InvalidSession(format!(
                "durable commit covers={}，permit last_sequence={}",
                commit.covers_through_sequence, self.last_sequence
            )));
        }
        let mut state = self
            .session
            .state
            .lock()
            .expect("generation session mutex poisoned");
        if !state.durable_commit_in_flight
            || state.snapshot.covers_through_sequence.saturating_add(1) != self.first_sequence
            || commit.checkpoint_revision <= state.checkpoint_revision
        {
            return Err(GenerationSessionError::InvalidSession(
                "committed event permit 已失效".to_string(),
            ));
        }
        let prepared_snapshot = self
            .snapshot
            .take()
            .expect("committed event permit snapshot exists");
        let prepared_replay = self
            .replay
            .take()
            .expect("committed event permit replay exists");
        let prepared_chunk_indexes = self
            .next_chunk_indexes
            .take()
            .expect("committed event permit chunk indexes exist");
        let old_snapshot = std::mem::replace(&mut state.snapshot, prepared_snapshot);
        let old_replay = std::mem::replace(&mut state.replay, prepared_replay);
        let old_chunk_indexes =
            std::mem::replace(&mut state.next_chunk_indexes, prepared_chunk_indexes);
        state.replay_bytes = self.replay_bytes;
        state.checkpoint_revision = commit.checkpoint_revision;
        state.persisted_covers_through_sequence = self.last_sequence;
        state.reserved_through_sequence = state
            .reserved_through_sequence
            .max(commit.reserved_through_sequence);
        state.last_persisted_at_ms = commit.persisted_at_ms;
        state.unpersisted_event_bytes = 0;
        if state.snapshot_only_covers.is_some() {
            state.snapshot_only_covers = Some(commit.covers_through_sequence);
        }
        state.durable_commit_in_flight = false;
        if state.snapshot.status.is_terminal() {
            self.session.capacity.release_active_slot();
        }
        self.events.clear();
        drop(old_snapshot);
        drop(old_replay);
        drop(old_chunk_indexes);
        if let Some(memory) = self.memory.take() {
            memory.commit(self.committed_memory_bytes);
        }
        self.session
            .latest_sequence
            .send_replace(self.last_sequence);
        self.committed = true;
        drop(state);
        self.session.durable_commit_notify.notify_waiters();
        Ok(())
    }
}

impl Drop for CommittedEventPermit {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        let mut state = self
            .session
            .state
            .lock()
            .expect("generation session mutex poisoned");
        state.durable_commit_in_flight = false;
        drop(state);
        self.events.clear();
        self.snapshot.take();
        self.replay.take();
        self.next_chunk_indexes.take();
        self.memory.take();
        self.session.durable_commit_notify.notify_waiters();
    }
}

#[async_trait]
pub(crate) trait GenerationEventPublisher: Send + Sync {
    async fn progress(&self, event: ProgressEvent) -> Result<(), GenerationSessionError>;
    async fn text_delta(&self, stream_id: &str, delta: &str) -> Result<(), GenerationSessionError>;
    async fn usage(&self, event: UsageEvent) -> Result<(), GenerationSessionError>;
}

#[derive(Clone)]
pub(crate) struct SessionGenerationEventPublisher {
    session: Arc<GenerationTaskSession>,
    replenisher: Arc<dyn GenerationSequenceLeaseReplenisher>,
}

impl SessionGenerationEventPublisher {
    pub(crate) fn new(
        session: Arc<GenerationTaskSession>,
        replenisher: Arc<dyn GenerationSequenceLeaseReplenisher>,
    ) -> Self {
        Self {
            session,
            replenisher,
        }
    }
}

#[async_trait]
impl GenerationEventPublisher for SessionGenerationEventPublisher {
    async fn progress(&self, event: ProgressEvent) -> Result<(), GenerationSessionError> {
        self.session
            .publish_ordinary(OrdinaryInput::Progress(event), self.replenisher.as_ref())
            .await
    }

    async fn text_delta(&self, stream_id: &str, delta: &str) -> Result<(), GenerationSessionError> {
        self.session
            .publish_ordinary(
                OrdinaryInput::Text {
                    stream_id: stream_id.to_string(),
                    delta: delta.to_string(),
                },
                self.replenisher.as_ref(),
            )
            .await
    }

    async fn usage(&self, event: UsageEvent) -> Result<(), GenerationSessionError> {
        self.session
            .publish_ordinary(OrdinaryInput::Usage(event), self.replenisher.as_ref())
            .await
    }
}
