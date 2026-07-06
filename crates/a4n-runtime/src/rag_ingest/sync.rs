use crate::state::AppState;
use a4n_core::rag::RagDataType;
use a4n_engine::embedding::provider::EmbeddingModelIdentity;
use a4n_engine::rag::vector_store::DeleteFilter;
use a4n_services::rag::{RagEmbeddingInput, RagService};
use futures::stream::{self, StreamExt};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tokio_util::sync::CancellationToken;

const RAG_SYNC_MAX_CONCURRENCY: usize = 4;

#[derive(Default)]
pub struct ExpectedSyncStats {
    pub item_count: u32,
    pub chunk_count: u32,
    pub added_count: u32,
    pub updated_count: u32,
    pub skipped_count: u32,
    pub deleted_count: u32,
    pub errors: Vec<String>,
}

impl ExpectedSyncStats {
    pub(crate) fn record_outcome(&mut self, outcome: RagRecordSyncOutcome) {
        match outcome {
            RagRecordSyncOutcome::Added => {
                self.added_count = self.added_count.saturating_add(1);
                self.chunk_count = self.chunk_count.saturating_add(1);
            }
            RagRecordSyncOutcome::Updated => {
                self.updated_count = self.updated_count.saturating_add(1);
                self.chunk_count = self.chunk_count.saturating_add(1);
            }
            RagRecordSyncOutcome::Skipped => {
                self.skipped_count = self.skipped_count.saturating_add(1);
            }
        }
    }

    pub fn merge(&mut self, other: ExpectedSyncStats) {
        self.item_count = self.item_count.saturating_add(other.item_count);
        self.chunk_count = self.chunk_count.saturating_add(other.chunk_count);
        self.added_count = self.added_count.saturating_add(other.added_count);
        self.updated_count = self.updated_count.saturating_add(other.updated_count);
        self.skipped_count = self.skipped_count.saturating_add(other.skipped_count);
        self.deleted_count = self.deleted_count.saturating_add(other.deleted_count);
        self.errors.extend(other.errors);
    }
}

pub type RagProgressReporter<'a> = Option<&'a (dyn Fn(u32, u32, &str) + Send + Sync)>;

pub struct RagIngestProgress<'a> {
    current: u32,
    total: u32,
    reporter: RagProgressReporter<'a>,
    cancel_token: Option<&'a CancellationToken>,
}

impl<'a> RagIngestProgress<'a> {
    pub fn new(
        total: usize,
        reporter: RagProgressReporter<'a>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        let total = u32::try_from(total).unwrap_or(u32::MAX).max(1);
        Self {
            current: 0,
            total,
            reporter,
            cancel_token,
        }
    }

    pub fn stage(&self, message: impl AsRef<str>) -> Result<(), String> {
        self.check_cancelled()?;
        self.report(message.as_ref());
        Ok(())
    }

    pub fn advance(&mut self, amount: u32, message: impl AsRef<str>) -> Result<(), String> {
        self.check_cancelled()?;
        self.current = self.current.saturating_add(amount).min(self.total);
        self.report(message.as_ref());
        Ok(())
    }

    pub fn check_cancelled(&self) -> Result<(), String> {
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err("RAG 同步已停止".to_string());
        }
        Ok(())
    }

    fn report(&self, message: &str) {
        if let Some(report) = self.reporter {
            report(self.current, self.total, message);
        }
    }
}

pub(super) fn progress_stage(
    progress: &mut Option<&mut RagIngestProgress<'_>>,
    message: impl AsRef<str>,
) -> Result<(), String> {
    if let Some(progress) = progress.as_deref_mut() {
        progress.stage(message)?;
    }
    Ok(())
}

fn progress_advance(
    progress: &mut Option<&mut RagIngestProgress<'_>>,
    amount: u32,
    message: impl AsRef<str>,
) -> Result<(), String> {
    if let Some(progress) = progress.as_deref_mut() {
        progress.advance(amount, message)?;
    }
    Ok(())
}

fn sync_progress_message(outcome: RagRecordSyncOutcome, label: impl AsRef<str>) -> String {
    let label = label.as_ref();
    match outcome {
        RagRecordSyncOutcome::Added => format!("已新增{label}"),
        RagRecordSyncOutcome::Updated => format!("已更新{label}"),
        RagRecordSyncOutcome::Skipped => format!("已跳过未变化：{label}"),
    }
}

