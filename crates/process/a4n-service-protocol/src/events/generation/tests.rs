use super::*;

use a4n_generation_artifact_contract::GenerationArtifactRef;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::Digest;

const TASK_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const SHA256_HELLO: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

#[test]
fn page_scoped_streams_fit_a_long_chapter_without_removing_byte_limits() {
    let empty_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let mut snapshot = GenerationTaskSnapshot {
        schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: TASK_ID.into(),
        attempt: 1,
        covers_through_sequence: 1,
        status: GenerationTaskStatus::Running,
        started: started(TASK_ID),
        cancel_reason: None,
        progress: None,
        streams: (0..202)
            .map(|index| TextStreamSnapshot {
                stream_id: format!("page_{index}"),
                text: String::new(),
                next_offset: 0,
                sha256: empty_hash.into(),
            })
            .collect(),
        artifacts: vec![],
        usage: None,
        latest_checkpoint: None,
        terminal_event: None,
        updated_at_ms: 0,
    };
    snapshot.validate().unwrap();
    let schema = schemars::schema_for!(GenerationTaskSnapshot).to_value();
    assert_eq!(
        schema["properties"]["streams"]["maxItems"],
        MAX_GENERATION_STREAMS
    );
    snapshot.streams = (0..=MAX_GENERATION_STREAMS)
        .map(|index| TextStreamSnapshot {
            stream_id: format!("page_{index}"),
            text: String::new(),
            next_offset: 0,
            sha256: empty_hash.into(),
        })
        .collect();
    assert!(
        snapshot
            .validate()
            .unwrap_err()
            .to_string()
            .contains("snapshot text stream")
    );
    snapshot.streams.truncate(1);
    let text = "x".repeat(MAX_GENERATION_SNAPSHOT_BYTES);
    snapshot.streams[0].sha256 = format!("{:x}", sha2::Sha256::digest(text.as_bytes()));
    snapshot.streams[0].next_offset = text.len() as u64;
    snapshot.streams[0].text = text;
    assert!(snapshot.validate().is_err());
}

#[test]
fn terminal_hashes_cover_a_hundred_pages_with_one_correction_per_call() {
    let hashes = (0..1_002)
        .map(|index| StreamTextHash {
            stream_id: format!("page_{index}_content.correction"),
            utf8_bytes: 100_000,
            sha256: SHA256_HELLO.into(),
        })
        .collect::<Vec<_>>();
    let event = envelope(
        2,
        GenerationEvent::Completed(CompletedEvent {
            result_ref: None,
            final_text_hashes: hashes,
            final_usage: None,
            finish_reason: Some(GenerationFinishReason::Stop),
            summary: None,
            batch_summary: None,
        }),
    );
    assert!(serde_json::to_vec(&event).unwrap().len() > 64 * 1024);
    event.validate().unwrap();
}

fn started(task_id: &str) -> StartedEvent {
    StartedEvent {
        task_kind: "llm.chat".to_string(),
        delivery_mode: GenerationDeliveryMode::Stream,
        project_id: Some(1),
        root_project_id: Some(1),
        workflow_run_id: None,
        generation_run_id: None,
        target: Some(GenerationTargetRef {
            kind: "chapter".to_string(),
            id: "42".to_string(),
            revision: Some("1".to_string()),
        }),
        batch_policy: None,
        root_task_id: task_id.to_string(),
        parent_task_id: None,
        item_key: None,
        retry_of_task_id: None,
    }
}

fn envelope(sequence: u64, event: GenerationEvent) -> GenerationTaskEvent {
    GenerationTaskEvent {
        schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: TASK_ID.to_string(),
        attempt: 1,
        sequence,
        emitted_at_ms: 0,
        event,
    }
}

fn artifact(temporary: bool) -> GenerationArtifactRef {
    GenerationArtifactRef {
        artifact_id: "artifact-1".to_string(),
        artifact_kind: "image".to_string(),
        content_type: "image/png".to_string(),
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        byte_size: 0,
        revision: "1".to_string(),
        temporary,
        expires_at_ms: temporary.then_some(1_000),
    }
}

#[test]
fn started_event_round_trips_and_validates() {
    let event = envelope(1, GenerationEvent::Started(started(TASK_ID)));
    let encoded = serde_json::to_vec(&event).expect("started event serializes");
    let decoded: GenerationTaskEvent =
        serde_json::from_slice(&encoded).expect("started event deserializes");
    assert_eq!(decoded, event);
    decoded.validate().expect("started event is valid");
}

#[test]
fn event_validator_rejects_invalid_identity_and_sequence() {
    let mut invalid_uuid = envelope(1, GenerationEvent::Started(started(TASK_ID)));
    invalid_uuid.task_id = "00000000-0000-1000-8000-000000000000".to_string();
    assert!(invalid_uuid.validate().is_err());

    let invalid_sequence = envelope(0, GenerationEvent::Started(started(TASK_ID)));
    assert!(invalid_sequence.validate().is_err());
}

#[test]
fn event_validator_rejects_usage_text_and_artifact_contract_errors() {
    let usage = GenerationUsage {
        prompt_tokens: 2,
        completion_tokens: 3,
        total_tokens: 4,
        prompt_cache_hit_tokens: None,
        prompt_cache_miss_tokens: None,
        prompt_cache_write_tokens: None,
    };
    assert!(
        envelope(
            2,
            GenerationEvent::Usage(UsageEvent {
                usage,
                cumulative: true,
            }),
        )
        .validate()
        .is_err()
    );

    let text = envelope(
        2,
        GenerationEvent::TextDelta(TextDeltaEvent {
            stream_id: "raw_json".to_string(),
            offset: JS_SAFE_INTEGER_U64 + 1,
            delta: "x".to_string(),
            chunk_index: 0,
        }),
    );
    assert!(text.validate().is_err());

    let invalid_preview = envelope(
        2,
        GenerationEvent::Artifact(ArtifactEvent {
            artifact: artifact(false),
            operation: ArtifactOperation::PreviewAvailable,
            replaces_artifact_id: None,
            item_key: None,
            item_index: None,
            item_total: None,
        }),
    );
    assert!(invalid_preview.validate().is_err());
}

