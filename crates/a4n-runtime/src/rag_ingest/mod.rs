mod chapters;
mod characters;
mod expected_records;
mod record_builders;
mod scenes;
mod sync;
#[cfg(test)]
mod tests;

pub(crate) use self::chapters::ingest_chapters_inner;
pub(crate) use self::characters::{
    ingest_character_attribute_events_inner, ingest_character_records_inner,
    ingest_character_states_inner, ingest_foreshadowing_inner, ingest_protagonists_inner,
    ingest_relationships_inner,
};
pub(crate) use self::expected_records::expected_rag_records;
use self::record_builders::{
    blueprint_metadata_rag_record, blueprint_rag_record, blueprint_world_setting_rag_records,
    outline_rag_record, part_outline_rag_record,
};
pub(crate) use self::record_builders::{
    character_attribute_event_rag_record, character_state_rag_record, foreshadowing_rag_record,
};
pub(crate) use self::scenes::{
    ingest_scene_occurrences_inner, ingest_scene_profiles_inner, ingest_scene_transitions_inner,
};
pub use self::sync::{ExpectedSyncStats as IngestStats, RagIngestProgress, RagProgressReporter};
pub(crate) use self::sync::{RagRecord, RagSyncContext, embed_rag_record};
use self::sync::{
    RagRecordSyncOutcome, labeled_rag_record, required_record_metadata_str,
    root_text_family_metadata, sync_data_type_records,
};
use crate::{actions::access, state::AppState};
use a4n_core::models::chapter::{chapter_outline, part_outline};
use a4n_core::models::project::novel_blueprint;
use a4n_core::rag::RagDataType;
use a4n_engine::embedding::provider::EmbeddingModelIdentity;
use a4n_engine::rag::vector_store::DeleteFilter;
use a4n_services::rag::{RagEmbeddedInput, RagService};
use sea_orm::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy)]
pub struct RagIngestTaskScope {
    pub project_id: i64,
    pub expected_records: usize,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreparedClientRagIngestRecord {
    pub chunk_id: String,
    pub text: String,
    pub content_hash: String,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientRagIngestPrepareOutput {
    pub project_id: i64,
    pub embedding_identity: EmbeddingModelIdentity,
    pub records: Vec<PreparedClientRagIngestRecord>,
    pub expected_count: usize,
    pub skipped_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadedRagEmbedding {
    pub chunk_id: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClientRagIngestUploadInput {
    pub embedding_identity: EmbeddingModelIdentity,
    pub embeddings: Vec<UploadedRagEmbedding>,
}

pub(crate) async fn ingest_outlines_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for outline in &outlines {
        let record = match outline_rag_record(project_id, outline) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            format!("第{}章大纲", outline.chapter_number),
            format!("大纲 {}", outline.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::Outline,
        records,
        &mut stats,
        &mut progress,
        "章节大纲",
    )
    .await?;
    stats.item_count = stats.item_count.saturating_add(processed);
    Ok(stats)
}

pub(crate) async fn ingest_part_outlines_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for part in &parts {
        let record = match part_outline_rag_record(project_id, part) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            format!("第{}部分部大纲", part.part_number),
            format!("分部大纲 {}", part.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::PartOutline,
        records,
        &mut stats,
        &mut progress,
        "分部大纲",
    )
    .await?;
    stats.item_count = stats.item_count.saturating_add(processed);
    Ok(stats)
}

pub(crate) async fn ingest_blueprints_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let blueprints = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for blueprint in &blueprints {
        let record = match blueprint_rag_record(project_id, blueprint) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            "蓝图正文",
            format!("蓝图 {}", blueprint.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::Blueprint,
        records,
        &mut stats,
        &mut progress,
        "蓝图",
    )
    .await?;
    stats.item_count = stats.item_count.saturating_add(processed);
    Ok(stats)
}

pub(crate) async fn ingest_blueprint_metadata_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let blueprints = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for blueprint in &blueprints {
        let record = match blueprint_metadata_rag_record(project_id, blueprint) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            "蓝图元数据",
            format!("蓝图元数据 {}", blueprint.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::BlueprintMetadata,
        records,
        &mut stats,
        &mut progress,
        "蓝图元数据",
    )
    .await?;
    stats.item_count = stats.item_count.saturating_add(processed);
    Ok(stats)
}

pub(crate) async fn ingest_world_setting_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let blueprints = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for blueprint in &blueprints {
        let world_setting_records = match blueprint_world_setting_rag_records(project_id, blueprint)
        {
            Ok(records) => records,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        for record in world_setting_records {
            let field = match required_record_metadata_str(
                &record,
                "field",
                format!("世界观 {} RAG 分块 {}", blueprint.id, record.chunk_id),
            ) {
                Ok(field) => field,
                Err(err) => {
                    stats.errors.push(err);
                    continue;
                }
            };
            records.push(labeled_rag_record(
                record,
                format!("世界观：{field}"),
                format!("世界观 {}", blueprint.id),
            ));
        }
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::WorldSetting,
        records,
        &mut stats,
        &mut progress,
        "世界观",
    )
    .await?;
    stats.item_count = stats.item_count.saturating_add(processed);
    Ok(stats)
}

pub async fn expected_rag_record_total(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<usize, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    expected_rag_records(state, project_id)
        .await
        .map(|expected| expected.total())
}

pub async fn rag_ingest_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<RagIngestTaskScope, String> {
    let expected_records = expected_rag_record_total(state, user_id, project_id).await?;
    Ok(RagIngestTaskScope {
        project_id,
        expected_records,
        total: u32::try_from(expected_records).unwrap_or(u32::MAX).max(1),
    })
}

pub async fn rag_prepare_ingest_all(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ClientRagIngestPrepareOutput, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    let embedding_identity = crate::actions::rag_search::current_embedding_identity(state)
        .await
        .map_err(|err| err.to_string())?;
    let expected = expected_rag_records(state, project_id).await?;
    let expected_total = expected.total();
    let stored_metadata = project_rag_metadata(state, project_id).await?;
    ensure_project_embedding_identity_matches(&stored_metadata, &embedding_identity)?;

    let mut records = Vec::new();
    let mut skipped_count = 0u32;
    for mut record in expected.records {
        record.metadata = root_text_family_metadata(project_id, record.metadata)?;
        let content_hash = RagService::content_hash(&record.text).map_err(|err| err.to_string())?;
        let outcome = classify_prepared_record(
            stored_metadata.get(&record.chunk_id),
            &content_hash,
            &embedding_identity,
        );
        if outcome == RagRecordSyncOutcome::Skipped {
            skipped_count = skipped_count.saturating_add(1);
            continue;
        }
        records.push(PreparedClientRagIngestRecord {
            chunk_id: record.chunk_id,
            text: record.text,
            content_hash,
            outcome: match outcome {
                RagRecordSyncOutcome::Added => "added",
                RagRecordSyncOutcome::Updated => "updated",
                RagRecordSyncOutcome::Skipped => "skipped",
            }
            .to_string(),
        });
    }

    Ok(ClientRagIngestPrepareOutput {
        project_id,
        embedding_identity,
        records,
        expected_count: expected_total,
        skipped_count,
    })
}

pub async fn rag_upload_ingest_embeddings(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ClientRagIngestUploadInput,
) -> Result<serde_json::Value, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    let embedding_identity = crate::actions::rag_search::current_embedding_identity(state)
        .await
        .map_err(|err| err.to_string())?;
    ensure_embedding_identity_matches(&input.embedding_identity, &embedding_identity)?;

    let expected = expected_rag_records(state, project_id).await?;
    let expected_total = expected.total();
    let counts = expected.counts.clone();
    let expected_ids = expected.ids.iter().cloned().collect::<HashSet<_>>();
    let stored_metadata = project_rag_metadata(state, project_id).await?;
    ensure_project_embedding_identity_matches(&stored_metadata, &embedding_identity)?;

    let mut record_by_id = HashMap::new();
    for mut record in expected.records {
        record.metadata = root_text_family_metadata(project_id, record.metadata)?;
        if record_by_id
            .insert(record.chunk_id.clone(), record)
            .is_some()
        {
            return Err("RAG 上传失败：canonical chunk_id 重复".to_string());
        }
    }

    let mut seen = HashSet::new();
    let mut embedded_inputs = Vec::new();
    let mut stats = IngestStats::default();
    for uploaded in input.embeddings {
        let chunk_id = uploaded.chunk_id.trim().to_string();
        if chunk_id.is_empty() {
            return Err("RAG 上传失败：chunk_id 不能为空".to_string());
        }
        if !seen.insert(chunk_id.clone()) {
            return Err(format!("RAG 上传失败：chunk_id 重复：{chunk_id}"));
        }
        let Some(record) = record_by_id.get(&chunk_id) else {
            return Err(format!(
                "RAG 上传失败：chunk_id 不属于当前项目 prepare 结果：{chunk_id}"
            ));
        };
        let content_hash = RagService::content_hash(&record.text).map_err(|err| err.to_string())?;
        let outcome = classify_prepared_record(
            stored_metadata.get(&chunk_id),
            &content_hash,
            &embedding_identity,
        );
        if outcome == RagRecordSyncOutcome::Skipped {
            stats.record_outcome(outcome);
            continue;
        }
        stats.record_outcome(outcome);
        embedded_inputs.push(RagEmbeddedInput {
            chunk_id,
            text: record.text.clone(),
            metadata: record.metadata.clone(),
            embedding: uploaded.embedding,
        });
    }

    state
        .rag
        .store_embedded_records(project_id, embedded_inputs, embedding_identity.clone())
        .await
        .map_err(|err| err.to_string())?;

    let stale_ids = stored_metadata
        .keys()
        .filter(|id| !expected_ids.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    let deleted_count = if stale_ids.is_empty() {
        0
    } else {
        let deleted = state
            .rag
            .vector_store
            .delete(
                &project_id.to_string(),
                DeleteFilter {
                    project_id,
                    data_type: None,
                    chapter_number: None,
                    ids: stale_ids,
                    id_prefixes: Vec::new(),
                },
            )
            .await
            .map_err(|err| format!("清理过期 RAG 记录失败: {err}"))?;
        u32::try_from(deleted).unwrap_or(u32::MAX)
    };
    stats.deleted_count = deleted_count;
    stats.skipped_count = u32::try_from(expected_total)
        .unwrap_or(u32::MAX)
        .saturating_sub(stats.added_count)
        .saturating_sub(stats.updated_count);

    Ok(serde_json::json!({
        "status": "completed",
        "chapter_count": counts.chapters,
        "foreshadowing_count": counts.foreshadowing,
        "outline_count": counts.outlines,
        "part_outline_count": counts.part_outlines,
        "blueprint_count": counts.blueprints,
        "blueprint_metadata_count": counts.blueprint_metadata,
        "world_setting_count": counts.world_settings,
        "character_count": counts.characters,
        "protagonist_count": counts.protagonists,
        "character_state_count": counts.character_states,
        "character_attribute_event_count": counts.character_attribute_events,
        "relationship_count": counts.relationships,
        "scene_profile_count": counts.scene_profiles,
        "scene_occurrence_count": counts.scene_occurrences,
        "scene_transition_count": counts.scene_transitions,
        "chunk_count": stats.chunk_count,
        "added_count": stats.added_count,
        "updated_count": stats.updated_count,
        "skipped_count": stats.skipped_count,
        "deleted_count": stats.deleted_count,
        "errors": [],
        "success": true,
    }))
}

pub(crate) async fn project_rag_metadata(
    state: &AppState,
    project_id: i64,
) -> Result<HashMap<String, serde_json::Value>, String> {
    state
        .rag
        .vector_store
        .metadata_by_filter(
            &project_id.to_string(),
            DeleteFilter {
                project_id,
                data_type: None,
                chapter_number: None,
                ids: Vec::new(),
                id_prefixes: Vec::new(),
            },
        )
        .await
        .map_err(|err| format!("读取 RAG 已同步记录失败: {err}"))
}

fn classify_prepared_record(
    stored: Option<&serde_json::Value>,
    content_hash: &str,
    embedding_identity: &EmbeddingModelIdentity,
) -> RagRecordSyncOutcome {
    let already_synced = stored.is_some_and(|metadata| {
        RagService::metadata_content_hash(metadata)
            .is_some_and(|stored_hash| stored_hash == content_hash)
            && RagService::metadata_matches_current_text_index(metadata)
            && RagService::metadata_matches_embedding_identity(metadata, embedding_identity)
    });
    if already_synced {
        RagRecordSyncOutcome::Skipped
    } else if stored.is_some() {
        RagRecordSyncOutcome::Updated
    } else {
        RagRecordSyncOutcome::Added
    }
}

pub(crate) fn ensure_project_embedding_identity_matches(
    stored_metadata: &HashMap<String, serde_json::Value>,
    expected: &EmbeddingModelIdentity,
) -> Result<(), String> {
    for metadata in stored_metadata.values() {
        let Some(actual) = metadata_embedding_identity(metadata) else {
            continue;
        };
        if &actual != expected {
            return Err(format!(
                "项目 RAG embedding identity 已锁定为 {} / {} / {} 维，当前本机配置为 {} / {} / {} 维；请切回项目模型或显式重建 RAG",
                actual.backend,
                actual.model,
                actual.dimension,
                expected.backend,
                expected.model,
                expected.dimension
            ));
        }
    }
    Ok(())
}

fn ensure_embedding_identity_matches(
    actual: &EmbeddingModelIdentity,
    expected: &EmbeddingModelIdentity,
) -> Result<(), String> {
    if actual == expected {
        return Ok(());
    }
    Err(format!(
        "Embedding identity 不匹配：服务器期望 {} / {} / {} 维，客户端提交 {} / {} / {} 维",
        expected.backend,
        expected.model,
        expected.dimension,
        actual.backend,
        actual.model,
        actual.dimension
    ))
}

fn metadata_embedding_identity(metadata: &serde_json::Value) -> Option<EmbeddingModelIdentity> {
    Some(EmbeddingModelIdentity {
        backend: metadata.get("embedding_backend")?.as_str()?.to_string(),
        model: metadata.get("embedding_model")?.as_str()?.to_string(),
        dimension: usize::try_from(metadata.get("embedding_dimension")?.as_u64()?).ok()?,
    })
}

pub async fn rag_ingest_all(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<serde_json::Value, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    if let Some(progress) = progress.as_deref_mut() {
        progress.stage("正在检查已同步 RAG 数据")?;
    }
    let mut total_stats = IngestStats::default();

    let chapter_stats = ingest_chapters_inner(state, project_id, progress.as_deref_mut()).await?;
    let chapter_count = chapter_stats.item_count;
    total_stats.merge(chapter_stats);

    let foreshadowing_stats =
        ingest_foreshadowing_inner(state, project_id, progress.as_deref_mut()).await?;
    let foreshadowing_count = foreshadowing_stats.item_count;
    total_stats.merge(foreshadowing_stats);

    let outline_stats = ingest_outlines_inner(state, project_id, progress.as_deref_mut()).await?;
    let outline_count = outline_stats.item_count;
    total_stats.merge(outline_stats);

    let part_stats = ingest_part_outlines_inner(state, project_id, progress.as_deref_mut()).await?;
    let part_outline_count = part_stats.item_count;
    total_stats.merge(part_stats);

    let blueprint_stats =
        ingest_blueprints_inner(state, project_id, progress.as_deref_mut()).await?;
    let blueprint_count = blueprint_stats.item_count;
    total_stats.merge(blueprint_stats);

    let blueprint_metadata_stats =
        ingest_blueprint_metadata_inner(state, project_id, progress.as_deref_mut()).await?;
    let blueprint_metadata_count = blueprint_metadata_stats.item_count;
    total_stats.merge(blueprint_metadata_stats);

    let world_setting_stats =
        ingest_world_setting_inner(state, project_id, progress.as_deref_mut()).await?;
    let world_setting_count = world_setting_stats.item_count;
    total_stats.merge(world_setting_stats);

    let character_stats =
        ingest_character_records_inner(state, project_id, progress.as_deref_mut()).await?;
    let character_count = character_stats.item_count;
    total_stats.merge(character_stats);

    let protagonist_stats =
        ingest_protagonists_inner(state, project_id, progress.as_deref_mut()).await?;
    let protagonist_count = protagonist_stats.item_count;
    total_stats.merge(protagonist_stats);

    let character_state_stats =
        ingest_character_states_inner(state, project_id, progress.as_deref_mut()).await?;
    let character_state_count = character_state_stats.item_count;
    total_stats.merge(character_state_stats);

    let character_attribute_event_stats =
        ingest_character_attribute_events_inner(state, project_id, progress.as_deref_mut()).await?;
    let character_attribute_event_count = character_attribute_event_stats.item_count;
    total_stats.merge(character_attribute_event_stats);

    let relationship_stats =
        ingest_relationships_inner(state, project_id, progress.as_deref_mut()).await?;
    let relationship_count = relationship_stats.item_count;
    total_stats.merge(relationship_stats);

    let scene_profile_stats =
        ingest_scene_profiles_inner(state, project_id, progress.as_deref_mut()).await?;
    let scene_profile_count = scene_profile_stats.item_count;
    total_stats.merge(scene_profile_stats);

    let scene_occurrence_stats =
        ingest_scene_occurrences_inner(state, project_id, progress.as_deref_mut()).await?;
    let scene_occurrence_count = scene_occurrence_stats.item_count;
    total_stats.merge(scene_occurrence_stats);

    let scene_transition_stats =
        ingest_scene_transitions_inner(state, project_id, progress).await?;
    let scene_transition_count = scene_transition_stats.item_count;
    total_stats.merge(scene_transition_stats);

    if !total_stats.errors.is_empty() {
        return Err(rag_ingest_error_message("RAG 同步", &total_stats.errors));
    }

    Ok(serde_json::json!({
        "status": "completed",
        "chapter_count": chapter_count,
        "foreshadowing_count": foreshadowing_count,
        "outline_count": outline_count,
        "part_outline_count": part_outline_count,
        "blueprint_count": blueprint_count,
        "blueprint_metadata_count": blueprint_metadata_count,
        "world_setting_count": world_setting_count,
        "character_count": character_count,
        "protagonist_count": protagonist_count,
        "character_state_count": character_state_count,
        "character_attribute_event_count": character_attribute_event_count,
        "relationship_count": relationship_count,
        "scene_profile_count": scene_profile_count,
        "scene_occurrence_count": scene_occurrence_count,
        "scene_transition_count": scene_transition_count,
        "chunk_count": total_stats.chunk_count,
        "added_count": total_stats.added_count,
        "updated_count": total_stats.updated_count,
        "skipped_count": total_stats.skipped_count,
        "deleted_count": total_stats.deleted_count,
        "errors": [],
        "success": true,
    }))
}

pub async fn sync_scene_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    let mut total_stats = IngestStats::default();

    let scene_profile_stats = ingest_scene_profiles_inner(state, project_id, None).await?;
    let scene_profile_count = scene_profile_stats.item_count;
    total_stats.merge(scene_profile_stats);

    let scene_occurrence_stats = ingest_scene_occurrences_inner(state, project_id, None).await?;
    let scene_occurrence_count = scene_occurrence_stats.item_count;
    total_stats.merge(scene_occurrence_stats);

    let scene_transition_stats = ingest_scene_transitions_inner(state, project_id, None).await?;
    let scene_transition_count = scene_transition_stats.item_count;
    total_stats.merge(scene_transition_stats);

    if !total_stats.errors.is_empty() {
        return Err(scene_rag_sync_error_message(&total_stats.errors));
    }

    Ok(serde_json::json!({
        "scene_profile_count": scene_profile_count,
        "scene_occurrence_count": scene_occurrence_count,
        "scene_transition_count": scene_transition_count,
        "chunk_count": total_stats.chunk_count,
        "added_count": total_stats.added_count,
        "updated_count": total_stats.updated_count,
        "skipped_count": total_stats.skipped_count,
        "deleted_count": total_stats.deleted_count,
    }))
}

fn rag_ingest_error_message(scope: &str, errors: &[String]) -> String {
    let details = errors
        .iter()
        .take(8)
        .cloned()
        .collect::<Vec<_>>()
        .join("；");
    let suffix = if errors.len() > 8 {
        format!("；另有 {} 个错误未显示", errors.len() - 8)
    } else {
        String::new()
    };
    if details.is_empty() {
        format!("{scope}失败")
    } else {
        format!("{scope}失败：{details}{suffix}")
    }
}

fn scene_rag_sync_error_message(errors: &[String]) -> String {
    let details = errors
        .iter()
        .take(8)
        .cloned()
        .collect::<Vec<_>>()
        .join("；");
    let suffix = if errors.len() > 8 {
        format!("；另有 {} 个错误未显示", errors.len() - 8)
    } else {
        String::new()
    };
    format!("场景 RAG 同步失败：{details}{suffix}")
}