pub(super) async fn cleanup_stale_records(
    sync: &mut RagSyncContext,
    state: &AppState,
    stats: &mut ExpectedSyncStats,
    progress: &mut Option<&mut RagIngestProgress<'_>>,
    label: &str,
) -> Result<(), String> {
    let deleted = sync.cleanup_stale(state).await?;
    if deleted > 0 {
        stats.deleted_count = stats.deleted_count.saturating_add(deleted);
        progress_stage(progress, format!("已清理过期{label} RAG 记录 {deleted} 个"))?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub(crate) struct RagRecord {
    pub(crate) chunk_id: String,
    pub(crate) text: String,
    pub(crate) metadata: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RagRecordSyncOutcome {
    Added,
    Updated,
    Skipped,
}

#[derive(Debug, Clone)]
struct PreparedRagRecordSync {
    record: RagRecord,
    outcome: RagRecordSyncOutcome,
}

#[derive(Debug, Clone)]
pub(super) struct LabeledRagRecord {
    record: RagRecord,
    label: String,
    error_context: String,
}

#[derive(Debug, Clone)]
struct PreparedLabeledRagRecord {
    prepared: PreparedRagRecordSync,
    label: String,
    error_context: String,
}

pub(crate) struct RagSyncContext {
    pub(super) project_id: i64,
    pub(super) embedding_identity: EmbeddingModelIdentity,
    pub(super) stored_metadata: HashMap<String, serde_json::Value>,
    pub(super) stale_ids: HashSet<String>,
}

impl RagSyncContext {
    async fn for_ids(state: &AppState, project_id: i64, ids: &[String]) -> Result<Self, String> {
        let stored_metadata = state
            .rag
            .vector_store
            .metadata_by_ids(&project_id.to_string(), ids)
            .await
            .map_err(|e| format!("读取 RAG 已同步记录失败: {e}"))?;
        let embedding_identity = state
            .embedding
            .model_identity()
            .await
            .map_err(|e| format!("读取当前嵌入模型身份失败: {e}"))?;
        Ok(Self {
            project_id,
            embedding_identity,
            stored_metadata,
            stale_ids: HashSet::new(),
        })
    }

    pub(crate) async fn for_id_prefixes(
        state: &AppState,
        project_id: i64,
        prefixes: Vec<String>,
    ) -> Result<Self, String> {
        Self::for_filter(
            state,
            project_id,
            DeleteFilter {
                project_id,
                data_type: None,
                chapter_number: None,
                ids: Vec::new(),
                id_prefixes: prefixes,
            },
        )
        .await
    }

    pub(super) async fn for_data_type(
        state: &AppState,
        project_id: i64,
        data_type: RagDataType,
    ) -> Result<Self, String> {
        Self::for_id_prefixes(
            state,
            project_id,
            vec![rag_id_prefix(project_id, data_type)?],
        )
        .await
    }

    async fn for_filter(
        state: &AppState,
        project_id: i64,
        filter: DeleteFilter,
    ) -> Result<Self, String> {
        let stored_metadata = state
            .rag
            .vector_store
            .metadata_by_filter(&project_id.to_string(), filter)
            .await
            .map_err(|e| format!("读取 RAG 已同步记录失败: {e}"))?;
        let stale_ids = stored_metadata.keys().cloned().collect();
        let embedding_identity = state
            .embedding
            .model_identity()
            .await
            .map_err(|e| format!("读取当前嵌入模型身份失败: {e}"))?;
        Ok(Self {
            project_id,
            embedding_identity,
            stored_metadata,
            stale_ids,
        })
    }

    pub(crate) async fn sync_record(
        &mut self,
        state: &AppState,
        record: &RagRecord,
    ) -> Result<RagRecordSyncOutcome, String> {
        let prepared = self.prepare_record(record.clone())?;
        Self::store_prepared_record(state, self.project_id, prepared).await
    }

    fn prepare_record(&mut self, record: RagRecord) -> Result<PreparedRagRecordSync, String> {
        let mut record = record;
        record.metadata = root_text_family_metadata(self.project_id, record.metadata)?;
        let content_hash = RagService::content_hash(&record.text).map_err(|e| e.to_string())?;
        let outcome = self.classify_record_hash(&record.chunk_id, &content_hash);
        Ok(PreparedRagRecordSync { record, outcome })
    }

    async fn store_prepared_record(
        state: &AppState,
        project_id: i64,
        prepared: PreparedRagRecordSync,
    ) -> Result<RagRecordSyncOutcome, String> {
        let outcome = prepared.outcome;
        if outcome == RagRecordSyncOutcome::Skipped {
            return Ok(RagRecordSyncOutcome::Skipped);
        }

        state
            .rag
            .embed_and_store(
                project_id,
                &prepared.record.chunk_id,
                &prepared.record.text,
                state.embedding.as_ref(),
                prepared.record.metadata,
            )
            .await
            .map_err(|e| e.to_string())?;

        Ok(outcome)
    }

    pub(super) fn classify_record_hash(
        &mut self,
        chunk_id: &str,
        content_hash: &str,
    ) -> RagRecordSyncOutcome {
        let stored = self.stored_metadata.get(chunk_id);
        let already_synced = stored.is_some_and(|metadata| {
            RagService::metadata_content_hash(metadata)
                .is_some_and(|stored_hash| stored_hash == content_hash)
                && RagService::metadata_matches_current_text_index(metadata)
                && RagService::metadata_matches_embedding_identity(
                    metadata,
                    &self.embedding_identity,
                )
        });

        self.stale_ids.remove(chunk_id);
        if already_synced {
            RagRecordSyncOutcome::Skipped
        } else if stored.is_some() {
            RagRecordSyncOutcome::Updated
        } else {
            RagRecordSyncOutcome::Added
        }
    }

    pub(crate) async fn cleanup_stale(&mut self, state: &AppState) -> Result<u32, String> {
        if self.stale_ids.is_empty() {
            return Ok(0);
        }
        let mut ids = self.stale_ids.drain().collect::<Vec<_>>();
        ids.sort();
        let deleted = state
            .rag
            .vector_store
            .delete(
                &self.project_id.to_string(),
                DeleteFilter {
                    project_id: self.project_id,
                    data_type: None,
                    chapter_number: None,
                    ids,
                    id_prefixes: Vec::new(),
                },
            )
            .await
            .map_err(|e| format!("清理过期 RAG 记录失败: {e}"))?;
        Ok(u32::try_from(deleted).unwrap_or(u32::MAX))
    }
}

pub(crate) fn root_text_family_metadata(project_id: i64, metadata: Value) -> Result<Value, String> {
    let Value::Object(mut object) = metadata else {
        return Err("RAG 记录 metadata 必须是 JSON 对象".to_string());
    };
    let source_chapter_id = metadata_positive_i64(&object, "chapter_id")
        .map(serde_json::Value::from)
        .unwrap_or(Value::Null);
    insert_or_validate(
        &mut object,
        "root_project_id",
        serde_json::json!(project_id),
    )?;
    insert_or_validate(&mut object, "source_snapshot_id", Value::Null)?;
    insert_or_validate(&mut object, "source_chapter_id", source_chapter_id)?;
    insert_or_validate(&mut object, "source_snapshot_chapter_id", Value::Null)?;
    insert_or_validate(&mut object, "derived_project_id", Value::Null)?;
    Ok(Value::Object(object))
}

fn insert_or_validate(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    expected: Value,
) -> Result<(), String> {
    match object.get(key) {
        Some(value) if value == &expected => Ok(()),
        Some(value) => Err(format!(
            "RAG 记录 metadata {key} 与文本根项目上下文不一致: expected={expected}, actual={value}"
        )),
        None => {
            object.insert(key.to_string(), expected);
            Ok(())
        }
    }
}

fn metadata_positive_i64(object: &serde_json::Map<String, Value>, key: &str) -> Option<i64> {
    let value = object.get(key)?;
    let parsed = value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))?;
    (parsed > 0).then_some(parsed)
}

fn rag_id_prefix(project_id: i64, data_type: RagDataType) -> Result<String, String> {
    match data_type {
        RagDataType::Chapter
        | RagDataType::ChapterSummary
        | RagDataType::ChapterMetadata
        | RagDataType::KeyEvent => Err(format!(
            "{} 需要按具体章节清理，不能按共享章节前缀清理",
            data_type.as_str()
        )),
        RagDataType::Foreshadowing => Ok(format!("foreshadowing:{project_id}:")),
        RagDataType::Outline => Ok(format!("outline:{project_id}:")),
        RagDataType::PartOutline => Ok(format!("part_outline:{project_id}:")),
        RagDataType::Blueprint => Ok(format!("blueprint:{project_id}:")),
        RagDataType::BlueprintMetadata => Ok(format!("blueprint_metadata:{project_id}:")),
        RagDataType::WorldSetting => Ok(format!("world_setting:{project_id}:")),
        RagDataType::Character => Ok(format!("character:{project_id}:")),
        RagDataType::Protagonist => Ok(format!("protagonist:{project_id}:")),
        RagDataType::CharacterState => Ok(format!("character_state:{project_id}:")),
        RagDataType::CharacterAttributeEvent => {
            Ok(format!("character_attribute_event:{project_id}:"))
        }
        RagDataType::Relationship => Ok(format!("relationship:{project_id}:")),
        RagDataType::SceneProfile => Ok(format!("scene_profile:{project_id}:")),
        RagDataType::SceneOccurrence => Ok(format!("scene_occurrence:{project_id}:")),
        RagDataType::SceneTransition => Ok(format!("scene_transition:{project_id}:")),
    }
}

pub(super) fn labeled_rag_record(
    record: RagRecord,
    label: impl Into<String>,
    error_context: impl Into<String>,
) -> LabeledRagRecord {
    LabeledRagRecord {
        record,
        label: label.into(),
        error_context: error_context.into(),
    }
}

pub(super) fn required_record_metadata_str(
    record: &RagRecord,
    key: &str,
    context: impl AsRef<str>,
) -> Result<String, String> {
    record
        .metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("{} 缺少 {key} 元数据", context.as_ref()))
}

