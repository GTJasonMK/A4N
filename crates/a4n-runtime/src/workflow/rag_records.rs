use crate::state::AppState;
use a4n_core::models::{
    chapter::chapter,
    character::{attribute_event, character_state_index, foreshadowing_index},
};
use a4n_core::rag::RagDataType;
use a4n_engine::embedding::provider::EmbeddingProvider;
use a4n_engine::rag::chunk_strategy::chapter_rag_chunk_config;
use a4n_engine::rag::semantic_chunker::{
    SemanticChunk, split_novel_sentences, split_semantic_dp_text,
};
use a4n_engine::rag::vector_store::DeleteFilter;
use a4n_services::chapter::analysis::analysis_rag_items;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub async fn sync_chapter_rag_records(
    state: &AppState,
    ch: &chapter::Model,
) -> Result<usize, String> {
    let records = chapter_rag_records(ch, state.embedding.as_ref()).await?;
    let mut sync = crate::rag_ingest::RagSyncContext::for_id_prefixes(
        state,
        ch.project_id,
        vec![format!("chapter:{}:{}:", ch.project_id, ch.id)],
    )
    .await?;

    let mut processed = 0usize;
    for record in &records {
        sync.sync_record(state, record)
            .await
            .map_err(|err| format!("同步章节 RAG 失败：{}：{err}", record.chunk_id))?;
        processed += 1;
    }
    sync.cleanup_stale(state)
        .await
        .map_err(|err| format!("清理章节过期 RAG 失败：chapter_id={}：{err}", ch.id))?;
    Ok(processed)
}

pub async fn ingest_character_state_rag(
    state: &AppState,
    project_id: i64,
    state_index: &character_state_index::Model,
    character_name: &str,
) -> Result<(), String> {
    let record =
        crate::rag_ingest::character_state_rag_record(project_id, state_index, character_name)?;
    crate::rag_ingest::embed_rag_record(state, project_id, &record).await
}

pub async fn ingest_character_attribute_event_rag(
    state: &AppState,
    project_id: i64,
    event: &attribute_event::Model,
    character_name: &str,
) -> Result<(), String> {
    let record =
        crate::rag_ingest::character_attribute_event_rag_record(project_id, event, character_name)?;
    crate::rag_ingest::embed_rag_record(state, project_id, &record).await
}

pub async fn ingest_foreshadowing_rag(
    state: &AppState,
    project_id: i64,
    item: &foreshadowing_index::Model,
) -> Result<(), String> {
    let record = crate::rag_ingest::foreshadowing_rag_record(project_id, item)?;
    crate::rag_ingest::embed_rag_record(state, project_id, &record).await
}

pub async fn delete_foreshadowing_rag_by_ids(
    state: &AppState,
    project_id: i64,
    foreshadowing_ids: &[i64],
) -> Result<usize, String> {
    if foreshadowing_ids.is_empty() {
        return Ok(0);
    }
    let chunk_ids = foreshadowing_ids
        .iter()
        .map(|id| format!("foreshadowing:{project_id}:{id}"))
        .collect::<Vec<_>>();
    state
        .rag
        .vector_store
        .delete(
            &project_id.to_string(),
            DeleteFilter {
                project_id,
                data_type: Some(RagDataType::Foreshadowing.as_str().to_string()),
                chapter_number: None,
                ids: chunk_ids,
                id_prefixes: Vec::new(),
            },
        )
        .await
        .map_err(|err| err.to_string())
}

pub async fn reingest_foreshadowing_rag_by_ids(
    state: &AppState,
    project_id: i64,
    foreshadowing_ids: &[i64],
) -> Result<(), String> {
    if foreshadowing_ids.is_empty() {
        return Ok(());
    }
    let items = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::Id.is_in(foreshadowing_ids.to_vec()))
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("查询需更新的伏笔 RAG 失败: {err}"))?;
    for item in &items {
        ingest_foreshadowing_rag(state, project_id, item).await?;
    }
    Ok(())
}

