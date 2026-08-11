use super::super::contracts::ClientRagIngestUploadInput;
use super::super::sync::{ExpectedSyncStats as IngestStats, RagRecordSyncOutcome};
use super::context::{ClientIngestContext, load_client_ingest_context};
use crate::state::AppState;
use a4n_orchestrator::rag::{
    DeleteRagRecordsInput, RagEmbeddedInput, classify_rag_record, root_text_family_metadata,
};
use a4n_rag_core::metadata::content_hash;
use std::collections::{HashMap, HashSet};

pub async fn rag_upload_ingest_embeddings(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ClientRagIngestUploadInput,
) -> Result<serde_json::Value, String> {
    let trace_id = crate::sidecar_clients::validate_rag_client_ingest_operation_trace_id(
        project_id,
        &input.operation_trace_id,
    )?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id,
        rag_upload_ingest_embeddings_inner(state, user_id, project_id, input),
    )
    .await
}

async fn rag_upload_ingest_embeddings_inner(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: ClientRagIngestUploadInput,
) -> Result<serde_json::Value, String> {
    let ClientIngestContext {
        embedding_identity,
        expected,
        stored_metadata,
    } = load_client_ingest_context(
        state,
        user_id,
        project_id,
        false,
        Some(&input.embedding_identity),
    )
    .await?;
    let expected_total = expected.total();
    let counts = expected.counts.clone();
    let expected_ids = expected.ids.iter().cloned().collect::<HashSet<_>>();

    let mut record_by_id = HashMap::new();
    for mut record in expected.records {
        record.metadata = root_text_family_metadata(project_id, record.metadata)
            .map_err(|error| error.to_string())?;
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
        let content_hash = content_hash(&record.text).map_err(|error| error.to_string())?;
        let outcome = classify_rag_record(
            stored_metadata.get(&chunk_id),
            &content_hash,
            &embedding_identity,
        );
        if outcome == RagRecordSyncOutcome::Skipped {
            stats.record_outcome(outcome)?;
            continue;
        }
        stats.record_outcome(outcome)?;
        embedded_inputs.push(RagEmbeddedInput {
            chunk_id,
            text: record.text.clone(),
            metadata: record.metadata.clone(),
            embedding: uploaded.embedding,
        });
    }

    state
        .orchestrator()
        .store_embedded_rag_records(project_id, embedded_inputs, embedding_identity.clone())
        .await
        .map_err(|error| error.to_string())?;

    let stale_ids = stored_metadata
        .keys()
        .filter(|id| !expected_ids.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    let deleted_count = if stale_ids.is_empty() {
        0
    } else {
        let deleted = state
            .orchestrator()
            .delete_rag_records(DeleteRagRecordsInput {
                project_id,
                data_type: None,
                ids: stale_ids,
                id_prefixes: Vec::new(),
            })
            .await
            .map_err(|error| format!("清理过期 RAG 记录失败: {error}"))?;
        u32::try_from(deleted)
            .map_err(|_| format!("清理过期 RAG 记录数量超过 u32 上限：{deleted}"))?
    };
    stats.deleted_count = deleted_count;
    let expected_total = u32::try_from(expected_total)
        .map_err(|_| format!("RAG 预期记录数超过 u32 上限：{expected_total}"))?;
    let changed_count = stats
        .added_count
        .checked_add(stats.updated_count)
        .ok_or_else(|| "RAG 新增与更新记录总数超过 u32 上限".to_string())?;
    stats.skipped_count = expected_total.checked_sub(changed_count).ok_or_else(|| {
        format!("RAG 统计不一致：预期记录数 {expected_total} 小于新增与更新记录数 {changed_count}")
    })?;

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
