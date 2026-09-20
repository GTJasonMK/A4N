use std::collections::{BTreeMap, BTreeSet};

use a4n_generation_artifact_contract::GenerationArtifactRef;
use sha2::{Digest, Sha256};
use uuid::{Uuid, Version};

use super::super::generation_error_codes::is_registered_generation_error_code;
use super::*;
use super::{
    JS_SAFE_INTEGER_I64, JS_SAFE_INTEGER_U64, MAX_ERROR_MESSAGE_BYTES, MAX_FAILED_ITEM_KEYS,
    MAX_GENERATION_ARTIFACTS, MAX_GENERATION_EVENT_BYTES, MAX_GENERATION_SNAPSHOT_BYTES,
    MAX_GENERATION_STREAMS, MAX_IDENTIFIER_BYTES, MAX_KEY_BYTES, MAX_MESSAGE_BYTES,
    MAX_STREAM_ID_BYTES, MAX_TEXT_DELTA_BYTES,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("生成事件合同无效: {0}")]
pub struct GenerationContractError(String);

pub(super) fn validate_generation_task_event(
    event: &GenerationTaskEvent,
) -> Result<(), GenerationContractError> {
    if event.schema_version != GENERATION_EVENT_SCHEMA_VERSION {
        return Err(invalid(format!(
            "schema_version 必须为 {GENERATION_EVENT_SCHEMA_VERSION}"
        )));
    }
    validate_uuid_v4("task_id", &event.task_id)?;
    if event.attempt == 0 {
        return Err(invalid("attempt 必须大于 0"));
    }
    validate_safe_u64("sequence", event.sequence, false)?;
    validate_safe_i64("emitted_at_ms", event.emitted_at_ms)?;

    match &event.event {
        GenerationEvent::Started(started) => {
            if event.sequence != 1 {
                return Err(invalid("started 固定使用 sequence=1"));
            }
            validate_started_event(started, &event.task_id)?;
        }
        GenerationEvent::StatusChanged(status) => {
            require_after_started(event.sequence)?;
            if !matches!(
                status.status,
                GenerationTaskStatus::Running | GenerationTaskStatus::Finalizing
            ) {
                return Err(invalid("status_changed 只允许 running 或 finalizing"));
            }
        }
        GenerationEvent::Progress(progress) => {
            require_after_started(event.sequence)?;
            validate_progress_event(progress)?;
        }
        GenerationEvent::TextDelta(delta) => {
            require_after_started(event.sequence)?;
            validate_text_delta_event(delta)?;
        }
        GenerationEvent::Usage(usage) => {
            require_after_started(event.sequence)?;
            if !usage.cumulative {
                return Err(invalid("V1 usage 必须是累计值"));
            }
            validate_usage(&usage.usage)?;
        }
        GenerationEvent::Checkpoint(checkpoint) => {
            require_after_started(event.sequence)?;
            validate_checkpoint_event(checkpoint)?;
        }
        GenerationEvent::Artifact(artifact) => {
            require_after_started(event.sequence)?;
            validate_artifact_event(artifact)?;
        }
        GenerationEvent::CancelRequested(_) => require_after_started(event.sequence)?,
        GenerationEvent::Completed(completed) => {
            require_after_started(event.sequence)?;
            validate_completed_event(completed)?;
        }
        GenerationEvent::Failed(failed) => {
            require_after_started(event.sequence)?;
            validate_generation_error(&failed.error)?;
            validate_optional_batch_summary(failed.batch_summary.as_ref())?;
        }
        GenerationEvent::Cancelled(cancelled) => {
            require_after_started(event.sequence)?;
            validate_optional_text(
                "cancelled.summary",
                cancelled.summary.as_deref(),
                MAX_MESSAGE_BYTES,
            )?;
            validate_optional_batch_summary(cancelled.batch_summary.as_ref())?;
        }
        GenerationEvent::Interrupted(interrupted) => {
            require_after_started(event.sequence)?;
            validate_optional_text(
                "interrupted.summary",
                interrupted.summary.as_deref(),
                MAX_MESSAGE_BYTES,
            )?;
            validate_optional_batch_summary(interrupted.batch_summary.as_ref())?;
        }
    }

    let bytes =
        serde_json::to_vec(event).map_err(|error| invalid(format!("事件序列化失败: {error}")))?;
    if bytes.len() > MAX_GENERATION_EVENT_BYTES {
        return Err(invalid(format!(
            "单事件超过 {} 字节上限",
            MAX_GENERATION_EVENT_BYTES
        )));
    }
    Ok(())
}

