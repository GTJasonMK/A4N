use super::*;

pub(super) const SEQUENCE_LEASE_BLOCK: u64 = 256;
pub(super) const SEQUENCE_EMERGENCY_SLOTS: u64 = 16;
const DEFAULT_TERMINAL_MEMORY_TTL: Duration = Duration::from_secs(30 * 60);
const DEFAULT_CHECKPOINT_INTERVAL: Duration = Duration::from_secs(2);
const DEFAULT_CHECKPOINT_BYTE_THRESHOLD: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct GenerationSessionConfig {
    pub(super) replay_max_events: usize,
    pub(super) replay_max_bytes: usize,
    pub(super) active_session_max: usize,
    pub(super) session_admission_base_bytes: usize,
    pub(super) global_session_memory_bytes: usize,
    pub(super) subscriptions_per_task: usize,
    pub(super) subscriptions_per_host: usize,
    pub(super) terminal_memory_ttl: Duration,
    pub(super) checkpoint_interval: Duration,
    pub(super) checkpoint_byte_threshold: usize,
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
    fn validate(&self) -> Result<(), GenerationSessionError> {
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
pub(super) struct GenerationCapacitySnapshot {
    pub(super) active_sessions: usize,
    pub(super) session_memory_bytes: usize,
    pub(super) subscriptions: usize,
}

#[derive(Clone)]
pub(super) struct GenerationAdmissionController {
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

pub(crate) struct SessionAdmissionPermit {
    lease: Option<SessionCapacityLease>,
}

pub(super) struct SessionCapacityLease {
    inner: Arc<SessionCapacityInner>,
}

pub(super) struct SessionCapacityInner {
    controller: Arc<AdmissionInner>,
    state: Mutex<SessionCapacityState>,
}

struct SessionCapacityState {
    bytes: usize,
    active: bool,
}

pub(super) struct MemoryReservation {
    lease: Arc<SessionCapacityInner>,
    old_bytes: usize,
    committed: bool,
}

pub(super) struct HostSubscriptionPermit {
    controller: Arc<AdmissionInner>,
}

impl GenerationAdmissionController {
    pub(super) fn new(config: GenerationSessionConfig) -> Result<Self, GenerationSessionError> {
        config.validate()?;
        Ok(Self {
            inner: Arc::new(AdmissionInner {
                config,
                state: Mutex::new(AdmissionState::default()),
            }),
        })
    }

    pub(super) fn try_admit(&self) -> Result<SessionAdmissionPermit, GenerationSessionError> {
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

    pub(super) fn snapshot(&self) -> GenerationCapacitySnapshot {
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

pub(crate) struct GenerationSessionRegistry {
    pub(super) admission: GenerationAdmissionController,
    sessions: Mutex<HashMap<String, Arc<GenerationTaskSession>>>,
}

impl GenerationSessionRegistry {
    pub(crate) fn new(config: GenerationSessionConfig) -> Result<Self, GenerationSessionError> {
        Ok(Self {
            admission: GenerationAdmissionController::new(config)?,
            sessions: Mutex::new(HashMap::new()),
        })
    }

    pub(crate) fn try_admit(&self) -> Result<SessionAdmissionPermit, GenerationSessionError> {
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

    pub(crate) fn insert_or_get(
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
                    "task {task_id} 已登记 attempt {existing_attempt}，不能插入 attempt {attempt}"
                )));
            }
            return Ok(Arc::clone(existing));
        }
        sessions.insert(task_id, Arc::clone(&session));
        Ok(session)
    }

    pub(crate) fn get(
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

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.sessions
            .lock()
            .expect("generation session registry mutex poisoned")
            .len()
    }

    pub(super) fn cleanup_terminal_sessions_at(&self, now_ms: i64) -> usize {
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

    pub(super) fn evict_oldest_terminal(&self) -> bool {
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
    pub(super) fn activate(mut self) -> SessionCapacityLease {
        self.lease
            .take()
            .expect("session admission permit already consumed")
    }
}

impl SessionCapacityLease {
    pub(super) fn config(&self) -> &GenerationSessionConfig {
        &self.inner.controller.config
    }

    pub(super) fn current_memory(&self) -> usize {
        self.inner
            .state
            .lock()
            .expect("session capacity mutex poisoned")
            .bytes
    }

    pub(super) fn reserve_memory(
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

    pub(super) fn set_memory(&self, target_bytes: usize) -> Result<(), GenerationSessionError> {
        self.reserve_memory(target_bytes)?.commit(target_bytes);
        Ok(())
    }

    pub(super) fn release_active_slot(&self) {
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

    pub(super) fn try_acquire_subscription(
        &self,
    ) -> Result<HostSubscriptionPermit, GenerationSessionError> {
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
    pub(super) fn commit(mut self, target_bytes: usize) {
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
