use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use a4n_service_protocol::{
    ArtifactEvent, ArtifactOperation, CancelRequestedEvent, DEFAULT_GENERATION_ACTIVE_SESSION_MAX,
    GENERATION_EVENT_SCHEMA_VERSION, GenerationEvent, GenerationTaskEvent, GenerationTaskSnapshot,
    GenerationTaskStatus, GenerationUsage, JS_SAFE_INTEGER_I64, JS_SAFE_INTEGER_U64,
    MAX_GENERATION_EVENT_BYTES, ProgressEvent, TextDeltaEvent, TextStreamSnapshot, UsageEvent,
};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex as AsyncMutex, Notify, watch};

const SEQUENCE_LEASE_BLOCK: u64 = 256;
const SEQUENCE_EMERGENCY_SLOTS: u64 = 16;
const DEFAULT_TERMINAL_MEMORY_TTL: Duration = Duration::from_secs(30 * 60);
const DEFAULT_CHECKPOINT_INTERVAL: Duration = Duration::from_secs(2);
const DEFAULT_CHECKPOINT_BYTE_THRESHOLD: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct GenerationSessionConfig {
    pub replay_max_events: usize,
    pub replay_max_bytes: usize,
    pub active_session_max: usize,
    pub session_admission_base_bytes: usize,
    pub global_session_memory_bytes: usize,
    pub subscriptions_per_task: usize,
    pub subscriptions_per_host: usize,
    pub terminal_memory_ttl: Duration,
    pub checkpoint_interval: Duration,
    pub checkpoint_byte_threshold: usize,
}

impl Default for GenerationSessionConfig {
    fn default() -> Self {
        Self {
            replay_max_events: 2_048,
            replay_max_bytes: 4 * 1024 * 1024,
            active_session_max: DEFAULT_GENERATION_ACTIVE_SESSION_MAX,
            session_admission_base_bytes: 64 * 1024,
            global_session_memory_bytes: 512 * 1024 * 1024,
            subscriptions_per_task: 128,
            subscriptions_per_host: 1_024,
            terminal_memory_ttl: DEFAULT_TERMINAL_MEMORY_TTL,
            checkpoint_interval: DEFAULT_CHECKPOINT_INTERVAL,
            checkpoint_byte_threshold: DEFAULT_CHECKPOINT_BYTE_THRESHOLD,
        }
    }
}