pub(super) fn validate_generation_task_snapshot(
    snapshot: &GenerationTaskSnapshot,
) -> Result<(), GenerationContractError> {
    if snapshot.schema_version != GENERATION_EVENT_SCHEMA_VERSION {
        return Err(invalid(format!(
            "snapshot schema_version 必须为 {GENERATION_EVENT_SCHEMA_VERSION}"
        )));
    }
    validate_uuid_v4("snapshot.task_id", &snapshot.task_id)?;
    if snapshot.attempt == 0 {
        return Err(invalid("snapshot.attempt 必须大于 0"));
    }
    validate_safe_u64(
        "snapshot.covers_through_sequence",
        snapshot.covers_through_sequence,
        false,
    )?;
    validate_safe_i64("snapshot.updated_at_ms", snapshot.updated_at_ms)?;
    validate_started_event(&snapshot.started, &snapshot.task_id)?;

    if snapshot.status == GenerationTaskStatus::Cancelling {
        if snapshot.cancel_reason.is_none() {
            return Err(invalid("cancelling snapshot 必须包含 cancel_reason"));
        }
    } else if snapshot.cancel_reason.is_some() {
        return Err(invalid("仅 cancelling snapshot 可包含 cancel_reason"));
    }

    if let Some(progress) = &snapshot.progress {
        validate_progress_event(progress)?;
    }
    if snapshot.streams.len() > MAX_GENERATION_STREAMS {
        return Err(invalid(format!(
            "snapshot text stream 超过 {MAX_GENERATION_STREAMS} 个"
        )));
    }
    let mut streams = BTreeMap::new();
    for stream in &snapshot.streams {
        validate_stream_id(&stream.stream_id)?;
        validate_safe_u64("stream.next_offset", stream.next_offset, true)?;
        let expected_offset = stream.text.len() as u64;
        if stream.next_offset != expected_offset {
            return Err(invalid(format!(
                "stream {} next_offset={}，实际 UTF-8 字节数={expected_offset}",
                stream.stream_id, stream.next_offset
            )));
        }
        validate_sha256("stream.sha256", &stream.sha256)?;
        let actual_sha = format!("{:x}", Sha256::digest(stream.text.as_bytes()));
        if stream.sha256 != actual_sha {
            return Err(invalid(format!(
                "stream {} sha256 不匹配",
                stream.stream_id
            )));
        }
        if streams.insert(stream.stream_id.as_str(), stream).is_some() {
            return Err(invalid(format!(
                "snapshot stream_id 重复: {}",
                stream.stream_id
            )));
        }
    }

    if snapshot.artifacts.len() > MAX_GENERATION_ARTIFACTS {
        return Err(invalid(format!(
            "snapshot artifact 超过 {MAX_GENERATION_ARTIFACTS} 个"
        )));
    }
    let mut artifact_ids = BTreeSet::new();
    for artifact in &snapshot.artifacts {
        validate_generation_artifact_ref(artifact)?;
        if !artifact_ids.insert(artifact.artifact_id.as_str()) {
            return Err(invalid(format!(
                "snapshot artifact_id 重复: {}",
                artifact.artifact_id
            )));
        }
    }
    if let Some(usage) = &snapshot.usage {
        validate_usage(usage)?;
    }
    if let Some(checkpoint) = &snapshot.latest_checkpoint {
        validate_checkpoint_event(checkpoint)?;
    }

    match (&snapshot.status, &snapshot.terminal_event) {
        (status, None) if !status.is_terminal() => {}
        (status, Some(_)) if !status.is_terminal() => {
            return Err(invalid(format!(
                "active snapshot status={status:?} 不得包含 terminal_event"
            )));
        }
        (status, None) => {
            return Err(invalid(format!(
                "terminal snapshot status={status:?} 必须包含 terminal_event"
            )));
        }
        (status, Some(terminal)) => {
            terminal.validate()?;
            if terminal.task_id != snapshot.task_id
                || terminal.attempt != snapshot.attempt
                || terminal.sequence != snapshot.covers_through_sequence
            {
                return Err(invalid(
                    "terminal_event 的 task_id/attempt/sequence 与 snapshot 不一致",
                ));
            }
            validate_terminal_status(*status, &terminal.event)?;
            validate_terminal_batch_contract(&snapshot.started, &terminal.event)?;
            if let GenerationEvent::Completed(completed) = &terminal.event {
                validate_completed_against_snapshot(completed, &streams, snapshot.usage.as_ref())?;
            }
        }
    }

    let bytes = serde_json::to_vec(snapshot)
        .map_err(|error| invalid(format!("snapshot 序列化失败: {error}")))?;
    if bytes.len() > MAX_GENERATION_SNAPSHOT_BYTES {
        return Err(invalid(format!(
            "snapshot 超过 {} 字节上限",
            MAX_GENERATION_SNAPSHOT_BYTES
        )));
    }
    Ok(())
}

