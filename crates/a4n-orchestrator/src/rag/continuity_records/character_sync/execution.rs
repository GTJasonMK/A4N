use a4n_rag_core::{PreparedRagRecordSync, RagRecord, RagRecordSyncOutcome};
use std::collections::HashMap;

use crate::{
    OrchestratorContext,
    rag::{RagSyncContext, character_attribute_event_rag_record, character_state_rag_record},
};

use super::contracts::{CharacterSyncRagInput, CharacterSyncRagSummary};

pub(super) fn merged_rag_error(
    delete_error: Option<String>,
    ingest_errors: &[String],
) -> Option<String> {
    if ingest_errors.is_empty() {
        return delete_error;
    }
    let ingest_error = ingest_errors.join("；");
    Some(match delete_error {
        Some(delete_error) => format!("清理失败：{delete_error}；入库失败：{ingest_error}"),
        None => format!("入库失败：{ingest_error}"),
    })
}

pub(super) fn all_records_indexed(indexed_count: usize, expected_count: usize) -> bool {
    indexed_count == expected_count
}

async fn sync_character_rag_records(
    context: &OrchestratorContext,
    project_id: i64,
    records: Vec<(usize, String, RagRecord)>,
) -> (usize, Vec<(usize, String)>) {
    if records.is_empty() {
        return (0, Vec::new());
    }
    let ids = records
        .iter()
        .map(|(_, _, record)| record.chunk_id.clone())
        .collect::<Vec<_>>();
    let mut sync = match RagSyncContext::for_ids(context, project_id, &ids).await {
        Ok(sync) => sync,
        Err(error) => {
            let error = error.to_string();
            return (
                0,
                records
                    .into_iter()
                    .map(|(index, label, _)| (index, format!("{label}：{error}")))
                    .collect(),
            );
        }
    };

    let mut indexed_count = 0usize;
    let mut errors = Vec::new();
    let mut pending = Vec::<(usize, String, PreparedRagRecordSync)>::new();
    for (index, label, record) in records {
        match sync.prepare_record(record) {
            Ok(prepared) if prepared.outcome() == RagRecordSyncOutcome::Skipped => {
                indexed_count += 1;
            }
            Ok(prepared) => pending.push((index, label, prepared)),
            Err(error) => errors.push((index, format!("{label}：{error}"))),
        }
    }

    let batch_size = context.embedding_batch_size();
    if batch_size == 0 {
        let error = "RAG embedding provider batch_size 必须大于 0";
        errors.extend(
            pending
                .into_iter()
                .map(|(index, label, _)| (index, format!("{label}：{error}"))),
        );
        errors.sort_by_key(|(index, _)| *index);
        return (indexed_count, errors);
    }

    let mut pending = pending.into_iter();
    loop {
        let batch = pending.by_ref().take(batch_size).collect::<Vec<_>>();
        if batch.is_empty() {
            break;
        }
        let labels = batch
            .iter()
            .map(|(index, label, _)| (*index, label.clone()))
            .collect::<Vec<_>>();
        let prepared = batch
            .into_iter()
            .map(|(_, _, prepared)| prepared)
            .collect::<Vec<_>>();
        match context
            .store_prepared_rag_records(project_id, prepared)
            .await
        {
            Ok(()) => indexed_count += labels.len(),
            Err(error) => {
                let error = error.to_string();
                errors.extend(
                    labels
                        .into_iter()
                        .map(|(index, label)| (index, format!("{label}：{error}"))),
                );
            }
        }
    }
    errors.sort_by_key(|(index, _)| *index);
    (indexed_count, errors)
}