#[test]
fn snapshot_rejects_text_hash_offset_and_terminal_status_mismatch() {
    let stream = TextStreamSnapshot {
        stream_id: "raw_json".to_string(),
        text: "hello".to_string(),
        next_offset: 4,
        sha256: SHA256_HELLO.to_string(),
    };
    let failed = GenerationTaskEvent {
        schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: TASK_ID.to_string(),
        attempt: 1,
        sequence: 2,
        emitted_at_ms: 0,
        event: GenerationEvent::Failed(FailedEvent {
            error: GenerationError {
                code: GenerationErrorCode::registered("generation_event_invalid").unwrap(),
                message: "invalid".to_string(),
                retryable: false,
                stage_code: "validate".to_string(),
                trace_id: None,
                source_error: None,
            },
            draft_disposition: DraftDisposition::Discarded,
            batch_summary: None,
        }),
    };
    let snapshot = GenerationTaskSnapshot {
        schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: TASK_ID.to_string(),
        attempt: 1,
        covers_through_sequence: 2,
        status: GenerationTaskStatus::Completed,
        started: started(TASK_ID),
        cancel_reason: None,
        progress: None,
        streams: vec![stream],
        artifacts: vec![artifact(false)],
        usage: None,
        latest_checkpoint: None,
        terminal_event: Some(failed),
        updated_at_ms: 0,
    };
    assert!(snapshot.validate().is_err());
}

#[test]
fn corrupt_batch_failure_can_omit_unrecoverable_summary() {
    let mut batch_started = started(TASK_ID);
    batch_started.batch_policy = Some(BatchCommitPolicy::BestEffort);
    let terminal = GenerationTaskEvent {
        schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: TASK_ID.to_string(),
        attempt: 1,
        sequence: 2,
        emitted_at_ms: 0,
        event: GenerationEvent::Failed(FailedEvent {
            error: GenerationError {
                code: GenerationErrorCode::registered("generation_checkpoint_corrupt").unwrap(),
                message: "checkpoint 损坏".to_string(),
                retryable: false,
                stage_code: "recovery".to_string(),
                trace_id: None,
                source_error: None,
            },
            draft_disposition: DraftDisposition::RetainedUntilExpiry,
            batch_summary: None,
        }),
    };
    let snapshot = GenerationTaskSnapshot {
        schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: TASK_ID.to_string(),
        attempt: 1,
        covers_through_sequence: 2,
        status: GenerationTaskStatus::Failed,
        started: batch_started.clone(),
        cancel_reason: None,
        progress: None,
        streams: Vec::new(),
        artifacts: Vec::new(),
        usage: None,
        latest_checkpoint: None,
        terminal_event: Some(terminal),
        updated_at_ms: 0,
    };
    snapshot
        .validate()
        .expect("corrupt batch recovery does not invent a summary");

    let mut ordinary_terminal = snapshot.terminal_event.clone().unwrap();
    if let GenerationEvent::Failed(event) = &mut ordinary_terminal.event {
        event.error.code = GenerationErrorCode::registered("generation_event_invalid").unwrap();
    }
    let mut ordinary_snapshot = snapshot;
    ordinary_snapshot.terminal_event = Some(ordinary_terminal);
    assert!(ordinary_snapshot.validate().is_err());
}

#[test]
fn unknown_error_codes_are_rejected_at_deserialization() {
    let error = serde_json::from_value::<GenerationErrorCode>(json!("temporary_unknown"))
        .expect_err("unregistered codes must not enter the wire contract");
    assert!(error.to_string().contains("未注册的生成错误码"));
}

#[test]
fn rust_accepts_checked_generation_fixtures_and_utf8_offsets() {
    for fixture in [
        include_str!("../../../fixtures/generation-events-v1/started.json"),
        include_str!("../../../fixtures/generation-events-v1/text-delta-mixed-unicode.json"),
        include_str!("../../../fixtures/generation-events-v1/text-delta-second-chunk.json"),
    ] {
        let event: GenerationTaskEvent =
            serde_json::from_str(fixture).expect("fixture parses as GenerationTaskEvent");
        event
            .validate()
            .expect("fixture satisfies generation contract");
    }

    #[derive(Deserialize)]
    struct Utf8Vector {
        text: String,
        utf8_bytes: usize,
    }

    let vectors: Vec<Utf8Vector> = serde_json::from_str(include_str!(
        "../../../fixtures/generation-events-v1/utf8-vectors.json"
    ))
    .expect("UTF-8 vectors parse");
    assert!(
        vectors
            .iter()
            .all(|vector| vector.text.len() == vector.utf8_bytes)
    );
}

#[test]
fn generation_schema_is_strict_and_marks_safe_integer_boundaries() {
    let schema = serde_json::to_value(schemars::schema_for!(GenerationTaskEvent))
        .expect("schema serializes");
    assert_eq!(
        schema.get("$schema").and_then(Value::as_str),
        Some("https://json-schema.org/draft/2020-12/schema")
    );
    assert_eq!(
        schema.get("additionalProperties").and_then(Value::as_bool),
        Some(false)
    );
    let json_text = schema.to_string();
    assert!(json_text.contains("9007199254740991"));
}