fn validate_started_event(
    started: &StartedEvent,
    task_id: &str,
) -> Result<(), GenerationContractError> {
    validate_identifier(
        "started.task_kind",
        &started.task_kind,
        MAX_IDENTIFIER_BYTES,
    )?;
    validate_optional_safe_i64("started.project_id", started.project_id)?;
    validate_optional_safe_i64("started.root_project_id", started.root_project_id)?;
    validate_optional_safe_i64("started.workflow_run_id", started.workflow_run_id)?;
    validate_optional_safe_i64("started.generation_run_id", started.generation_run_id)?;
    if let Some(target) = &started.target {
        validate_identifier("target.kind", &target.kind, MAX_IDENTIFIER_BYTES)?;
        validate_bounded_nonempty("target.id", &target.id, MAX_KEY_BYTES)?;
        if let Some(revision) = &target.revision {
            validate_bounded_nonempty("target.revision", revision, MAX_KEY_BYTES)?;
        }
    }
    validate_uuid_v4("started.root_task_id", &started.root_task_id)?;
    if let Some(parent_task_id) = &started.parent_task_id {
        validate_uuid_v4("started.parent_task_id", parent_task_id)?;
        if parent_task_id == task_id {
            return Err(invalid("child parent_task_id 不能等于自身 task_id"));
        }
        if started.root_task_id == task_id {
            return Err(invalid("child root_task_id 不能等于自身 task_id"));
        }
        validate_bounded_nonempty(
            "started.item_key",
            started.item_key.as_deref().unwrap_or_default(),
            MAX_KEY_BYTES,
        )?;
    } else {
        if started.root_task_id != task_id {
            return Err(invalid("根任务 root_task_id 必须等于 envelope task_id"));
        }
        if started.item_key.is_some() {
            return Err(invalid("根任务不得包含 item_key"));
        }
    }
    if let Some(retry_of) = &started.retry_of_task_id {
        validate_uuid_v4("started.retry_of_task_id", retry_of)?;
        if retry_of == task_id {
            return Err(invalid("retry_of_task_id 不能等于当前 task_id"));
        }
    }
    Ok(())
}

fn validate_progress_event(progress: &ProgressEvent) -> Result<(), GenerationContractError> {
    validate_identifier(
        "progress.stage_code",
        &progress.stage_code,
        MAX_IDENTIFIER_BYTES,
    )?;
    validate_text("progress.message", &progress.message, MAX_MESSAGE_BYTES)?;
    validate_optional_text(
        "progress.unit_label",
        progress.unit_label.as_deref(),
        MAX_KEY_BYTES,
    )?;
    if let Some(item_key) = &progress.item_key {
        validate_bounded_nonempty("progress.item_key", item_key, MAX_KEY_BYTES)?;
    }
    match (progress.current, progress.total) {
        (None, None) => {}
        (Some(current), Some(total)) => {
            validate_safe_u64("progress.current", current, true)?;
            validate_safe_u64("progress.total", total, false)?;
            if current > total {
                return Err(invalid("progress.current 不能大于 total"));
            }
        }
        _ => return Err(invalid("progress.current/total 必须同时有值或同时为空")),
    }
    match (progress.item_index, progress.item_total) {
        (None, None) => {}
        (Some(index), Some(total)) => {
            validate_safe_u64("progress.item_index", index, true)?;
            validate_safe_u64("progress.item_total", total, false)?;
            if index >= total {
                return Err(invalid("progress.item_index 必须小于 item_total"));
            }
        }
        _ => return Err(invalid("progress.item_index/item_total 必须成对出现")),
    }
    Ok(())
}