pub async fn delete_character_state_rag_by_ids(
    state: &AppState,
    project_id: i64,
    character_state_ids: &[i64],
) -> Result<usize, String> {
    if character_state_ids.is_empty() {
        return Ok(0);
    }

    state
        .rag
        .vector_store
        .delete(
            &project_id.to_string(),
            DeleteFilter {
                project_id,
                data_type: Some(RagDataType::CharacterState.as_str().to_string()),
                chapter_number: None,
                ids: character_state_chunk_ids(project_id, character_state_ids),
                id_prefixes: Vec::new(),
            },
        )
        .await
        .map_err(|err| err.to_string())
}

fn character_state_chunk_ids(project_id: i64, character_state_ids: &[i64]) -> Vec<String> {
    character_state_ids
        .iter()
        .map(|id| format!("character_state:{project_id}:{id}"))
        .collect()
}

pub async fn delete_character_attribute_event_rag_by_ids(
    state: &AppState,
    project_id: i64,
    attribute_event_ids: &[i64],
) -> Result<usize, String> {
    if attribute_event_ids.is_empty() {
        return Ok(0);
    }

    state
        .rag
        .vector_store
        .delete(
            &project_id.to_string(),
            DeleteFilter {
                project_id,
                data_type: Some(RagDataType::CharacterAttributeEvent.as_str().to_string()),
                chapter_number: None,
                ids: character_attribute_event_chunk_ids(project_id, attribute_event_ids),
                id_prefixes: Vec::new(),
            },
        )
        .await
        .map_err(|err| err.to_string())
}

pub async fn delete_protagonist_rag_by_ids(
    state: &AppState,
    project_id: i64,
    profile_ids: &[i64],
) -> Result<usize, String> {
    if profile_ids.is_empty() {
        return Ok(0);
    }

    state
        .rag
        .vector_store
        .delete(
            &project_id.to_string(),
            DeleteFilter {
                project_id,
                data_type: Some(RagDataType::Protagonist.as_str().to_string()),
                chapter_number: None,
                ids: Vec::new(),
                id_prefixes: protagonist_chunk_prefixes(project_id, profile_ids),
            },
        )
        .await
        .map_err(|err| err.to_string())
}

fn protagonist_chunk_prefixes(project_id: i64, profile_ids: &[i64]) -> Vec<String> {
    profile_ids
        .iter()
        .map(|id| format!("protagonist:{project_id}:{id}:"))
        .collect()
}

fn character_attribute_event_chunk_ids(
    project_id: i64,
    attribute_event_ids: &[i64],
) -> Vec<String> {
    attribute_event_ids
        .iter()
        .map(|id| format!("character_attribute_event:{project_id}:{id}"))
        .collect()
}

#[cfg(test)]
pub(crate) async fn expected_chapter_rag_record_ids(
    ch: &chapter::Model,
    provider: &dyn EmbeddingProvider,
) -> Result<Vec<String>, String> {
    Ok(chapter_rag_records(ch, provider)
        .await?
        .into_iter()
        .map(|record| record.chunk_id)
        .collect())
}