impl GenerationSessionConfig {
    pub fn validate(&self) -> Result<(), GenerationSessionError> {
        if !(256..=8_192).contains(&self.replay_max_events) {
            return Err(GenerationSessionError::InvalidConfiguration(
                "replay_max_events 必须在 256..=8192".to_string(),
            ));
        }
        if !(1024 * 1024..=16 * 1024 * 1024).contains(&self.replay_max_bytes) {
            return Err(GenerationSessionError::InvalidConfiguration(
                "replay_max_bytes 必须在 1 MiB..=16 MiB".to_string(),
            ));
        }
        if !(16..=256).contains(&self.active_session_max) {
            return Err(GenerationSessionError::InvalidConfiguration(
                "active_session_max 必须在 16..=256".to_string(),
            ));
        }
        if self.session_admission_base_bytes < 64 * 1024 {
            return Err(GenerationSessionError::InvalidConfiguration(
                "session_admission_base_bytes 不能低于 64 KiB".to_string(),
            ));
        }
        if !(128 * 1024 * 1024..=2 * 1024 * 1024 * 1024).contains(&self.global_session_memory_bytes)
        {
            return Err(GenerationSessionError::InvalidConfiguration(
                "global_session_memory_bytes 必须在 128 MiB..=2 GiB".to_string(),
            ));
        }
        if !(8..=256).contains(&self.subscriptions_per_task) {
            return Err(GenerationSessionError::InvalidConfiguration(
                "subscriptions_per_task 必须在 8..=256".to_string(),
            ));
        }
        if !(64..=4_096).contains(&self.subscriptions_per_host) {
            return Err(GenerationSessionError::InvalidConfiguration(
                "subscriptions_per_host 必须在 64..=4096".to_string(),
            ));
        }
        if !(Duration::from_secs(5 * 60)..=Duration::from_secs(2 * 60 * 60))
            .contains(&self.terminal_memory_ttl)
        {
            return Err(GenerationSessionError::InvalidConfiguration(
                "terminal_memory_ttl 必须在 5m..=2h".to_string(),
            ));
        }
        if !(Duration::from_secs(1)..=Duration::from_secs(10)).contains(&self.checkpoint_interval) {
            return Err(GenerationSessionError::InvalidConfiguration(
                "checkpoint_interval 必须在 1s..=10s".to_string(),
            ));
        }
        if !(16 * 1024..=512 * 1024).contains(&self.checkpoint_byte_threshold) {
            return Err(GenerationSessionError::InvalidConfiguration(
                "checkpoint_byte_threshold 必须在 16 KiB..=512 KiB".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationCapacitySnapshot {
    pub active_sessions: usize,
    pub session_memory_bytes: usize,
    pub subscriptions: usize,
}

#[derive(Clone)]
pub struct GenerationAdmissionController {
    inner: Arc<AdmissionInner>,
}

struct AdmissionInner {
    config: GenerationSessionConfig,
    state: Mutex<AdmissionState>,
}

#[derive(Default)]
struct AdmissionState {
    active_sessions: usize,
    session_memory_bytes: usize,
    subscriptions: usize,
}

pub struct SessionAdmissionPermit {
    lease: Option<SessionCapacityLease>,
}

struct SessionCapacityLease {
    inner: Arc<SessionCapacityInner>,
}

struct SessionCapacityInner {
    controller: Arc<AdmissionInner>,
    state: Mutex<SessionCapacityState>,
}

struct SessionCapacityState {
    bytes: usize,
    active: bool,
}

struct MemoryReservation {
    lease: Arc<SessionCapacityInner>,
    old_bytes: usize,
    committed: bool,
}

struct HostSubscriptionPermit {
    controller: Arc<AdmissionInner>,
}

impl GenerationAdmissionController {
    pub fn new(config: GenerationSessionConfig) -> Result<Self, GenerationSessionError> {
        config.validate()?;
        Ok(Self {
            inner: Arc::new(AdmissionInner {
                config,
                state: Mutex::new(AdmissionState::default()),
            }),
        })
    }

    pub fn try_admit(&self) -> Result<SessionAdmissionPermit, GenerationSessionError> {
        let base_bytes = self.inner.config.session_admission_base_bytes;
        let mut state = self
            .inner
            .state
            .lock()
            .expect("generation admission mutex poisoned");
        let next_memory = state
            .session_memory_bytes
            .checked_add(base_bytes)
            .ok_or(GenerationSessionError::SessionCapacityExceeded)?;
        if state.active_sessions >= self.inner.config.active_session_max
            || next_memory > self.inner.config.global_session_memory_bytes
        {
            return Err(GenerationSessionError::SessionCapacityExceeded);
        }
        state.active_sessions += 1;
        state.session_memory_bytes = next_memory;
        drop(state);

        Ok(SessionAdmissionPermit {
            lease: Some(SessionCapacityLease {
                inner: Arc::new(SessionCapacityInner {
                    controller: Arc::clone(&self.inner),
                    state: Mutex::new(SessionCapacityState {
                        bytes: base_bytes,
                        active: true,
                    }),
                }),
            }),
        })
    }

    pub fn snapshot(&self) -> GenerationCapacitySnapshot {
        let state = self
            .inner
            .state
            .lock()
            .expect("generation admission mutex poisoned");
        GenerationCapacitySnapshot {
            active_sessions: state.active_sessions,
            session_memory_bytes: state.session_memory_bytes,
            subscriptions: state.subscriptions,
        }
    }
}

pub struct GenerationSessionRegistry {
    admission: GenerationAdmissionController,
    sessions: Mutex<BTreeMap<String, Arc<GenerationTaskSession>>>,
}

impl GenerationSessionRegistry {
    pub fn new(config: GenerationSessionConfig) -> Result<Self, GenerationSessionError> {
        Ok(Self {
            admission: GenerationAdmissionController::new(config)?,
            sessions: Mutex::new(BTreeMap::new()),
        })
    }

    pub fn try_admit(&self) -> Result<SessionAdmissionPermit, GenerationSessionError> {
        self.cleanup_terminal_sessions_at(now_ms()?);
        loop {
            match self.admission.try_admit() {
                Ok(permit) => return Ok(permit),
                Err(GenerationSessionError::SessionCapacityExceeded)
                    if self.admission.snapshot().active_sessions
                        < self.admission.inner.config.active_session_max
                        && self.evict_oldest_terminal() => {}
                Err(error) => return Err(error),
            }
        }
    }

    pub fn insert(
        &self,
        session: Arc<GenerationTaskSession>,
    ) -> Result<(), GenerationSessionError> {
        let (task_id, attempt) = session.identity();
        let mut sessions = self
            .sessions
            .lock()
            .expect("generation session registry mutex poisoned");
        if let Some(existing) = sessions.get(&task_id) {
            let (_, existing_attempt) = existing.identity();
            if existing_attempt != attempt {
                return Err(GenerationSessionError::InvalidSession(format!(
                    "task {task_id} 已登记 attempt {existing_attempt}，不能插入 attempt {attempt}"
                )));
            }
            if Arc::ptr_eq(existing, &session) {
                return Ok(());
            }
            return Err(GenerationSessionError::InvalidSession(format!(
                "task {task_id} attempt {attempt} 已有 session"
            )));
        }
        sessions.insert(task_id, session);
        Ok(())
    }

    pub(crate) fn insert_restored_or_get(
        &self,
        session: Arc<GenerationTaskSession>,
    ) -> Result<Arc<GenerationTaskSession>, GenerationSessionError> {
        let (task_id, attempt) = session.identity();
        let mut sessions = self
            .sessions
            .lock()
            .expect("generation session registry mutex poisoned");
        if let Some(existing) = sessions.get(&task_id) {
            let (_, existing_attempt) = existing.identity();
            if existing_attempt != attempt {
                return Err(GenerationSessionError::InvalidSession(format!(
                    "task {task_id} 已登记 attempt {existing_attempt}，不能恢复 attempt {attempt}"
                )));
            }
            return Ok(Arc::clone(existing));
        }
        sessions.insert(task_id, Arc::clone(&session));
        Ok(session)
    }

    pub fn get(
        &self,
        task_id: &str,
        attempt: u32,
    ) -> Result<Option<Arc<GenerationTaskSession>>, GenerationSessionError> {
        let sessions = self
            .sessions
            .lock()
            .expect("generation session registry mutex poisoned");
        let Some(session) = sessions.get(task_id) else {
            return Ok(None);
        };
        let (_, actual_attempt) = session.identity();
        if actual_attempt != attempt {
            return Err(GenerationSessionError::InvalidSession(format!(
                "task {task_id} 当前 attempt={actual_attempt}，请求 attempt={attempt}"
            )));
        }
        Ok(Some(Arc::clone(session)))
    }

    pub fn remove(
        &self,
        task_id: &str,
        attempt: u32,
    ) -> Result<Option<Arc<GenerationTaskSession>>, GenerationSessionError> {
        let mut sessions = self
            .sessions
            .lock()
            .expect("generation session registry mutex poisoned");
        let Some(session) = sessions.get(task_id) else {
            return Ok(None);
        };
        let (_, actual_attempt) = session.identity();
        if actual_attempt != attempt {
            return Err(GenerationSessionError::InvalidSession(format!(
                "task {task_id} 当前 attempt={actual_attempt}，不能移除 attempt={attempt}"
            )));
        }
        Ok(sessions.remove(task_id))
    }

    pub fn len(&self) -> usize {
        self.sessions
            .lock()
            .expect("generation session registry mutex poisoned")
            .len()
    }

    pub fn cleanup_terminal_sessions(&self) -> Result<usize, GenerationSessionError> {
        Ok(self.cleanup_terminal_sessions_at(now_ms()?))
    }

    fn cleanup_terminal_sessions_at(&self, now_ms: i64) -> usize {
        let ttl_ms = duration_ms_i64(self.admission.inner.config.terminal_memory_ttl);
        let mut sessions = self
            .sessions
            .lock()
            .expect("generation session registry mutex poisoned");
        let before = sessions.len();
        sessions.retain(|_, session| {
            session
                .terminal_persisted_at()
                .is_none_or(|updated_at_ms| now_ms.saturating_sub(updated_at_ms) < ttl_ms)
        });
        before - sessions.len()
    }

    fn evict_oldest_terminal(&self) -> bool {
        let mut sessions = self
            .sessions
            .lock()
            .expect("generation session registry mutex poisoned");
        let oldest = sessions
            .iter()
            .filter_map(|(task_id, session)| {
                session
                    .terminal_persisted_at()
                    .map(|updated_at_ms| (updated_at_ms, task_id.clone()))
            })
            .min();
        oldest
            .and_then(|(_, task_id)| sessions.remove(&task_id))
            .is_some()
    }
}

impl SessionAdmissionPermit {
    fn activate(mut self) -> SessionCapacityLease {
        self.lease
            .take()
            .expect("session admission permit already consumed")
    }
}

impl SessionCapacityLease {
    fn config(&self) -> &GenerationSessionConfig {
        &self.inner.controller.config
    }

    fn current_memory(&self) -> usize {
        self.inner
            .state
            .lock()
            .expect("session capacity mutex poisoned")
            .bytes
    }

    fn reserve_memory(
        &self,
        reserved_bytes: usize,
    ) -> Result<MemoryReservation, GenerationSessionError> {
        let mut lease = self
            .inner
            .state
            .lock()
            .expect("session capacity mutex poisoned");
        let old_bytes = lease.bytes;
        let reserved_bytes = old_bytes.max(reserved_bytes);
        if reserved_bytes > old_bytes {
            let mut admission = self
                .inner
                .controller
                .state
                .lock()
                .expect("generation admission mutex poisoned");
            let next_memory = admission
                .session_memory_bytes
                .checked_add(reserved_bytes - old_bytes)
                .ok_or(GenerationSessionError::SessionCapacityExceeded)?;
            if next_memory > self.inner.controller.config.global_session_memory_bytes {
                return Err(GenerationSessionError::SessionCapacityExceeded);
            }
            admission.session_memory_bytes = next_memory;
            lease.bytes = reserved_bytes;
        }
        drop(lease);

        Ok(MemoryReservation {
            lease: Arc::clone(&self.inner),
            old_bytes,
            committed: false,
        })
    }

    fn set_memory(&self, target_bytes: usize) -> Result<(), GenerationSessionError> {
        self.reserve_memory(target_bytes)?.commit(target_bytes);
        Ok(())
    }

    fn release_active_slot(&self) {
        let mut lease = self
            .inner
            .state
            .lock()
            .expect("session capacity mutex poisoned");
        if !lease.active {
            return;
        }
        let mut admission = self
            .inner
            .controller
            .state
            .lock()
            .expect("generation admission mutex poisoned");
        admission.active_sessions -= 1;
        lease.active = false;
    }

    fn try_acquire_subscription(&self) -> Result<HostSubscriptionPermit, GenerationSessionError> {
        let mut admission = self
            .inner
            .controller
            .state
            .lock()
            .expect("generation admission mutex poisoned");
        if admission.subscriptions >= self.inner.controller.config.subscriptions_per_host {
            return Err(GenerationSessionError::SubscriptionCapacityExceeded);
        }
        admission.subscriptions += 1;
        Ok(HostSubscriptionPermit {
            controller: Arc::clone(&self.inner.controller),
        })
    }
}

impl Drop for SessionCapacityInner {
    fn drop(&mut self) {
        let lease = self.state.lock().expect("session capacity mutex poisoned");
        let mut admission = self
            .controller
            .state
            .lock()
            .expect("generation admission mutex poisoned");
        admission.session_memory_bytes -= lease.bytes;
        if lease.active {
            admission.active_sessions -= 1;
        }
    }
}

impl MemoryReservation {
    fn commit(mut self, target_bytes: usize) {
        resize_reserved_memory(&self.lease, target_bytes);
        self.committed = true;
    }
}

impl Drop for MemoryReservation {
    fn drop(&mut self) {
        if !self.committed {
            resize_reserved_memory(&self.lease, self.old_bytes);
        }
    }
}

fn resize_reserved_memory(lease: &SessionCapacityInner, target_bytes: usize) {
    let mut state = lease.state.lock().expect("session capacity mutex poisoned");
    assert!(
        target_bytes <= state.bytes,
        "memory target must be reserved before commit"
    );
    let mut admission = lease
        .controller
        .state
        .lock()
        .expect("generation admission mutex poisoned");
    if target_bytes < state.bytes {
        admission.session_memory_bytes -= state.bytes - target_bytes;
    }
    state.bytes = target_bytes;
}

impl Drop for HostSubscriptionPermit {
    fn drop(&mut self) {
        let mut state = self
            .controller
            .state
            .lock()
            .expect("generation admission mutex poisoned");
        state.subscriptions -= 1;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GenerationSessionError {
    #[error("生成会话配置无效: {0}")]
    InvalidConfiguration(String),
    #[error("生成事件合同无效: {0}")]
    Contract(String),
    #[error("生成会话身份或 replay 无效: {0}")]
    InvalidSession(String),
    #[error("生成会话状态 {status:?} 不允许 {operation}")]
    InvalidState {
        status: GenerationTaskStatus,
        operation: &'static str,
    },
    #[error("生成会话已进入终态")]
    TerminalAlreadySet,
    #[error("文本流 {stream_id} offset 不匹配: expected={expected}, actual={actual}")]
    OffsetMismatch {
        stream_id: String,
        expected: u64,
        actual: u64,
    },
    #[error("usage 累计值发生回退")]
    UsageRegression,
    #[error("sequence lease 扩租失败: {0}")]
    SequenceLeaseUnavailable(String),
    #[error("sequence lease 高水位无效: current={current}, next={next}")]
    InvalidSequenceLease { current: u64, next: u64 },
    #[error("生成会话容量已满")]
    SessionCapacityExceeded,
    #[error("生成订阅容量已满")]
    SubscriptionCapacityExceeded,
    #[error("普通 publisher 已封闭: {0}")]
    PublisherSealed(String),
    #[error("时间戳超出生成合同范围")]
    InvalidTimestamp,
}

impl GenerationSessionError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::OffsetMismatch { .. } => "generation_offset_mismatch",
            Self::SequenceLeaseUnavailable(_) | Self::InvalidSequenceLease { .. } => {
                "generation_checkpoint_unavailable"
            }
            Self::SessionCapacityExceeded => "generation_session_capacity_exceeded",
            Self::SubscriptionCapacityExceeded => "generation_subscription_capacity_exceeded",
            Self::TerminalAlreadySet | Self::InvalidState { .. } => {
                "generation_terminal_already_set"
            }
            Self::PublisherSealed(_) => "generation_checkpoint_unavailable",
            Self::InvalidConfiguration(_)
            | Self::Contract(_)
            | Self::InvalidSession(_)
            | Self::UsageRegression
            | Self::InvalidTimestamp => "generation_event_invalid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationDurableCommit {
    pub checkpoint_revision: i64,
    pub covers_through_sequence: u64,
    pub reserved_through_sequence: u64,
    pub persisted_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationSequenceLeaseUpdate {
    pub checkpoint_revision: i64,
    pub reserved_through_sequence: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct SessionCheckpointCandidate {
    pub(crate) snapshot: GenerationTaskSnapshot,
    pub(crate) checkpoint_revision: i64,
    pub(crate) unpersisted_event_bytes: usize,
    pub(crate) last_persisted_at_ms: i64,
}

struct SessionState {
    snapshot: GenerationTaskSnapshot,
    replay: VecDeque<GenerationTaskEvent>,
    replay_bytes: usize,
    checkpoint_revision: i64,
    persisted_covers_through_sequence: u64,
    reserved_through_sequence: u64,
    last_persisted_at_ms: i64,
    unpersisted_event_bytes: usize,
    next_chunk_indexes: BTreeMap<String, u64>,
    durable_commit_in_flight: bool,
    ordinary_publisher_sealed: Option<String>,
    snapshot_only_covers: Option<u64>,
    subscriptions: usize,
}

pub struct GenerationTaskSession {
    config: GenerationSessionConfig,
    state: Mutex<SessionState>,
    latest_sequence: watch::Sender<u64>,
    durable_commit_notify: Notify,
    checkpoint_write_lock: AsyncMutex<()>,
    capacity: SessionCapacityLease,
}

impl GenerationTaskSession {
    pub fn from_started_event(
        started_event: GenerationTaskEvent,
        reserved_through_sequence: u64,
        admission: SessionAdmissionPermit,
    ) -> Result<Arc<Self>, GenerationSessionError> {
        started_event
            .validate()
            .map_err(|error| GenerationSessionError::Contract(error.to_string()))?;
        let GenerationEvent::Started(started) = &started_event.event else {
            return Err(GenerationSessionError::InvalidSession(
                "初始事件必须是 started".to_string(),
            ));
        };
        let persisted_at_ms = started_event.emitted_at_ms;
        let snapshot = GenerationTaskSnapshot {
            schema_version: GENERATION_EVENT_SCHEMA_VERSION,
            task_id: started_event.task_id.clone(),
            attempt: started_event.attempt,
            covers_through_sequence: started_event.sequence,
            status: GenerationTaskStatus::Reserved,
            started: started.clone(),
            cancel_reason: None,
            progress: None,
            streams: Vec::new(),
            artifacts: Vec::new(),
            usage: None,
            latest_checkpoint: None,
            terminal_event: None,
            updated_at_ms: started_event.emitted_at_ms,
        };
        Self::from_snapshot(
            snapshot,
            vec![started_event],
            1,
            reserved_through_sequence,
            persisted_at_ms,
            admission,
        )
    }

    pub fn from_snapshot(
        snapshot: GenerationTaskSnapshot,
        replay: Vec<GenerationTaskEvent>,
        checkpoint_revision: i64,
        reserved_through_sequence: u64,
        persisted_at_ms: i64,
        admission: SessionAdmissionPermit,
    ) -> Result<Arc<Self>, GenerationSessionError> {
        snapshot
            .validate()
            .map_err(|error| GenerationSessionError::Contract(error.to_string()))?;
        if reserved_through_sequence < snapshot.covers_through_sequence
            || reserved_through_sequence > JS_SAFE_INTEGER_U64
        {
            return Err(GenerationSessionError::InvalidSequenceLease {
                current: snapshot.covers_through_sequence,
                next: reserved_through_sequence,
            });
        }
        validate_checkpoint_revision(checkpoint_revision)?;
        validate_persisted_at_ms(persisted_at_ms)?;

        let capacity = admission.activate();
        let config = capacity.config().clone();
        let mut replay = VecDeque::from(replay);
        validate_replay(&snapshot, &replay)?;
        let mut replay_bytes = replay
            .iter()
            .map(encoded_event_bytes)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .sum();
        trim_replay(
            &mut replay,
            &mut replay_bytes,
            snapshot.covers_through_sequence,
            &config,
        )?;
        let next_chunk_indexes = replay_chunk_indexes(&replay);
        let memory_bytes = session_memory_bytes(&snapshot, replay_bytes, &config)?;
        capacity.set_memory(memory_bytes)?;
        if snapshot.status.is_terminal() {
            capacity.release_active_slot();
        }
        let (latest_sequence, _) = watch::channel(snapshot.covers_through_sequence);

        Ok(Arc::new(Self {
            config,
            state: Mutex::new(SessionState {
                checkpoint_revision,
                persisted_covers_through_sequence: snapshot.covers_through_sequence,
                reserved_through_sequence,
                last_persisted_at_ms: persisted_at_ms,
                unpersisted_event_bytes: 0,
                snapshot,
                replay,
                replay_bytes,
                next_chunk_indexes,
                durable_commit_in_flight: false,
                ordinary_publisher_sealed: None,
                snapshot_only_covers: None,
                subscriptions: 0,
            }),
            latest_sequence,
            durable_commit_notify: Notify::new(),
            checkpoint_write_lock: AsyncMutex::new(()),
            capacity,
        }))
    }

    pub fn from_snapshot_only(
        snapshot: GenerationTaskSnapshot,
        checkpoint_revision: i64,
        reserved_through_sequence: u64,
        persisted_at_ms: i64,
        admission: SessionAdmissionPermit,
    ) -> Result<Arc<Self>, GenerationSessionError> {
        let covers_through_sequence = snapshot.covers_through_sequence;
        let session = Self::from_snapshot(
            snapshot,
            Vec::new(),
            checkpoint_revision,
            reserved_through_sequence,
            persisted_at_ms,
            admission,
        )?;
        session
            .state
            .lock()
            .expect("generation session mutex poisoned")
            .snapshot_only_covers = Some(covers_through_sequence);
        Ok(session)
    }

    pub fn snapshot(&self) -> GenerationTaskSnapshot {
        self.state
            .lock()
            .expect("generation session mutex poisoned")
            .snapshot
            .clone()
    }

    pub fn reserved_through_sequence(&self) -> u64 {
        self.state
            .lock()
            .expect("generation session mutex poisoned")
            .reserved_through_sequence
    }

    fn identity(&self) -> (String, u32) {
        let state = self
            .state
            .lock()
            .expect("generation session mutex poisoned");
        (state.snapshot.task_id.clone(), state.snapshot.attempt)
    }

    fn terminal_persisted_at(&self) -> Option<i64> {
        let state = self
            .state
            .lock()
            .expect("generation session mutex poisoned");
        (state.snapshot.status.is_terminal()
            && state.persisted_covers_through_sequence >= state.snapshot.covers_through_sequence)
            .then_some(state.snapshot.updated_at_ms)
    }
}

fn validate_replay(
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

fn replay_chunk_indexes(replay: &VecDeque<GenerationTaskEvent>) -> BTreeMap<String, u64> {
    let mut indexes = BTreeMap::new();
    for event in replay {
        if let GenerationEvent::TextDelta(delta) = &event.event {
            indexes.insert(delta.stream_id.clone(), delta.chunk_index.saturating_add(1));
        }
    }
    indexes
}

fn encoded_event_bytes(event: &GenerationTaskEvent) -> Result<usize, GenerationSessionError> {
    serde_json::to_vec(event)
        .map(|bytes| bytes.len())
        .map_err(|error| GenerationSessionError::Contract(error.to_string()))
}

fn session_memory_bytes(
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

fn trim_replay(
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

fn duration_ms_i64(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).expect("validated generation duration fits i64")
}

fn validate_checkpoint_revision(revision: i64) -> Result<(), GenerationSessionError> {
    if (1..=JS_SAFE_INTEGER_I64).contains(&revision) {
        Ok(())
    } else {
        Err(GenerationSessionError::InvalidSession(
            "checkpoint_revision 超出 JavaScript safe positive integer 范围".to_string(),
        ))
    }
}

fn validate_persisted_at_ms(persisted_at_ms: i64) -> Result<(), GenerationSessionError> {
    if (0..=JS_SAFE_INTEGER_I64).contains(&persisted_at_ms) {
        Ok(())
    } else {
        Err(GenerationSessionError::InvalidTimestamp)
    }
}

fn validate_durable_commit(commit: GenerationDurableCommit) -> Result<(), GenerationSessionError> {
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
    initial: Option<GenerationReplay>,
    receiver: watch::Receiver<u64>,
    session: Arc<GenerationTaskSession>,
    host_permit: Option<HostSubscriptionPermit>,
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

#[async_trait]
pub trait GenerationSequenceLeaseReplenisher: Send + Sync {
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
enum OrdinaryInput {
    Progress(ProgressEvent),
    Text { stream_id: String, delta: String },
    Usage(UsageEvent),
}

#[derive(Debug, Clone, Copy)]
enum ApplyMode {
    Ordinary,
    Committed,
}

impl GenerationTaskSession {
    pub fn is_terminal(&self) -> bool {
        self.state
            .lock()
            .expect("generation session mutex poisoned")
            .snapshot
            .status
            .is_terminal()
    }

    pub(crate) async fn checkpoint_write_guard(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.checkpoint_write_lock.lock().await
    }

    pub(crate) fn checkpoint_candidate(&self) -> SessionCheckpointCandidate {
        let state = self
            .state
            .lock()
            .expect("generation session mutex poisoned");
        SessionCheckpointCandidate {
            snapshot: state.snapshot.clone(),
            checkpoint_revision: state.checkpoint_revision,
            unpersisted_event_bytes: state.unpersisted_event_bytes,
            last_persisted_at_ms: state.last_persisted_at_ms,
        }
    }

    pub(crate) fn checkpoint_due(
        &self,
        candidate: &SessionCheckpointCandidate,
        now_ms: i64,
        force: bool,
    ) -> bool {
        force
            || (candidate.unpersisted_event_bytes > 0
                && (candidate.unpersisted_event_bytes >= self.config.checkpoint_byte_threshold
                    || now_ms.saturating_sub(candidate.last_persisted_at_ms)
                        >= duration_ms_i64(self.config.checkpoint_interval)))
    }

    pub(crate) fn mark_snapshot_persisted(
        &self,
        commit: GenerationDurableCommit,
    ) -> Result<(), GenerationSessionError> {
        validate_durable_commit(commit)?;
        let mut state = self
            .state
            .lock()
            .expect("generation session mutex poisoned");
        if commit.checkpoint_revision <= state.checkpoint_revision
            || commit.covers_through_sequence < state.persisted_covers_through_sequence
            || commit.covers_through_sequence > state.snapshot.covers_through_sequence
        {
            return Err(GenerationSessionError::InvalidSession(
                "checkpoint commit 与当前 session cursor 不一致".to_string(),
            ));
        }

        let mut replay = state.replay.clone();
        let mut replay_bytes = state.replay_bytes;
        trim_replay(
            &mut replay,
            &mut replay_bytes,
            commit.covers_through_sequence,
            &self.config,
        )?;
        let unpersisted_event_bytes = replay
            .iter()
            .filter(|event| event.sequence > commit.covers_through_sequence)
            .map(encoded_event_bytes)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .sum();
        let memory_bytes = session_memory_bytes(&state.snapshot, replay_bytes, &self.config)?;
        let memory = self.capacity.reserve_memory(memory_bytes)?;

        state.replay = replay;
        state.replay_bytes = replay_bytes;
        state.checkpoint_revision = commit.checkpoint_revision;
        state.persisted_covers_through_sequence = commit.covers_through_sequence;
        state.reserved_through_sequence = state
            .reserved_through_sequence
            .max(commit.reserved_through_sequence);
        state.last_persisted_at_ms = commit.persisted_at_ms;
        state.unpersisted_event_bytes = unpersisted_event_bytes;
        if state.snapshot_only_covers.is_some() {
            state.snapshot_only_covers = Some(commit.covers_through_sequence);
        }
        memory.commit(memory_bytes);
        Ok(())
    }

    pub(crate) fn rebuild_from_persisted_snapshot(
        &self,
        snapshot: GenerationTaskSnapshot,
        commit: GenerationDurableCommit,
    ) -> Result<(), GenerationSessionError> {
        snapshot
            .validate()
            .map_err(|error| GenerationSessionError::Contract(error.to_string()))?;
        validate_durable_commit(commit)?;
        if snapshot.covers_through_sequence != commit.covers_through_sequence {
            return Err(GenerationSessionError::InvalidSession(
                "persisted snapshot covers 与 commit 不一致".to_string(),
            ));
        }
        let memory_bytes = session_memory_bytes(&snapshot, 0, &self.config)?;
        let memory = self.capacity.reserve_memory(memory_bytes)?;
        let mut state = self
            .state
            .lock()
            .expect("generation session mutex poisoned");
        if snapshot.task_id != state.snapshot.task_id || snapshot.attempt != state.snapshot.attempt
        {
            return Err(GenerationSessionError::InvalidSession(
                "persisted snapshot task_id/attempt 与 session 不一致".to_string(),
            ));
        }
        if state.durable_commit_in_flight {
            return Err(GenerationSessionError::InvalidSession(
                "durable permit 释放前不能重建 session".to_string(),
            ));
        }
        let terminal = snapshot.status.is_terminal();
        state.snapshot = snapshot;
        state.replay.clear();
        state.replay_bytes = 0;
        state.checkpoint_revision = commit.checkpoint_revision;
        state.persisted_covers_through_sequence = commit.covers_through_sequence;
        state.reserved_through_sequence = commit.reserved_through_sequence;
        state.last_persisted_at_ms = commit.persisted_at_ms;
        state.unpersisted_event_bytes = 0;
        state.snapshot_only_covers = Some(commit.covers_through_sequence);
        let latest_sequence = state.snapshot.covers_through_sequence;
        if terminal {
            self.capacity.release_active_slot();
        }
        memory.commit(memory_bytes);
        self.latest_sequence.send_replace(latest_sequence);
        Ok(())
    }

    pub fn replay_after(&self, after_sequence: Option<u64>) -> GenerationReplay {
        let state = self
            .state
            .lock()
            .expect("generation session mutex poisoned");
        replay_after_locked(&state, after_sequence)
    }

    pub fn subscribe(
        self: &Arc<Self>,
        after_sequence: Option<u64>,
    ) -> Result<GenerationSubscription, GenerationSessionError> {
        let host_permit = self.capacity.try_acquire_subscription()?;
        let mut state = self
            .state
            .lock()
            .expect("generation session mutex poisoned");
        if state.subscriptions >= self.config.subscriptions_per_task {
            drop(state);
            drop(host_permit);
            return Err(GenerationSessionError::SubscriptionCapacityExceeded);
        }
        state.subscriptions += 1;
        if state.snapshot_only_covers.is_some_and(|covers| {
            !state.snapshot.status.is_terminal() && after_sequence == Some(covers)
        }) {
            state.snapshot_only_covers = None;
        }
        let initial = replay_after_locked(&state, after_sequence);
        let receiver = self.latest_sequence.subscribe();
        drop(state);
        Ok(GenerationSubscription {
            initial: Some(initial),
            receiver,
            session: Arc::clone(self),
            host_permit: Some(host_permit),
        })
    }

    pub fn prepare_committed_events(
        self: &Arc<Self>,
        events: Vec<GenerationEvent>,
    ) -> Result<CommittedEventPermit, GenerationSessionError> {
        if events.is_empty() {
            return Err(GenerationSessionError::InvalidSession(
                "committed event batch 不能为空".to_string(),
            ));
        }

        let mut state = self
            .state
            .lock()
            .expect("generation session mutex poisoned");
        if state.durable_commit_in_flight {
            return Err(GenerationSessionError::InvalidSession(
                "committed event 必须由 checkpoint write guard 串行准备".to_string(),
            ));
        }
        let first_sequence = state
            .snapshot
            .covers_through_sequence
            .checked_add(1)
            .ok_or(GenerationSessionError::InvalidSequenceLease {
                current: state.snapshot.covers_through_sequence,
                next: u64::MAX,
            })?;
        let event_copy_budget = events
            .len()
            .checked_mul(MAX_GENERATION_EVENT_BYTES * 2)
            .ok_or(GenerationSessionError::SessionCapacityExceeded)?;
        let pending_memory_bytes = self
            .capacity
            .current_memory()
            .checked_mul(2)
            .and_then(|bytes| bytes.checked_add(event_copy_budget))
            .ok_or(GenerationSessionError::SessionCapacityExceeded)?;
        let memory = self.capacity.reserve_memory(pending_memory_bytes)?;
        let mut prepared_snapshot = state.snapshot.clone();
        let mut prepared_chunks = state.next_chunk_indexes.clone();
        let mut prepared_events = Vec::with_capacity(events.len());
        let mut next_sequence = first_sequence;
        for event in events.iter().cloned() {
            let emitted_at_ms = now_ms()?;
            let envelope = GenerationTaskEvent {
                schema_version: GENERATION_EVENT_SCHEMA_VERSION,
                task_id: prepared_snapshot.task_id.clone(),
                attempt: prepared_snapshot.attempt,
                sequence: next_sequence,
                emitted_at_ms,
                event,
            };
            apply_event(
                &mut prepared_snapshot,
                &mut prepared_chunks,
                &envelope,
                ApplyMode::Committed,
            )?;
            prepared_events.push(envelope);
            next_sequence = next_sequence.checked_add(1).ok_or(
                GenerationSessionError::InvalidSequenceLease {
                    current: next_sequence,
                    next: u64::MAX,
                },
            )?;
        }
        let last_sequence = prepared_events.last().expect("non-empty batch").sequence;
        let mut prepared_replay = state.replay.clone();
        let mut prepared_replay_bytes = state.replay_bytes;
        for event in &prepared_events {
            prepared_replay_bytes = prepared_replay_bytes
                .checked_add(encoded_event_bytes(event)?)
                .ok_or(GenerationSessionError::SessionCapacityExceeded)?;
            prepared_replay.push_back(event.clone());
        }
        trim_replay(
            &mut prepared_replay,
            &mut prepared_replay_bytes,
            last_sequence,
            &self.config,
        )?;
        let memory_bytes =
            session_memory_bytes(&prepared_snapshot, prepared_replay_bytes, &self.config)?;
        if memory_bytes > pending_memory_bytes {
            return Err(GenerationSessionError::SessionCapacityExceeded);
        }
        state.durable_commit_in_flight = true;
        Ok(CommittedEventPermit {
            session: Arc::clone(self),
            events: prepared_events,
            snapshot: Some(prepared_snapshot),
            replay: Some(prepared_replay),
            replay_bytes: prepared_replay_bytes,
            next_chunk_indexes: Some(prepared_chunks),
            first_sequence,
            last_sequence,
            committed_memory_bytes: memory_bytes,
            memory: Some(memory),
            committed: false,
        })
    }

    async fn publish_ordinary(
        &self,
        input: OrdinaryInput,
        replenisher: &dyn GenerationSequenceLeaseReplenisher,
    ) -> Result<(), GenerationSessionError> {
        loop {
            let notified = self.durable_commit_notify.notified();
            let action = {
                let mut state = self
                    .state
                    .lock()
                    .expect("generation session mutex poisoned");
                if state.durable_commit_in_flight {
                    OrdinaryAction::Wait
                } else if let Some(reason) = state.ordinary_publisher_sealed.clone() {
                    return Err(GenerationSessionError::PublisherSealed(reason));
                } else if state.snapshot.status.is_terminal() {
                    return Err(GenerationSessionError::TerminalAlreadySet);
                } else if !matches!(
                    state.snapshot.status,
                    GenerationTaskStatus::Reserved | GenerationTaskStatus::Running
                ) {
                    return Err(GenerationSessionError::InvalidState {
                        status: state.snapshot.status,
                        operation: "ordinary publish",
                    });
                } else if state
                    .reserved_through_sequence
                    .saturating_sub(state.snapshot.covers_through_sequence)
                    <= SEQUENCE_EMERGENCY_SLOTS
                {
                    OrdinaryAction::Replenish
                } else {
                    append_ordinary_locked(
                        &mut state,
                        &self.config,
                        &self.capacity,
                        &self.latest_sequence,
                        input.clone(),
                    )?;
                    OrdinaryAction::Published
                }
            };

            match action {
                OrdinaryAction::Published => return Ok(()),
                OrdinaryAction::Wait => notified.await,
                OrdinaryAction::Replenish => {
                    let _guard = self.checkpoint_write_lock.lock().await;
                    let (task_id, attempt, checkpoint_revision, current, required) = {
                        let state = self
                            .state
                            .lock()
                            .expect("generation session mutex poisoned");
                        if state.durable_commit_in_flight {
                            continue;
                        }
                        if state
                            .reserved_through_sequence
                            .saturating_sub(state.snapshot.covers_through_sequence)
                            > SEQUENCE_EMERGENCY_SLOTS
                        {
                            continue;
                        }
                        let current = state.reserved_through_sequence;
                        let Some(required) = current.checked_add(SEQUENCE_LEASE_BLOCK) else {
                            let reason = "sequence lease 已达到上限".to_string();
                            drop(state);
                            self.seal_ordinary_publisher(reason.clone());
                            return Err(GenerationSessionError::SequenceLeaseUnavailable(reason));
                        };
                        (
                            state.snapshot.task_id.clone(),
                            state.snapshot.attempt,
                            state.checkpoint_revision,
                            current,
                            required,
                        )
                    };
                    let next = replenisher
                        .replenish(
                            &task_id,
                            attempt,
                            checkpoint_revision,
                            current,
                            SEQUENCE_LEASE_BLOCK,
                        )
                        .await;
                    match next {
                        Ok(next)
                            if next.checkpoint_revision > checkpoint_revision
                                && next.reserved_through_sequence >= required
                                && next.reserved_through_sequence <= JS_SAFE_INTEGER_U64 =>
                        {
                            let mut state = self
                                .state
                                .lock()
                                .expect("generation session mutex poisoned");
                            if state.checkpoint_revision != checkpoint_revision
                                || state.reserved_through_sequence != current
                            {
                                let reason = "sequence lease cursor 在扩租期间发生变化".to_string();
                                drop(state);
                                self.seal_ordinary_publisher(reason.clone());
                                return Err(GenerationSessionError::SequenceLeaseUnavailable(
                                    reason,
                                ));
                            }
                            state.checkpoint_revision = next.checkpoint_revision;
                            state.reserved_through_sequence = next.reserved_through_sequence;
                        }
                        Ok(next) => {
                            let reason = format!(
                                "replenisher 返回 revision={} high-water={}，预期 revision>{checkpoint_revision} 且 high-water 在 {required}..={JS_SAFE_INTEGER_U64}",
                                next.checkpoint_revision, next.reserved_through_sequence
                            );
                            self.seal_ordinary_publisher(reason.clone());
                            return Err(GenerationSessionError::SequenceLeaseUnavailable(reason));
                        }
                        Err(error) => {
                            let reason = error.to_string();
                            self.seal_ordinary_publisher(reason.clone());
                            return Err(GenerationSessionError::SequenceLeaseUnavailable(reason));
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn seal_ordinary_publisher(&self, reason: String) {
        let mut state = self
            .state
            .lock()
            .expect("generation session mutex poisoned");
        state.ordinary_publisher_sealed = Some(reason);
    }

    pub fn latest_sequence(&self) -> u64 {
        self.state
            .lock()
            .expect("generation session mutex poisoned")
            .snapshot
            .covers_through_sequence
    }
}

enum OrdinaryAction {
    Published,
    Wait,
    Replenish,
}

fn replay_after_locked(state: &SessionState, after_sequence: Option<u64>) -> GenerationReplay {
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

fn append_ordinary_locked(
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

fn apply_event(
    snapshot: &mut GenerationTaskSnapshot,
    next_chunk_indexes: &mut BTreeMap<String, u64>,
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
            snapshot.progress = Some(event.clone());
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
    let mut next_chunk_indexes = BTreeMap::new();
    apply_event(
        snapshot,
        &mut next_chunk_indexes,
        envelope,
        ApplyMode::Committed,
    )
}

fn apply_text_delta(
    snapshot: &mut GenerationTaskSnapshot,
    next_chunk_indexes: &mut BTreeMap<String, u64>,
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
    if let Some(previous) = &snapshot.usage {
        if usage.prompt_tokens < previous.prompt_tokens
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
        {
            return Err(GenerationSessionError::UsageRegression);
        }
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

pub struct CommittedEventPermit {
    session: Arc<GenerationTaskSession>,
    events: Vec<GenerationTaskEvent>,
    snapshot: Option<GenerationTaskSnapshot>,
    replay: Option<VecDeque<GenerationTaskEvent>>,
    replay_bytes: usize,
    next_chunk_indexes: Option<BTreeMap<String, u64>>,
    first_sequence: u64,
    last_sequence: u64,
    committed_memory_bytes: usize,
    memory: Option<MemoryReservation>,
    committed: bool,
}

impl CommittedEventPermit {
    pub fn events(&self) -> &[GenerationTaskEvent] {
        &self.events
    }

    pub fn snapshot(&self) -> &GenerationTaskSnapshot {
        self.snapshot
            .as_ref()
            .expect("committed event permit snapshot consumed")
    }

    pub fn first_sequence(&self) -> u64 {
        self.first_sequence
    }

    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    pub fn commit(mut self, commit: GenerationDurableCommit) -> Result<(), GenerationSessionError> {
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
pub trait GenerationEventPublisher: Send + Sync {
    async fn progress(&self, event: ProgressEvent) -> Result<(), GenerationSessionError>;
    async fn text_delta(&self, stream_id: &str, delta: &str) -> Result<(), GenerationSessionError>;
    async fn usage(&self, event: UsageEvent) -> Result<(), GenerationSessionError>;
}

#[derive(Clone)]
pub struct SessionGenerationEventPublisher {
    session: Arc<GenerationTaskSession>,
    replenisher: Arc<dyn GenerationSequenceLeaseReplenisher>,
}

impl SessionGenerationEventPublisher {
    pub fn new(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::artifacts::GenerationArtifactCoordinator;
    use a4n_service_protocol::{
        GenerationDeliveryMode, GenerationEvent, GenerationFinishReason, GenerationUsage,
        StartedEvent,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestReplenisher {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl GenerationSequenceLeaseReplenisher for TestReplenisher {
        async fn replenish(
            &self,
            _task_id: &str,
            _attempt: u32,
            expected_checkpoint_revision: i64,
            current_reserved_through_sequence: u64,
            block_size: u64,
        ) -> Result<GenerationSequenceLeaseUpdate, GenerationSessionError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(GenerationSequenceLeaseUpdate {
                checkpoint_revision: expected_checkpoint_revision + 1,
                reserved_through_sequence: current_reserved_through_sequence + block_size,
            })
        }
    }

    fn started(sequence: u64) -> GenerationTaskEvent {
        started_for("11111111-1111-4111-8111-111111111111", sequence, 1)
    }

    fn started_for(task_id: &str, sequence: u64, emitted_at_ms: i64) -> GenerationTaskEvent {
        GenerationTaskEvent {
            schema_version: GENERATION_EVENT_SCHEMA_VERSION,
            task_id: task_id.to_string(),
            attempt: 1,
            sequence,
            emitted_at_ms,
            event: GenerationEvent::Started(StartedEvent {
                task_kind: "test".to_string(),
                delivery_mode: GenerationDeliveryMode::Stream,
                project_id: None,
                root_project_id: None,
                workflow_run_id: None,
                generation_run_id: None,
                target: None,
                batch_policy: None,
                root_task_id: task_id.to_string(),
                parent_task_id: None,
                item_key: None,
                retry_of_task_id: None,
            }),
        }
    }

    fn terminal_snapshot(task_id: &str, updated_at_ms: i64) -> GenerationTaskSnapshot {
        let started = match started_for(task_id, 1, updated_at_ms - 1).event {
            GenerationEvent::Started(started) => started,
            _ => unreachable!(),
        };
        let terminal_event = GenerationTaskEvent {
            schema_version: GENERATION_EVENT_SCHEMA_VERSION,
            task_id: task_id.to_string(),
            attempt: 1,
            sequence: 2,
            emitted_at_ms: updated_at_ms,
            event: GenerationEvent::Completed(a4n_service_protocol::CompletedEvent {
                result_ref: None,
                final_text_hashes: Vec::new(),
                final_usage: None,
                finish_reason: Some(GenerationFinishReason::Stop),
                summary: None,
                batch_summary: None,
            }),
        };
        GenerationTaskSnapshot {
            schema_version: GENERATION_EVENT_SCHEMA_VERSION,
            task_id: task_id.to_string(),
            attempt: 1,
            covers_through_sequence: 2,
            status: GenerationTaskStatus::Completed,
            started,
            cancel_reason: None,
            progress: None,
            streams: Vec::new(),
            artifacts: Vec::new(),
            usage: None,
            latest_checkpoint: None,
            terminal_event: Some(terminal_event),
            updated_at_ms,
        }
    }

    fn controller() -> GenerationAdmissionController {
        GenerationAdmissionController::new(GenerationSessionConfig::default()).unwrap()
    }

    fn session() -> Arc<GenerationTaskSession> {
        let controller = controller();
        let permit = controller.try_admit().unwrap();
        GenerationTaskSession::from_started_event(started(1), 257, permit).unwrap()
    }

    fn progress() -> ProgressEvent {
        ProgressEvent {
            stage_code: "prepare".to_string(),
            phase_kind: a4n_service_protocol::ProgressPhaseKind::Prepare,
            message: "x".to_string(),
            current: None,
            total: None,
            unit_label: None,
            item_key: None,
            item_index: None,
            item_total: None,
            severity: a4n_service_protocol::ProgressSeverity::Info,
            estimated: false,
        }
    }

    fn durable_commit(revision: i64, covers_through_sequence: u64) -> GenerationDurableCommit {
        GenerationDurableCommit {
            checkpoint_revision: revision,
            covers_through_sequence,
            reserved_through_sequence: 257,
            persisted_at_ms: 10,
        }
    }

    #[tokio::test]
    async fn publisher_assigns_utf8_offsets_and_accumulates_usage() {
        let session = session();
        let replenisher = Arc::new(TestReplenisher {
            calls: AtomicUsize::new(0),
        });
        let publisher = SessionGenerationEventPublisher::new(session.clone(), replenisher);

        publisher.text_delta("text", "A中文🙂é").await.unwrap();
        publisher.text_delta("text", "第二块").await.unwrap();
        publisher
            .usage(UsageEvent {
                usage: GenerationUsage {
                    prompt_tokens: 2,
                    completion_tokens: 3,
                    total_tokens: 5,
                    prompt_cache_hit_tokens: None,
                    prompt_cache_miss_tokens: None,
                },
                cumulative: true,
            })
            .await
            .unwrap();

        let snapshot = session.snapshot();
        assert_eq!(snapshot.covers_through_sequence, 4);
        assert_eq!(
            snapshot.streams[0].next_offset,
            "A中文🙂é第二块".len() as u64
        );
        assert_eq!(snapshot.usage.unwrap().total_tokens, 5);
        match session.replay_after(Some(1)) {
            GenerationReplay::Events { events, .. } => {
                assert_eq!(events.len(), 3);
                assert_eq!(events[0].sequence, 2);
            }
            GenerationReplay::SnapshotRequired { .. } => panic!("replay should be available"),
        }
    }

    #[tokio::test]
    async fn checkpoint_mark_keeps_events_published_after_the_candidate_dirty() {
        let session = session();
        let replenisher = Arc::new(TestReplenisher {
            calls: AtomicUsize::new(0),
        });
        let publisher = SessionGenerationEventPublisher::new(session.clone(), replenisher);
        publisher.progress(progress()).await.unwrap();
        let persisted = session.checkpoint_candidate();
        publisher.progress(progress()).await.unwrap();

        session
            .mark_snapshot_persisted(GenerationDurableCommit {
                checkpoint_revision: 2,
                covers_through_sequence: persisted.snapshot.covers_through_sequence,
                reserved_through_sequence: session.reserved_through_sequence(),
                persisted_at_ms: 100,
            })
            .unwrap();

        let remaining = session.checkpoint_candidate();
        assert_eq!(remaining.snapshot.covers_through_sequence, 3);
        assert_eq!(remaining.checkpoint_revision, 2);
        assert!(remaining.unpersisted_event_bytes > 0);
        assert!(!session.checkpoint_due(&remaining, 100, false));
        assert!(session.checkpoint_due(&remaining, 2_100, false));
    }

    #[tokio::test]
    async fn committed_permit_accounts_pending_memory_and_does_not_consume_sequence() {
        let controller = controller();
        let permit = controller.try_admit().unwrap();
        let session = GenerationTaskSession::from_started_event(started(1), 257, permit).unwrap();
        let replenisher = Arc::new(TestReplenisher {
            calls: AtomicUsize::new(0),
        });
        let publisher = SessionGenerationEventPublisher::new(session.clone(), replenisher);
        let memory_before = controller.snapshot().session_memory_bytes;
        let permit = session
            .prepare_committed_events(vec![GenerationEvent::StatusChanged(
                a4n_service_protocol::StatusChangedEvent {
                    status: GenerationTaskStatus::Running,
                },
            )])
            .unwrap();
        assert!(controller.snapshot().session_memory_bytes > memory_before);
        assert!(matches!(
            session.prepare_committed_events(vec![GenerationEvent::StatusChanged(
                a4n_service_protocol::StatusChangedEvent {
                    status: GenerationTaskStatus::Running,
                },
            )]),
            Err(GenerationSessionError::InvalidSession(_))
        ));

        let publish = tokio::spawn(async move { publisher.progress(progress()).await });
        tokio::task::yield_now().await;
        assert!(!publish.is_finished());
        drop(permit);
        publish.await.unwrap().unwrap();
        assert_eq!(session.latest_sequence(), 2);
        assert_eq!(controller.snapshot().session_memory_bytes, memory_before);

        let permit = session
            .prepare_committed_events(vec![GenerationEvent::StatusChanged(
                a4n_service_protocol::StatusChangedEvent {
                    status: GenerationTaskStatus::Running,
                },
            )])
            .unwrap();
        assert_eq!(permit.first_sequence(), 3);
        permit.commit(durable_commit(2, 3)).unwrap();
        assert_eq!(session.snapshot().status, GenerationTaskStatus::Running);
        assert_eq!(session.latest_sequence(), 3);
    }

    #[tokio::test]
    async fn artifact_and_completed_share_one_commit_without_consuming_rollback_sequence() {
        let session = session();
        let events = || {
            vec![
                GenerationEvent::StatusChanged(a4n_service_protocol::StatusChangedEvent {
                    status: GenerationTaskStatus::Running,
                }),
                GenerationEvent::StatusChanged(a4n_service_protocol::StatusChangedEvent {
                    status: GenerationTaskStatus::Finalizing,
                }),
                GenerationEvent::Artifact(
                    GenerationArtifactCoordinator::available(
                        a4n_service_protocol::GenerationArtifactRef {
                            artifact_id: "portrait:42".to_string(),
                            artifact_kind: "portrait_image".to_string(),
                            content_type: "image/png".to_string(),
                            sha256: "a".repeat(64),
                            byte_size: 4,
                            revision: "a".repeat(64),
                            temporary: false,
                            expires_at_ms: None,
                        },
                        None,
                        None,
                        None,
                    )
                    .into_event(),
                ),
                GenerationEvent::Completed(a4n_service_protocol::CompletedEvent {
                    result_ref: None,
                    final_text_hashes: Vec::new(),
                    final_usage: None,
                    finish_reason: Some(GenerationFinishReason::Stop),
                    summary: None,
                    batch_summary: None,
                }),
            ]
        };

        let permit = session.prepare_committed_events(events()).unwrap();
        assert_eq!(permit.first_sequence(), 2);
        assert_eq!(permit.last_sequence(), 5);
        assert!(matches!(
            permit.events()[2].event,
            GenerationEvent::Artifact(_)
        ));
        assert!(matches!(
            permit.events()[3].event,
            GenerationEvent::Completed(_)
        ));
        drop(permit);
        assert_eq!(session.latest_sequence(), 1);

        let permit = session.prepare_committed_events(events()).unwrap();
        assert_eq!(permit.first_sequence(), 2);
        permit.commit(durable_commit(2, 5)).unwrap();
        let snapshot = session.snapshot();
        assert_eq!(snapshot.status, GenerationTaskStatus::Completed);
        assert_eq!(snapshot.artifacts.len(), 1);
        assert_eq!(snapshot.covers_through_sequence, 5);
    }

    #[tokio::test]
    async fn terminal_batch_releases_active_slot_and_closes_publisher() {
        let controller = controller();
        let permit = controller.try_admit().unwrap();
        let session = GenerationTaskSession::from_started_event(started(1), 257, permit).unwrap();
        let replenisher = Arc::new(TestReplenisher {
            calls: AtomicUsize::new(0),
        });
        let publisher = SessionGenerationEventPublisher::new(session.clone(), replenisher);

        session
            .prepare_committed_events(vec![
                GenerationEvent::StatusChanged(a4n_service_protocol::StatusChangedEvent {
                    status: GenerationTaskStatus::Running,
                }),
                GenerationEvent::StatusChanged(a4n_service_protocol::StatusChangedEvent {
                    status: GenerationTaskStatus::Finalizing,
                }),
                GenerationEvent::Completed(a4n_service_protocol::CompletedEvent {
                    result_ref: None,
                    final_text_hashes: Vec::new(),
                    final_usage: None,
                    finish_reason: Some(GenerationFinishReason::Stop),
                    summary: None,
                    batch_summary: None,
                }),
            ])
            .unwrap()
            .commit(durable_commit(2, 4))
            .unwrap();

        assert_eq!(session.snapshot().status, GenerationTaskStatus::Completed);
        assert!(publisher.text_delta("text", "late").await.is_err());
        assert_eq!(controller.snapshot().active_sessions, 0);
        let _new_permit = controller.try_admit().unwrap();
    }

    #[tokio::test]
    async fn lease_replenishes_before_emergency_slots() {
        let session = session();
        let replenisher = Arc::new(TestReplenisher {
            calls: AtomicUsize::new(0),
        });
        let publisher = SessionGenerationEventPublisher::new(session.clone(), replenisher.clone());
        for _ in 0..241 {
            publisher.progress(progress()).await.unwrap();
        }
        publisher.progress(progress()).await.unwrap();
        assert_eq!(replenisher.calls.load(Ordering::Relaxed), 1);
        assert!(session.reserved_through_sequence() >= 513);
    }

    #[tokio::test]
    async fn subscription_watch_observes_event_after_atomic_registration() {
        let session = session();
        let replenisher = Arc::new(TestReplenisher {
            calls: AtomicUsize::new(0),
        });
        let publisher = SessionGenerationEventPublisher::new(session.clone(), replenisher);
        let mut subscription = session.subscribe(Some(1)).unwrap();
        match subscription.take_initial().unwrap() {
            GenerationReplay::Events { events, .. } => assert!(events.is_empty()),
            GenerationReplay::SnapshotRequired { .. } => panic!("initial replay should be empty"),
        }

        publisher.progress(progress()).await.unwrap();
        subscription.receiver().changed().await.unwrap();
        assert_eq!(*subscription.receiver().borrow(), 2);
    }

    #[test]
    fn snapshot_only_session_requires_snapshot_ack_before_active_replay() {
        let source = session();
        let snapshot = source.snapshot();
        drop(source);

        let controller = controller();
        let admission = controller.try_admit().unwrap();
        let recovered =
            GenerationTaskSession::from_snapshot_only(snapshot, 1, 257, 1, admission).unwrap();

        assert!(matches!(
            recovered.replay_after(Some(1)),
            GenerationReplay::SnapshotRequired { .. }
        ));

        let mut blocked = recovered.subscribe(None).unwrap();
        assert!(matches!(
            blocked.take_initial().unwrap(),
            GenerationReplay::SnapshotRequired { .. }
        ));

        let mut acknowledged = recovered.subscribe(Some(1)).unwrap();
        match acknowledged.take_initial().unwrap() {
            GenerationReplay::Events { events, .. } => assert!(events.is_empty()),
            GenerationReplay::SnapshotRequired { .. } => {
                panic!("snapshot covers cursor should acknowledge an active recovery session")
            }
        }
        match recovered.replay_after(Some(1)) {
            GenerationReplay::Events { events, .. } => assert!(events.is_empty()),
            GenerationReplay::SnapshotRequired { .. } => {
                panic!("acknowledged recovery session should expose replay")
            }
        }
    }

    #[test]
    fn terminal_snapshot_only_session_never_reopens_event_replay() {
        let controller = controller();
        let admission = controller.try_admit().unwrap();
        let recovered = GenerationTaskSession::from_snapshot_only(
            terminal_snapshot("33333333-3333-4333-8333-333333333333", 10),
            2,
            257,
            10,
            admission,
        )
        .unwrap();

        let mut subscription = recovered.subscribe(Some(2)).unwrap();
        assert!(matches!(
            subscription.take_initial().unwrap(),
            GenerationReplay::SnapshotRequired { .. }
        ));
    }

    #[test]
    fn admission_rejects_the_65th_active_session() {
        let controller = controller();
        let mut sessions = Vec::new();
        for _ in 0..64 {
            let permit = controller.try_admit().unwrap();
            sessions
                .push(GenerationTaskSession::from_started_event(started(1), 257, permit).unwrap());
        }
        assert!(matches!(
            controller.try_admit(),
            Err(GenerationSessionError::SessionCapacityExceeded)
        ));
        drop(sessions.pop());
        assert!(controller.try_admit().is_ok());
    }

    #[test]
    fn subscription_limits_are_shared_by_task_and_host() {
        let controller = controller();
        let mut sessions = Vec::new();
        for _ in 0..9 {
            let permit = controller.try_admit().unwrap();
            sessions
                .push(GenerationTaskSession::from_started_event(started(1), 257, permit).unwrap());
        }
        let mut subscriptions = Vec::new();
        for session in sessions.iter().take(8) {
            for _ in 0..128 {
                subscriptions.push(session.subscribe(Some(1)).unwrap());
            }
        }
        assert!(matches!(
            sessions[0].subscribe(Some(1)),
            Err(GenerationSessionError::SubscriptionCapacityExceeded)
        ));
        assert!(matches!(
            sessions[8].subscribe(Some(1)),
            Err(GenerationSessionError::SubscriptionCapacityExceeded)
        ));
        assert_eq!(controller.snapshot().subscriptions, 1_024);

        subscriptions.pop();
        let replacement = sessions[8].subscribe(Some(1)).unwrap();
        assert_eq!(controller.snapshot().subscriptions, 1_024);
        drop(replacement);
    }

    #[test]
    fn registry_evicts_oldest_persisted_terminal_and_expires_by_ttl() {
        let mut config = GenerationSessionConfig::default();
        config.session_admission_base_bytes = 64 * 1024 * 1024;
        config.global_session_memory_bytes = 128 * 1024 * 1024;
        let ttl = config.terminal_memory_ttl;
        let registry = GenerationSessionRegistry::new(config).unwrap();
        let now = now_ms().unwrap();
        let oldest_id = "11111111-1111-4111-8111-111111111111";
        let newest_id = "22222222-2222-4222-8222-222222222222";

        for (task_id, updated_at_ms) in [(oldest_id, now - 1_000), (newest_id, now)] {
            let admission = registry.try_admit().unwrap();
            let session = GenerationTaskSession::from_snapshot(
                terminal_snapshot(task_id, updated_at_ms),
                Vec::new(),
                2,
                257,
                updated_at_ms,
                admission,
            )
            .unwrap();
            registry.insert(session).unwrap();
        }

        let admission = registry.try_admit().unwrap();
        assert_eq!(registry.len(), 1);
        assert!(registry.get(oldest_id, 1).unwrap().is_none());
        assert!(registry.get(newest_id, 2).is_err());
        drop(admission);

        assert_eq!(
            registry.cleanup_terminal_sessions_at(now + duration_ms_i64(ttl)),
            1
        );
        assert_eq!(registry.len(), 0);
        assert_eq!(
            registry.admission.snapshot(),
            GenerationCapacitySnapshot {
                active_sessions: 0,
                session_memory_bytes: 0,
                subscriptions: 0,
            }
        );
    }

    #[tokio::test]
    async fn one_hundred_subscribers_keep_interleaved_tasks_isolated_and_release_capacity() {
        const FIRST_TASK: &str = "11111111-1111-4111-8111-111111111111";
        const SECOND_TASK: &str = "22222222-2222-4222-8222-222222222222";

        let controller = controller();
        let first = GenerationTaskSession::from_started_event(
            started_for(FIRST_TASK, 1, 1),
            257,
            controller.try_admit().unwrap(),
        )
        .unwrap();
        let second = GenerationTaskSession::from_started_event(
            started_for(SECOND_TASK, 1, 1),
            257,
            controller.try_admit().unwrap(),
        )
        .unwrap();
        let first_publisher = SessionGenerationEventPublisher::new(
            first.clone(),
            Arc::new(TestReplenisher {
                calls: AtomicUsize::new(0),
            }),
        );
        let second_publisher = SessionGenerationEventPublisher::new(
            second.clone(),
            Arc::new(TestReplenisher {
                calls: AtomicUsize::new(0),
            }),
        );
        let mut first_subscriptions = (0..50)
            .map(|_| first.subscribe(Some(1)).unwrap())
            .collect::<Vec<_>>();
        let mut second_subscriptions = (0..50)
            .map(|_| second.subscribe(Some(1)).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(controller.snapshot().subscriptions, 100);

        for _ in 0..100 {
            first_publisher.text_delta("main", "a").await.unwrap();
            second_publisher.text_delta("main", "b").await.unwrap();
        }

        for (subscriptions, task_id, expected) in [
            (&mut first_subscriptions[..], FIRST_TASK, "a".repeat(100)),
            (&mut second_subscriptions[..], SECOND_TASK, "b".repeat(100)),
        ] {
            for subscription in subscriptions {
                subscription.receiver().changed().await.unwrap();
                let GenerationReplay::Events { events, .. } = subscription.replay_after(Some(1))
                else {
                    panic!("subscriber cursor should stay inside replay")
                };
                assert_eq!(events.len(), 100);
                assert!(events.iter().all(|event| event.task_id == task_id));
                let text = events
                    .iter()
                    .map(|event| match &event.event {
                        GenerationEvent::TextDelta(delta) => delta.delta.as_str(),
                        _ => panic!("interleaved stream should contain only text deltas"),
                    })
                    .collect::<String>();
                assert_eq!(text, expected);
            }
        }
        let first_snapshot = first.snapshot();
        let second_snapshot = second.snapshot();
        assert_eq!(first_snapshot.streams[0].text, "a".repeat(100));
        assert_eq!(second_snapshot.streams[0].text, "b".repeat(100));
        assert_eq!(
            first_snapshot.streams[0].sha256,
            format!("{:x}", Sha256::digest("a".repeat(100).as_bytes()))
        );
        assert_eq!(
            second_snapshot.streams[0].sha256,
            format!("{:x}", Sha256::digest("b".repeat(100).as_bytes()))
        );

        drop(first_subscriptions);
        drop(second_subscriptions);
        drop(first_publisher);
        drop(second_publisher);
        drop(first);
        drop(second);
        assert_eq!(
            controller.snapshot(),
            GenerationCapacitySnapshot {
                active_sessions: 0,
                session_memory_bytes: 0,
                subscriptions: 0,
            }
        );
    }

    #[tokio::test]
    async fn replay_floor_requires_snapshot_after_persisted_events_are_evicted() {
        let mut config = GenerationSessionConfig::default();
        config.replay_max_events = 256;
        let controller = GenerationAdmissionController::new(config).unwrap();
        let permit = controller.try_admit().unwrap();
        let session = GenerationTaskSession::from_started_event(started(1), 257, permit).unwrap();
        let replenisher = Arc::new(TestReplenisher {
            calls: AtomicUsize::new(0),
        });
        let publisher = SessionGenerationEventPublisher::new(session.clone(), replenisher);
        for _ in 0..256 {
            publisher.text_delta("main", "x").await.unwrap();
        }

        let recovered = match session.replay_after(None) {
            GenerationReplay::SnapshotRequired {
                snapshot,
                replay_floor,
                ..
            } => {
                assert_eq!(replay_floor, 2);
                snapshot
            }
            GenerationReplay::Events { .. } => panic!("cursor before replay floor needs snapshot"),
        };
        assert_eq!(recovered.covers_through_sequence, 257);
        assert_eq!(recovered.streams[0].text, "x".repeat(256));

        session
            .mark_snapshot_persisted(durable_commit(3, 257))
            .unwrap();
        publisher.text_delta("main", "tail").await.unwrap();
        match session.replay_after(Some(recovered.covers_through_sequence)) {
            GenerationReplay::Events { events, .. } => {
                assert_eq!(events.len(), 1);
                let GenerationEvent::TextDelta(delta) = &events[0].event else {
                    panic!("recovered cursor should receive the next text delta")
                };
                assert_eq!(delta.offset, 256);
                assert_eq!(
                    recovered.streams[0].text.clone() + &delta.delta,
                    "x".repeat(256) + "tail"
                );
            }
            GenerationReplay::SnapshotRequired { .. } => {
                panic!("snapshot cursor should remain inside replay")
            }
        };
    }
}
