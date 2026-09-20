use super::registry::{GenerationAdmissionController, GenerationCapacitySnapshot};
use super::*;
use crate::generation::artifacts::GenerationArtifactCoordinator;
use a4n_service_protocol::{
    GenerationDeliveryMode, GenerationEvent, GenerationFinishReason, GenerationUsage, StartedEvent,
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
        phase_kind: a4n_progress::ProgressPhaseKind::Prepare,
        message: "x".to_string(),
        current: None,
        total: None,
        unit_label: None,
        item_key: None,
        item_index: None,
        item_total: None,
        severity: a4n_progress::ProgressSeverity::Info,
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
                prompt_cache_write_tokens: None,
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
async fn snapshot_keeps_progress_counters_when_phase_update_omits_them() {
    let session = session();
    let replenisher = Arc::new(TestReplenisher {
        calls: AtomicUsize::new(0),
    });
    let publisher = SessionGenerationEventPublisher::new(session.clone(), replenisher);
    let mut counted = progress();
    counted.current = Some(2);
    counted.total = Some(12);

    publisher.progress(counted).await.unwrap();
    publisher.progress(progress()).await.unwrap();

    let snapshot = session.snapshot();
    let progress = snapshot.progress.expect("progress snapshot should exist");
    assert_eq!(progress.current, Some(2));
    assert_eq!(progress.total, Some(12));
    assert_eq!(progress.message, "x");
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

#[test]
fn cancelling_accepts_durable_text_delta_checkpoint() {
    let session = session();
    session
        .prepare_committed_events(vec![
            GenerationEvent::StatusChanged(a4n_service_protocol::StatusChangedEvent {
                status: GenerationTaskStatus::Running,
            }),
            GenerationEvent::CancelRequested(CancelRequestedEvent {
                reason: a4n_service_protocol::CancellationReason::UserRequested,
            }),
        ])
        .unwrap()
        .commit(durable_commit(2, 3))
        .unwrap();

    let mut events =
        session.next_committed_text_delta_events("draft", &["saved".to_string(), "\n".to_string()]);
    events.push(GenerationEvent::Checkpoint(
        a4n_service_protocol::CheckpointEvent {
            checkpoint_key: "saved-item".to_string(),
            resumable: true,
            summary: None,
        },
    ));
    let permit = session.prepare_committed_events(events).unwrap();
    assert_eq!(permit.snapshot().streams[0].text, "saved\n");
    permit.commit(durable_commit(3, 6)).unwrap();
    assert_eq!(session.snapshot().status, GenerationTaskStatus::Cancelling);
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
        sessions.push(GenerationTaskSession::from_started_event(started(1), 257, permit).unwrap());
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
        sessions.push(GenerationTaskSession::from_started_event(started(1), 257, permit).unwrap());
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
    let config = GenerationSessionConfig {
        session_admission_base_bytes: 64 * 1024 * 1024,
        global_session_memory_bytes: 128 * 1024 * 1024,
        ..Default::default()
    };
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
        registry.insert_or_get(session).unwrap();
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
            let GenerationReplay::Events { events, .. } = subscription.replay_after(Some(1)) else {
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
    let config = GenerationSessionConfig {
        replay_max_events: 256,
        ..Default::default()
    };
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