pub(crate) async fn chapter_rag_records(
    ch: &chapter::Model,
    provider: &dyn EmbeddingProvider,
) -> Result<Vec<crate::rag_ingest::RagRecord>, String> {
    if !ch.has_available_content() {
        return Ok(Vec::new());
    }

    let chunks = chapter_rag_chunks(ch, provider).await?;
    let chunk_count = chunks.len();
    let semantic_chunked = !crate::runtime_config::uses_client_desktop_embedding();
    let mut records = Vec::new();
    for (idx, chunk) in chunks.into_iter().enumerate() {
        records.push(crate::rag_ingest::RagRecord {
            chunk_id: format!("chapter:{}:{}:p{}", ch.project_id, ch.id, idx + 1),
            text: chunk.content,
            metadata: serde_json::json!({
                "data_type": RagDataType::Chapter.as_str(),
                "content_kind": "paragraph",
                "chunk_index": idx + 1,
                "chunk_count": chunk_count,
                "section_index": idx,
                "total_sections": chunk_count,
                "semantic_chunked": semantic_chunked,
                "sentence_count": chunk.sentence_count,
                "density_score": chunk.density_score,
                "start_sentence_idx": chunk.start_sentence_idx,
                "end_sentence_idx": chunk.end_sentence_idx,
                "chapter_id": ch.id,
                "chapter_number": ch.chapter_number,
                "title": ch.title,
            }),
        });
    }

    if let Some(summary) = ch
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        records.push(crate::rag_ingest::RagRecord {
            chunk_id: format!("chapter:{}:{}:summary", ch.project_id, ch.id),
            text: summary.to_string(),
            metadata: serde_json::json!({
                "data_type": RagDataType::ChapterSummary.as_str(),
                "content_kind": "summary",
                "chapter_id": ch.id,
                "chapter_number": ch.chapter_number,
                "title": ch.title,
            }),
        });
    }

    if let Some(analysis) = &ch.analysis_data {
        records.extend(analysis_rag_items(analysis).into_iter().map(|item| {
            crate::rag_ingest::RagRecord {
                chunk_id: format!("chapter:{}:{}:{}", ch.project_id, ch.id, item.id_suffix),
                text: item.text,
                metadata: serde_json::json!({
                    "data_type": item.data_type,
                    "content_kind": item.content_kind,
                    "chapter_id": ch.id,
                    "chapter_number": ch.chapter_number,
                    "title": ch.title,
                    "analysis": item.metadata,
                }),
            }
        }));
    }

    Ok(records)
}

pub(crate) fn chapter_rag_record_label(
    ch: &chapter::Model,
    record: &crate::rag_ingest::RagRecord,
) -> String {
    match record
        .metadata
        .get("content_kind")
        .and_then(serde_json::Value::as_str)
    {
        Some("paragraph") => {
            let index = record
                .metadata
                .get("chunk_index")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            let count = record
                .metadata
                .get("chunk_count")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0);
            format!("第{}章正文分块 {index}/{count}", ch.chapter_number)
        }
        Some("summary") => format!("第{}章摘要", ch.chapter_number),
        Some(kind) => format!("第{}章分析：{kind}", ch.chapter_number),
        None => format!("第{}章 RAG 记录", ch.chapter_number),
    }
}

pub(crate) async fn chapter_rag_chunks(
    ch: &chapter::Model,
    provider: &dyn EmbeddingProvider,
) -> Result<Vec<SemanticChunk>, String> {
    if crate::runtime_config::uses_client_desktop_embedding() {
        return Ok(split_client_desktop_chapter_chunks(&ch.content));
    }
    split_semantic_dp_text(&ch.content, &chapter_rag_chunk_config(), provider)
        .await
        .map_err(|err| format!("章节 RAG 分块失败：chapter_id={}：{err}", ch.id))
}

fn split_client_desktop_chapter_chunks(text: &str) -> Vec<SemanticChunk> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let config = chapter_rag_chunk_config();
    let sentences = split_novel_sentences(text);
    if sentences.is_empty() {
        return vec![SemanticChunk {
            content: text.to_string(),
            start_sentence_idx: 0,
            end_sentence_idx: 0,
            sentence_count: 0,
            density_score: 1.0,
        }];
    }

    let mut chunks = Vec::new();
    let mut start_idx = 0usize;
    let mut current = String::new();
    for (idx, sentence) in sentences.iter().enumerate() {
        let next_len = current.chars().count() + sentence.chars().count();
        if !current.is_empty()
            && next_len > config.max_chunk_length
            && current.chars().count() >= config.min_chunk_length
        {
            chunks.push(SemanticChunk {
                content: current.clone(),
                start_sentence_idx: start_idx,
                end_sentence_idx: idx,
                sentence_count: idx.saturating_sub(start_idx),
                density_score: 1.0,
            });
            current.clear();
            start_idx = idx;
        }
        current.push_str(sentence);
    }
    if !current.trim().is_empty() {
        let end_idx = sentences.len();
        chunks.push(SemanticChunk {
            content: current,
            start_sentence_idx: start_idx,
            end_sentence_idx: end_idx,
            sentence_count: end_idx.saturating_sub(start_idx),
            density_score: 1.0,
        });
    }
    chunks
}