fn validate_text_delta_event(delta: &TextDeltaEvent) -> Result<(), GenerationContractError> {
    validate_stream_id(&delta.stream_id)?;
    validate_safe_u64("text_delta.offset", delta.offset, true)?;
    validate_safe_u64("text_delta.chunk_index", delta.chunk_index, true)?;
    if delta.delta.is_empty() {
        return Err(invalid("text_delta.delta 不能为空"));
    }
    if delta.delta.len() > MAX_TEXT_DELTA_BYTES {
        return Err(invalid(format!(
            "text_delta.delta 超过 {MAX_TEXT_DELTA_BYTES} 字节"
        )));
    }
    Ok(())
}

fn validate_usage(usage: &GenerationUsage) -> Result<(), GenerationContractError> {
    validate_safe_u64("usage.prompt_tokens", usage.prompt_tokens, true)?;
    validate_safe_u64("usage.completion_tokens", usage.completion_tokens, true)?;
    validate_safe_u64("usage.total_tokens", usage.total_tokens, true)?;
    let total = usage
        .prompt_tokens
        .checked_add(usage.completion_tokens)
        .ok_or_else(|| invalid("usage token 计数溢出"))?;
    if usage.total_tokens != total {
        return Err(invalid(
            "usage.total_tokens 必须等于 prompt_tokens + completion_tokens",
        ));
    }
    let cache_hit = usage.prompt_cache_hit_tokens.unwrap_or(0);
    let cache_miss = usage.prompt_cache_miss_tokens.unwrap_or(0);
    validate_safe_u64("usage.prompt_cache_hit_tokens", cache_hit, true)?;
    validate_safe_u64("usage.prompt_cache_miss_tokens", cache_miss, true)?;
    validate_safe_u64(
        "usage.prompt_cache_write_tokens",
        usage.prompt_cache_write_tokens.unwrap_or(0),
        true,
    )?;
    if cache_hit
        .checked_add(cache_miss)
        .is_none_or(|value| value > usage.prompt_tokens)
    {
        return Err(invalid("prompt cache hit/miss 计数不能超过 prompt_tokens"));
    }
    Ok(())
}

fn validate_checkpoint_event(checkpoint: &CheckpointEvent) -> Result<(), GenerationContractError> {
    validate_bounded_nonempty(
        "checkpoint.checkpoint_key",
        &checkpoint.checkpoint_key,
        MAX_KEY_BYTES,
    )?;
    validate_optional_text(
        "checkpoint.summary",
        checkpoint.summary.as_deref(),
        MAX_MESSAGE_BYTES,
    )
}

fn validate_artifact_event(artifact: &ArtifactEvent) -> Result<(), GenerationContractError> {
    validate_generation_artifact_ref(&artifact.artifact)?;
    match artifact.operation {
        ArtifactOperation::PreviewAvailable => {
            if !artifact.artifact.temporary
                || artifact.artifact.expires_at_ms.is_none()
                || artifact.replaces_artifact_id.is_some()
            {
                return Err(invalid(
                    "preview_available 必须 temporary=true、包含 expires_at_ms 且不替换其他资产",
                ));
            }
        }
        ArtifactOperation::Available => {
            if artifact.artifact.temporary || artifact.replaces_artifact_id.is_some() {
                return Err(invalid(
                    "available 必须 temporary=false 且 replaces_artifact_id 为空",
                ));
            }
        }
        ArtifactOperation::Updated => {
            let Some(replaced) = artifact.replaces_artifact_id.as_deref() else {
                return Err(invalid("updated 必须包含 replaces_artifact_id"));
            };
            if artifact.artifact.temporary {
                return Err(invalid("updated 必须指向正式非临时资产"));
            }
            validate_bounded_nonempty("artifact.replaces_artifact_id", replaced, MAX_KEY_BYTES)?;
            if replaced == artifact.artifact.artifact_id {
                return Err(invalid("资产不能替换自身"));
            }
        }
        ArtifactOperation::Removed => {
            if artifact.replaces_artifact_id.is_some() {
                return Err(invalid("removed 不得包含 replaces_artifact_id"));
            }
        }
    }
    if let Some(item_key) = &artifact.item_key {
        validate_bounded_nonempty("artifact.item_key", item_key, MAX_KEY_BYTES)?;
    }
    match (artifact.item_index, artifact.item_total) {
        (None, None) => {}
        (Some(index), Some(total)) if total > 0 && index < total => {}
        (Some(_), Some(_)) => {
            return Err(invalid("artifact.item_index 必须小于非零 item_total"));
        }
        _ => return Err(invalid("artifact.item_index/item_total 必须成对出现")),
    }
    Ok(())
}

