use std::collections::HashMap;

use crate::OrchestratorContext;

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

        let mut state_rag_indexed_count = 0usize;
        let mut state_rag_index_errors = Vec::new();
        for state_index in &applied.state_indexes {
            match self
                .sync_character_state_rag(project_id, state_index, character_name)
                .await
            {
                Ok(()) => state_rag_indexed_count += 1,
                Err(error) => {
                    tracing::warn!(
                        profile_id,
                        project_id,
                        state_id = state_index.id,
                        chapter_number = state_index.chapter_number,
                        error = %error,
                        "角色档案同步后状态 RAG 入库失败"
                    );
                    state_rag_index_errors.push(format!(
                        "第{}章状态 {}：{}",
                        state_index.chapter_number, state_index.id, error
                    ));
                }
            }
        }
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
                    for event_id in attribute_event_ids {
                        let Some(event) = attribute_events_by_id.get(event_id) else {
                            attribute_event_rag_index_errors
                                .push(format!("属性事件 {event_id}：数据库记录不存在"));
                            continue;
                        };
                        match self
                            .sync_character_attribute_event_rag(project_id, event, character_name)
                            .await
                        {
                            Ok(()) => attribute_event_rag_indexed_count += 1,
                            Err(error) => {
                                tracing::warn!(
                                    profile_id,
                                    project_id,
                                    attribute_event_id = event.id,
                                    chapter_number = event.chapter_number,
                                    error = %error,
                                    "角色档案同步后属性事件 RAG 入库失败"
                                );
                                attribute_event_rag_index_errors.push(format!(
                                    "第{}章属性事件 {}：{}",
                                    event.chapter_number, event.id, error
                                ));
                            }
                        }
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
                    attribute_event_rag_index_errors.push(format!("查询属性事件失败：{error}"));
                }
            }
        }
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
