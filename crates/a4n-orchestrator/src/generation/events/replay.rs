use super::*;

pub(super) fn validate_replay(
    snapshot: &GenerationTaskSnapshot,
    replay: &VecDeque<GenerationTaskEvent>,
) -> Result<(), GenerationSessionError> {
    let mut previous = None;
    for event in replay {
        event
            .validate()
            .map_err(|error| GenerationSessionError::Contract(error.to_string()))?;
        if event.task_id != snapshot.task_id || event.attempt != snapshot.attempt {
            return Err(GenerationSessionError::InvalidSession(
                "replay task_id/attempt 与 snapshot 不一致".to_string(),
            ));
        }
        if previous.is_some_and(|sequence| event.sequence != sequence + 1) {
            return Err(GenerationSessionError::InvalidSession(
                "replay sequence 必须连续".to_string(),
            ));
        }
        if event.sequence > snapshot.covers_through_sequence {
            return Err(GenerationSessionError::InvalidSession(
                "replay 不能越过 snapshot covers".to_string(),
            ));
        }
        previous = Some(event.sequence);
    }
    if previous.is_some_and(|sequence| sequence != snapshot.covers_through_sequence) {
        return Err(GenerationSessionError::InvalidSession(
            "非空 replay 必须结束于 snapshot covers".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn replay_chunk_indexes(replay: &VecDeque<GenerationTaskEvent>) -> HashMap<String, u64> {
    let mut indexes = HashMap::new();
    for event in replay {
        if let GenerationEvent::TextDelta(delta) = &event.event {
            indexes.insert(delta.stream_id.clone(), delta.chunk_index.saturating_add(1));
        }
    }
    indexes
}

pub(super) fn encoded_event_bytes(
    event: &GenerationTaskEvent,
) -> Result<usize, GenerationSessionError> {
    serde_json::to_vec(event)
        .map(|bytes| bytes.len())
        .map_err(|error| GenerationSessionError::Contract(error.to_string()))
}

pub(super) fn session_memory_bytes(
    snapshot: &GenerationTaskSnapshot,
    replay_bytes: usize,
    config: &GenerationSessionConfig,
) -> Result<usize, GenerationSessionError> {
    // ponytail: exact serialization is deliberately simple; replace with incremental accounting
    // only if the generation memory benchmark shows this low-frequency boundary is material.
    let snapshot_bytes = serde_json::to_vec(snapshot)
        .map_err(|error| GenerationSessionError::Contract(error.to_string()))?
        .len();
    Ok(config
        .session_admission_base_bytes
        .max(snapshot_bytes.saturating_add(replay_bytes)))
}

pub(super) fn trim_replay(
    replay: &mut VecDeque<GenerationTaskEvent>,
    replay_bytes: &mut usize,
    persisted_covers_through_sequence: u64,
    config: &GenerationSessionConfig,
) -> Result<(), GenerationSessionError> {
    while replay.len() > config.replay_max_events || *replay_bytes > config.replay_max_bytes {
        let Some(front) = replay.front() else {
            break;
        };
        if front.sequence > persisted_covers_through_sequence {
            return Err(GenerationSessionError::SessionCapacityExceeded);
        }
        let removed = replay.pop_front().expect("front event exists");
        *replay_bytes -= encoded_event_bytes(&removed)?;
    }
    Ok(())
}

pub(crate) fn now_ms() -> Result<i64, GenerationSessionError> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| GenerationSessionError::InvalidTimestamp)?
        .as_millis();
    let milliseconds =
        i64::try_from(milliseconds).map_err(|_| GenerationSessionError::InvalidTimestamp)?;
    if milliseconds > JS_SAFE_INTEGER_I64 {
        return Err(GenerationSessionError::InvalidTimestamp);
    }
    Ok(milliseconds)
}

pub(super) fn duration_ms_i64(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).expect("validated generation duration fits i64")
}

pub(super) fn validate_checkpoint_revision(revision: i64) -> Result<(), GenerationSessionError> {
    if (1..=JS_SAFE_INTEGER_I64).contains(&revision) {
        Ok(())
    } else {
        Err(GenerationSessionError::InvalidSession(
            "checkpoint_revision 超出 JavaScript safe positive integer 范围".to_string(),
        ))
    }
}

pub(super) fn validate_persisted_at_ms(persisted_at_ms: i64) -> Result<(), GenerationSessionError> {
    if (0..=JS_SAFE_INTEGER_I64).contains(&persisted_at_ms) {
        Ok(())
    } else {
        Err(GenerationSessionError::InvalidTimestamp)
    }
}

pub(super) fn validate_durable_commit(
    commit: GenerationDurableCommit,
) -> Result<(), GenerationSessionError> {
    validate_checkpoint_revision(commit.checkpoint_revision)?;
    validate_persisted_at_ms(commit.persisted_at_ms)?;
    if commit.covers_through_sequence == 0
        || commit.covers_through_sequence > commit.reserved_through_sequence
        || commit.reserved_through_sequence > JS_SAFE_INTEGER_U64
    {
        return Err(GenerationSessionError::InvalidSequenceLease {
            current: commit.covers_through_sequence,
            next: commit.reserved_through_sequence,
        });
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub enum GenerationReplay {
    Events {
        events: Vec<GenerationTaskEvent>,
        latest_sequence: u64,
        replay_floor: u64,
    },
    SnapshotRequired {
        snapshot: GenerationTaskSnapshot,
        latest_sequence: u64,
        replay_floor: u64,
    },
}

pub struct GenerationSubscription {
    pub(super) initial: Option<GenerationReplay>,
    pub(super) receiver: watch::Receiver<u64>,
    pub(super) session: Arc<GenerationTaskSession>,
    pub(super) host_permit: Option<HostSubscriptionPermit>,
}

impl GenerationSubscription {
    pub fn take_initial(&mut self) -> Option<GenerationReplay> {
        self.initial.take()
    }

    pub fn receiver(&mut self) -> &mut watch::Receiver<u64> {
        &mut self.receiver
    }

    /// 从同一个 session replay 中读取 cursor 之后的事件；不创建第二份 transport 缓冲。
    pub fn replay_after(&self, after_sequence: Option<u64>) -> GenerationReplay {
        self.session.replay_after(after_sequence)
    }

    pub fn snapshot(&self) -> GenerationTaskSnapshot {
        self.session.snapshot()
    }
}

impl Drop for GenerationSubscription {
    fn drop(&mut self) {
        let mut state = self
            .session
            .state
            .lock()
            .expect("generation session mutex poisoned");
        assert!(
            state.subscriptions > 0,
            "generation subscription count underflow"
        );
        state.subscriptions -= 1;
        drop(state);
        self.host_permit.take();
    }
}

pub(super) fn replay_after_locked(
    state: &SessionState,
    after_sequence: Option<u64>,
) -> GenerationReplay {
    let cursor = after_sequence.unwrap_or(0);
    let latest_sequence = state.snapshot.covers_through_sequence;
    let replay_floor = state
        .replay
        .front()
        .map(|event| event.sequence)
        .unwrap_or_else(|| latest_sequence.saturating_add(1));
    if state.snapshot_only_covers.is_some() {
        return GenerationReplay::SnapshotRequired {
            snapshot: state.snapshot.clone(),
            latest_sequence,
            replay_floor,
        };
    }
    if cursor > latest_sequence || cursor.saturating_add(1) < replay_floor {
        return GenerationReplay::SnapshotRequired {
            snapshot: state.snapshot.clone(),
            latest_sequence,
            replay_floor,
        };
    }
    GenerationReplay::Events {
        events: state
            .replay
            .iter()
            .filter(|event| event.sequence > cursor)
            .cloned()
            .collect(),
        latest_sequence,
        replay_floor,
    }
}