pub(super) fn required_record_dimension(
    record: &RagRecord,
    context: impl AsRef<str>,
) -> Result<String, String> {
    required_record_metadata_str(record, "dimension", context)
}

pub(super) async fn sync_labeled_records_concurrent(
    sync: &mut RagSyncContext,
    state: &AppState,
    records: Vec<LabeledRagRecord>,
    stats: &mut ExpectedSyncStats,
    progress: &mut Option<&mut RagIngestProgress<'_>>,
) -> Result<u32, String> {
    let mut processed = 0u32;
    let mut pending = Vec::new();

    for item in records {
        progress_stage(progress, format!("正在比对 RAG 记录：{}", item.label))?;
        let prepared = match sync.prepare_record(item.record) {
            Ok(prepared) => prepared,
            Err(err) => {
                stats
                    .errors
                    .push(format!("{} 入库失败: {err}", item.error_context));
                progress_advance(
                    progress,
                    1,
                    format!("{}同步失败，继续处理后续记录", item.label),
                )?;
                continue;
            }
        };

        if prepared.outcome == RagRecordSyncOutcome::Skipped {
            processed = processed.saturating_add(1);
            stats.record_outcome(prepared.outcome);
            progress_advance(
                progress,
                1,
                sync_progress_message(prepared.outcome, item.label),
            )?;
            continue;
        }

        pending.push(PreparedLabeledRagRecord {
            prepared,
            label: item.label,
            error_context: item.error_context,
        });
    }

    let project_id = sync.project_id;
    let batch_size = state.embedding.batch_size().max(1);
    let batches = pending
        .chunks(batch_size)
        .map(<[PreparedLabeledRagRecord]>::to_vec)
        .collect::<Vec<_>>();
    let mut pending_stream = stream::iter(batches.into_iter().map(|batch| async move {
        let inputs = batch
            .iter()
            .map(|item| RagEmbeddingInput {
                chunk_id: item.prepared.record.chunk_id.clone(),
                text: item.prepared.record.text.clone(),
                metadata: item.prepared.record.metadata.clone(),
            })
            .collect::<Vec<_>>();
        let result = state
            .rag
            .embed_and_store_records(project_id, inputs, state.embedding.as_ref())
            .await
            .map_err(|e| e.to_string());
        (result, batch)
    }))
    .buffer_unordered(RAG_SYNC_MAX_CONCURRENCY);

    while let Some((result, batch)) = pending_stream.next().await {
        match result {
            Ok(_) => {
                for item in batch {
                    let outcome = item.prepared.outcome;
                    processed = processed.saturating_add(1);
                    stats.record_outcome(outcome);
                    progress_advance(progress, 1, sync_progress_message(outcome, item.label))?;
                }
            }
            Err(err) => {
                for item in batch {
                    stats
                        .errors
                        .push(format!("{} 入库失败: {err}", item.error_context));
                    progress_advance(
                        progress,
                        1,
                        format!("{}同步失败，继续处理后续记录", item.label),
                    )?;
                }
            }
        }
    }

    Ok(processed)
}

pub(super) async fn sync_data_type_records(
    state: &AppState,
    project_id: i64,
    data_type: RagDataType,
    records: Vec<LabeledRagRecord>,
    stats: &mut ExpectedSyncStats,
    progress: &mut Option<&mut RagIngestProgress<'_>>,
    cleanup_label: &str,
) -> Result<u32, String> {
    let mut sync = RagSyncContext::for_data_type(state, project_id, data_type).await?;
    let processed =
        sync_labeled_records_concurrent(&mut sync, state, records, stats, progress).await?;
    cleanup_stale_records(&mut sync, state, stats, progress, cleanup_label).await?;
    Ok(processed)
}

pub(crate) async fn embed_rag_record(
    state: &AppState,
    project_id: i64,
    record: &RagRecord,
) -> Result<(), String> {
    let mut sync =
        RagSyncContext::for_ids(state, project_id, std::slice::from_ref(&record.chunk_id)).await?;
    sync.sync_record(state, record).await.map(|_| ())
}