impl OrchestratorContext {
    pub(crate) async fn sync_character_continuity_rag(
        &self,
        input: CharacterSyncRagInput<'_>,
    ) -> CharacterSyncRagSummary {
        let CharacterSyncRagInput {
            project_id,
            profile_id,
            character_name,
            start_chapter,
            clear_to_chapter,
            applied,
        } = input;
        let stale_state_index_ids = &applied.deleted_timeline.state_index_ids;
        let stale_attribute_event_ids = &applied.deleted_timeline.attribute_event_ids;
        let (deleted_state_rag, state_delete_error) = match self
            .delete_character_state_rag_by_ids(project_id, stale_state_index_ids)
            .await
            .map_err(|error| error.to_string())
        {
            Ok(deleted) => (deleted, None),
            Err(error) => {
                tracing::warn!(
                    profile_id,
                    project_id,
                    start_chapter,
                    clear_to_chapter,
                    state_ids = ?stale_state_index_ids,
                    error = %error,
                    "角色档案重同步后清理状态 RAG 失败"
                );
                (0, Some(error))
            }
        };
        let (deleted_attribute_event_rag, attribute_event_delete_error) = match self
            .delete_character_attribute_event_rag_by_ids(project_id, stale_attribute_event_ids)
            .await
            .map_err(|error| error.to_string())
        {
            Ok(deleted) => (deleted, None),
            Err(error) => {
                tracing::warn!(
                    profile_id,
                    project_id,
                    start_chapter,
                    clear_to_chapter,
                    attribute_event_ids = ?stale_attribute_event_ids,
                    error = %error,
                    "角色档案重同步后清理属性事件 RAG 失败"
                );
                (0, Some(error))
            }
        };

        let mut state_records = Vec::new();
        let mut state_rag_index_errors = Vec::new();
        for (index, state_index) in applied.state_indexes.iter().enumerate() {
            match character_state_rag_record(project_id, state_index, character_name) {
                Ok(record) => state_records.push((
                    index,
                    format!("第{}章状态 {}", state_index.chapter_number, state_index.id),
                    record,
                )),
                Err(error) => {
                    tracing::warn!(
                        profile_id,
                        project_id,
                        state_id = state_index.id,
                        chapter_number = state_index.chapter_number,
                        error = %error,
                        "角色档案同步后状态 RAG 入库失败"
                    );
                    state_rag_index_errors.push((
                        index,
                        format!(
                            "第{}章状态 {}：{}",
                            state_index.chapter_number, state_index.id, error
                        ),
                    ));
                }
            }
        }
        let (state_rag_indexed_count, batch_errors) =
            sync_character_rag_records(self, project_id, state_records).await;
        state_rag_index_errors.extend(batch_errors);
        state_rag_index_errors.sort_by_key(|(index, _)| *index);
        for (_, error) in &state_rag_index_errors {
            tracing::warn!(
                profile_id,
                project_id,
                error = %error,
                "角色档案同步后状态 RAG 入库失败"
            );
        }
        let state_rag_index_errors = state_rag_index_errors
            .into_iter()
            .map(|(_, error)| error)
            .collect::<Vec<_>>();
        let state_rag_error = merged_rag_error(state_delete_error, &state_rag_index_errors);
        let state_rag_indexed =
            all_records_indexed(state_rag_indexed_count, applied.state_indexes.len());

        let attribute_event_ids = &applied.write_summary.attribute_event_ids;
        let mut attribute_event_rag_indexed_count = 0usize;
        let mut attribute_event_rag_index_errors = Vec::new();
        if !attribute_event_ids.is_empty() {
            match self.attribute_events_by_ids(attribute_event_ids).await {
                Ok(attribute_events) => {
                    let attribute_events_by_id = attribute_events
                        .into_iter()
                        .map(|event| (event.id, event))
                        .collect::<HashMap<_, _>>();
                    let mut records = Vec::new();
                    for (index, event_id) in attribute_event_ids.iter().enumerate() {
                        let Some(event) = attribute_events_by_id.get(event_id) else {
                            attribute_event_rag_index_errors
                                .push((index, format!("属性事件 {event_id}：数据库记录不存在")));
                            continue;
                        };
                        match character_attribute_event_rag_record(
                            project_id,
                            event,
                            character_name,
                        ) {
                            Ok(record) => records.push((
                                index,
                                format!("第{}章属性事件 {}", event.chapter_number, event.id),
                                record,
                            )),
                            Err(error) => {
                                tracing::warn!(
                                    profile_id,
                                    project_id,
                                    attribute_event_id = event.id,
                                    chapter_number = event.chapter_number,
                                    error = %error,
                                    "角色档案同步后属性事件 RAG 入库失败"
                                );
                                attribute_event_rag_index_errors.push((
                                    index,
                                    format!(
                                        "第{}章属性事件 {}：{}",
                                        event.chapter_number, event.id, error
                                    ),
                                ));
                            }
                        }
                    }
                    let (indexed_count, errors) =
                        sync_character_rag_records(self, project_id, records).await;
                    attribute_event_rag_indexed_count += indexed_count;
                    attribute_event_rag_index_errors.extend(errors);
                    attribute_event_rag_index_errors.sort_by_key(|(index, _)| *index);
                    for (_, error) in &attribute_event_rag_index_errors {
                        tracing::warn!(
                            profile_id,
                            project_id,
                            error = %error,
                            "角色档案同步后属性事件 RAG 入库失败"
                        );
                    }
                }
                Err(error) => {
                    let error = error.to_string();
                    tracing::warn!(
                        profile_id,
                        project_id,
                        attribute_event_ids = ?attribute_event_ids,
                        error = %error,
                        "角色档案同步后查询属性事件失败"
                    );
                    attribute_event_rag_index_errors
                        .push((0, format!("查询属性事件失败：{error}")));
                }
            }
        }
        attribute_event_rag_index_errors.sort_by_key(|(index, _)| *index);
        let attribute_event_rag_index_errors = attribute_event_rag_index_errors
            .into_iter()
            .map(|(_, error)| error)
            .collect::<Vec<_>>();
        let attribute_event_rag_error = merged_rag_error(
            attribute_event_delete_error,
            &attribute_event_rag_index_errors,
        );
        let attribute_event_rag_indexed =
            all_records_indexed(attribute_event_rag_indexed_count, attribute_event_ids.len());

        CharacterSyncRagSummary {
            deleted_state_rag,
            state_rag_indexed_count,
            state_rag_indexed,
            state_rag_error,
            deleted_attribute_event_rag,
            attribute_event_rag_indexed_count,
            attribute_event_rag_indexed,
            attribute_event_rag_error,
        }
    }
}
