use super::*;

pub(super) struct SessionState {
    pub(super) snapshot: GenerationTaskSnapshot,
    pub(super) replay: VecDeque<GenerationTaskEvent>,
    pub(super) replay_bytes: usize,
    pub(super) checkpoint_revision: i64,
    pub(super) persisted_covers_through_sequence: u64,
    pub(super) reserved_through_sequence: u64,
    pub(super) last_persisted_at_ms: i64,
    pub(super) unpersisted_event_bytes: usize,
    pub(super) next_chunk_indexes: HashMap<String, u64>,
    pub(super) durable_commit_in_flight: bool,
    pub(super) ordinary_publisher_sealed: Option<String>,
    pub(super) snapshot_only_covers: Option<u64>,
    pub(super) subscriptions: usize,
}

pub(crate) struct GenerationTaskSession {
    pub(super) config: GenerationSessionConfig,
    pub(super) state: Mutex<SessionState>,
    pub(super) latest_sequence: watch::Sender<u64>,
    pub(super) durable_commit_notify: Notify,
    pub(super) checkpoint_write_lock: AsyncMutex<()>,
    pub(super) capacity: SessionCapacityLease,
}

impl GenerationTaskSession {
    #[cfg(test)]
    pub(super) fn from_started_event(
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

    pub(crate) fn from_snapshot(
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

    pub(crate) fn from_snapshot_only(
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

    pub(crate) fn snapshot(&self) -> GenerationTaskSnapshot {
        self.state
            .lock()
            .expect("generation session mutex poisoned")
            .snapshot
            .clone()
    }

    #[cfg(test)]
    pub(super) fn reserved_through_sequence(&self) -> u64 {
        self.state
            .lock()
            .expect("generation session mutex poisoned")
            .reserved_through_sequence
    }

    pub(super) fn identity(&self) -> (String, u32) {
        let state = self
            .state
            .lock()
            .expect("generation session mutex poisoned");
        (state.snapshot.task_id.clone(), state.snapshot.attempt)
    }

    pub(super) fn terminal_persisted_at(&self) -> Option<i64> {
        let state = self
            .state
            .lock()
            .expect("generation session mutex poisoned");
        (state.snapshot.status.is_terminal()
            && state.persisted_covers_through_sequence >= state.snapshot.covers_through_sequence)
            .then_some(state.snapshot.updated_at_ms)
    }
}

impl GenerationTaskSession {
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

    pub(crate) fn next_committed_text_delta_events(
        &self,
        stream_id: &str,
        deltas: &[String],
    ) -> Vec<GenerationEvent> {
        let state = self
            .state
            .lock()
            .expect("generation session mutex poisoned");
        let mut offset = state
            .snapshot
            .streams
            .iter()
            .find(|stream| stream.stream_id == stream_id)
            .map(|stream| stream.next_offset)
            .unwrap_or(0);
        let mut chunk_index = state
            .next_chunk_indexes
            .get(stream_id)
            .copied()
            .unwrap_or(0);
        deltas
            .iter()
            .map(|delta| {
                let event = GenerationEvent::TextDelta(TextDeltaEvent {
                    stream_id: stream_id.to_string(),
                    offset,
                    delta: delta.clone(),
                    chunk_index,
                });
                offset += delta.len() as u64;
                chunk_index += 1;
                event
            })
            .collect()
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

    pub(super) fn replay_after(&self, after_sequence: Option<u64>) -> GenerationReplay {
        let state = self
            .state
            .lock()
            .expect("generation session mutex poisoned");
        replay_after_locked(&state, after_sequence)
    }

    pub(crate) fn subscribe(
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

    pub(crate) fn prepare_committed_events(
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

    pub(super) async fn publish_ordinary(
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

    pub(crate) fn latest_sequence(&self) -> u64 {
        self.state
            .lock()
            .expect("generation session mutex poisoned")
            .snapshot
            .covers_through_sequence
    }
}