fn validate_generation_artifact_ref(
    artifact: &GenerationArtifactRef,
) -> Result<(), GenerationContractError> {
    validate_bounded_nonempty("artifact.artifact_id", &artifact.artifact_id, MAX_KEY_BYTES)?;
    validate_identifier(
        "artifact.artifact_kind",
        &artifact.artifact_kind,
        MAX_IDENTIFIER_BYTES,
    )?;
    validate_bounded_nonempty(
        "artifact.content_type",
        &artifact.content_type,
        MAX_KEY_BYTES,
    )?;
    validate_sha256("artifact.sha256", &artifact.sha256)?;
    validate_safe_u64("artifact.byte_size", artifact.byte_size, true)?;
    validate_bounded_nonempty("artifact.revision", &artifact.revision, MAX_KEY_BYTES)?;
    if let Some(expires_at_ms) = artifact.expires_at_ms {
        validate_safe_i64("artifact.expires_at_ms", expires_at_ms)?;
    }
    if artifact.temporary && artifact.expires_at_ms.is_none() {
        return Err(invalid("temporary artifact 必须包含 expires_at_ms"));
    }
    if !artifact.temporary && artifact.expires_at_ms.is_some() {
        return Err(invalid("正式 artifact 不得包含 expires_at_ms"));
    }
    Ok(())
}

fn validate_completed_event(completed: &CompletedEvent) -> Result<(), GenerationContractError> {
    if let Some(result) = &completed.result_ref {
        validate_identifier(
            "completed.result_ref.kind",
            &result.kind,
            MAX_IDENTIFIER_BYTES,
        )?;
        validate_bounded_nonempty("completed.result_ref.id", &result.id, MAX_KEY_BYTES)?;
        validate_bounded_nonempty(
            "completed.result_ref.revision",
            &result.revision,
            MAX_KEY_BYTES,
        )?;
    }
    if completed.final_text_hashes.len() > MAX_GENERATION_STREAMS {
        return Err(invalid(format!(
            "completed.final_text_hashes 超过 {MAX_GENERATION_STREAMS} 项"
        )));
    }
    let mut stream_ids = BTreeSet::new();
    for hash in &completed.final_text_hashes {
        validate_stream_id(&hash.stream_id)?;
        validate_safe_u64(
            "completed.final_text_hash.utf8_bytes",
            hash.utf8_bytes,
            true,
        )?;
        validate_sha256("completed.final_text_hash.sha256", &hash.sha256)?;
        if !stream_ids.insert(hash.stream_id.as_str()) {
            return Err(invalid(format!(
                "completed stream_id 重复: {}",
                hash.stream_id
            )));
        }
    }
    if let Some(usage) = &completed.final_usage {
        validate_usage(usage)?;
    }
    validate_optional_text(
        "completed.summary",
        completed.summary.as_deref(),
        MAX_MESSAGE_BYTES,
    )?;
    validate_optional_batch_summary(completed.batch_summary.as_ref())
}

fn validate_generation_error(error: &GenerationError) -> Result<(), GenerationContractError> {
    if !is_registered_generation_error_code(error.code.as_str()) {
        return Err(invalid(format!("未注册的生成错误码: {}", error.code)));
    }
    validate_text(
        "generation_error.message",
        &error.message,
        MAX_ERROR_MESSAGE_BYTES,
    )?;
    validate_identifier(
        "generation_error.stage_code",
        &error.stage_code,
        MAX_IDENTIFIER_BYTES,
    )?;
    validate_optional_text(
        "generation_error.trace_id",
        error.trace_id.as_deref(),
        MAX_KEY_BYTES,
    )
}

fn validate_optional_batch_summary(
    summary: Option<&BatchSummary>,
) -> Result<(), GenerationContractError> {
    if let Some(summary) = summary {
        validate_batch_summary(summary)?;
    }
    Ok(())
}

fn validate_batch_summary(summary: &BatchSummary) -> Result<(), GenerationContractError> {
    if summary.total == 0 {
        return Err(invalid("batch_summary.total 必须大于 0"));
    }
    let counted = summary
        .succeeded
        .checked_add(summary.failed)
        .and_then(|value| value.checked_add(summary.cancelled))
        .and_then(|value| value.checked_add(summary.skipped))
        .ok_or_else(|| invalid("batch_summary 计数溢出"))?;
    if counted != summary.total {
        return Err(invalid("batch_summary 各计数之和必须等于 total"));
    }
    if summary.failed_item_keys.len() > MAX_FAILED_ITEM_KEYS {
        return Err(invalid(format!(
            "failed_item_keys 最多 {MAX_FAILED_ITEM_KEYS} 项"
        )));
    }
    let mut previous: Option<&str> = None;
    for key in &summary.failed_item_keys {
        validate_bounded_nonempty("batch_summary.failed_item_key", key, MAX_KEY_BYTES)?;
        if previous.is_some_and(|value| value >= key.as_str()) {
            return Err(invalid(
                "failed_item_keys 必须去重并按 UTF-8 字节序严格排序",
            ));
        }
        previous = Some(key);
    }
    if summary.failed_item_keys.len() > summary.failed as usize {
        return Err(invalid("failed_item_keys 数量不能大于 failed"));
    }
    if summary.failed_item_keys_truncated {
        if summary.failed_item_keys.len() >= summary.failed as usize {
            return Err(invalid(
                "truncated=true 时 failed_item_keys 必须是不完整子集",
            ));
        }
    } else if summary.failed_item_keys.len() != summary.failed as usize {
        return Err(invalid(
            "truncated=false 时 failed_item_keys 数量必须等于 failed",
        ));
    }
    Ok(())
}

fn validate_terminal_status(
    status: GenerationTaskStatus,
    event: &GenerationEvent,
) -> Result<(), GenerationContractError> {
    let matches = matches!(
        (status, event),
        (
            GenerationTaskStatus::Completed,
            GenerationEvent::Completed(_)
        ) | (GenerationTaskStatus::Failed, GenerationEvent::Failed(_))
            | (
                GenerationTaskStatus::Cancelled,
                GenerationEvent::Cancelled(_)
            )
            | (
                GenerationTaskStatus::Interrupted,
                GenerationEvent::Interrupted(_)
            )
    );
    if matches {
        Ok(())
    } else {
        Err(invalid(format!(
            "snapshot status={status:?} 与 terminal event 不一致"
        )))
    }
}

fn validate_terminal_batch_contract(
    started: &StartedEvent,
    terminal: &GenerationEvent,
) -> Result<(), GenerationContractError> {
    let summary = match terminal {
        GenerationEvent::Completed(event) => event.batch_summary.as_ref(),
        GenerationEvent::Failed(event) => event.batch_summary.as_ref(),
        GenerationEvent::Cancelled(event) => event.batch_summary.as_ref(),
        GenerationEvent::Interrupted(event) => event.batch_summary.as_ref(),
        _ => return Ok(()),
    };
    match (started.batch_policy, summary) {
        (None, None) => Ok(()),
        (Some(policy), Some(summary)) if summary.policy == policy => {
            if matches!(terminal, GenerationEvent::Completed(_)) {
                match policy {
                    BatchCommitPolicy::Atomic
                        if summary.succeeded != summary.total || summary.failed != 0 =>
                    {
                        return Err(invalid("atomic batch 只有全部成功时才能 completed"));
                    }
                    BatchCommitPolicy::BestEffort if summary.succeeded == 0 => {
                        return Err(invalid("best_effort batch completed 至少需要一个成功子项"));
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        (Some(_), None) => Err(invalid("批量任务 terminal 必须包含 batch_summary")),
        (None, Some(_)) => Err(invalid("非批量任务 terminal 不得包含 batch_summary")),
        (Some(_), Some(_)) => Err(invalid(
            "terminal batch_summary.policy 与 started.batch_policy 不一致",
        )),
    }
}

fn validate_completed_against_snapshot(
    completed: &CompletedEvent,
    streams: &BTreeMap<&str, &TextStreamSnapshot>,
    snapshot_usage: Option<&GenerationUsage>,
) -> Result<(), GenerationContractError> {
    if completed.final_text_hashes.len() != streams.len() {
        return Err(invalid(
            "completed.final_text_hashes 必须覆盖 snapshot 的全部 stream",
        ));
    }
    for hash in &completed.final_text_hashes {
        let Some(stream) = streams.get(hash.stream_id.as_str()) else {
            return Err(invalid(format!(
                "completed 引用了 snapshot 中不存在的 stream: {}",
                hash.stream_id
            )));
        };
        if hash.utf8_bytes != stream.next_offset || hash.sha256 != stream.sha256 {
            return Err(invalid(format!(
                "completed stream {} 的字节数或 sha256 与 snapshot 不一致",
                hash.stream_id
            )));
        }
    }
    if completed.final_usage.as_ref() != snapshot_usage {
        return Err(invalid(
            "completed.final_usage 必须与 snapshot 累计 usage 一致",
        ));
    }
    Ok(())
}

fn require_after_started(sequence: u64) -> Result<(), GenerationContractError> {
    if sequence <= 1 {
        Err(invalid("started 之外的事件 sequence 必须大于 1"))
    } else {
        Ok(())
    }
}

fn validate_uuid_v4(field: &str, value: &str) -> Result<(), GenerationContractError> {
    let parsed =
        Uuid::parse_str(value).map_err(|_| invalid(format!("{field} 必须是 UUID v4 文本")))?;
    if parsed.get_version() != Some(Version::Random) || parsed.hyphenated().to_string() != value {
        return Err(invalid(format!("{field} 必须是规范小写 UUID v4 文本")));
    }
    Ok(())
}

fn validate_identifier(
    field: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), GenerationContractError> {
    validate_bounded_nonempty(field, value, max_bytes)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_.-".contains(&byte))
    {
        return Err(invalid(format!(
            "{field} 只能包含小写 ASCII、数字、下划线、点或连字符"
        )));
    }
    Ok(())
}

fn validate_stream_id(value: &str) -> Result<(), GenerationContractError> {
    validate_identifier("stream_id", value, MAX_STREAM_ID_BYTES)
}

fn validate_bounded_nonempty(
    field: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), GenerationContractError> {
    if value.is_empty() || value.len() > max_bytes {
        return Err(invalid(format!(
            "{field} 必须为 1..={max_bytes} 个 UTF-8 字节"
        )));
    }
    Ok(())
}

fn validate_text(
    field: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), GenerationContractError> {
    if value.len() > max_bytes {
        return Err(invalid(format!("{field} 超过 {max_bytes} 个 UTF-8 字节")));
    }
    Ok(())
}

fn validate_optional_text(
    field: &str,
    value: Option<&str>,
    max_bytes: usize,
) -> Result<(), GenerationContractError> {
    if let Some(value) = value {
        validate_text(field, value, max_bytes)?;
    }
    Ok(())
}

fn validate_sha256(field: &str, value: &str) -> Result<(), GenerationContractError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(invalid(format!("{field} 必须是 64 位小写 SHA-256 hex")));
    }
    Ok(())
}

fn validate_safe_u64(
    field: &str,
    value: u64,
    allow_zero: bool,
) -> Result<(), GenerationContractError> {
    if value > JS_SAFE_INTEGER_U64 || (!allow_zero && value == 0) {
        return Err(invalid(format!(
            "{field} 必须在 {}..={JS_SAFE_INTEGER_U64} 范围内",
            if allow_zero { 0 } else { 1 }
        )));
    }
    Ok(())
}

fn validate_safe_i64(field: &str, value: i64) -> Result<(), GenerationContractError> {
    if !(0..=JS_SAFE_INTEGER_I64).contains(&value) {
        return Err(invalid(format!(
            "{field} 必须在 0..={JS_SAFE_INTEGER_I64} 范围内"
        )));
    }
    Ok(())
}

fn validate_optional_safe_i64(
    field: &str,
    value: Option<i64>,
) -> Result<(), GenerationContractError> {
    if let Some(value) = value {
        validate_safe_i64(field, value)?;
    }
    Ok(())
}

pub(super) fn invalid(message: impl Into<String>) -> GenerationContractError {
    GenerationContractError(message.into())
}
