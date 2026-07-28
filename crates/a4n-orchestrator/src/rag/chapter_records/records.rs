use a4n_core::{chapter_analysis::analysis_rag_items, models::chapter::chapter, rag::RagDataType};
use a4n_embedding::provider::EmbeddingProvider;

use super::{chunks::chapter_rag_chunks_with_provider, contracts::ChapterRagChunkingMode};
use crate::rag::RagRecord;

#[cfg(test)]
pub async fn expected_chapter_rag_record_ids_with_provider(
    chapter: &chapter::Model,
    provider: &dyn EmbeddingProvider,
    mode: ChapterRagChunkingMode,
) -> Result<Vec<String>, String> {
    Ok(chapter_rag_records_with_provider(chapter, provider, mode)
        .await?
        .into_iter()
        .map(|record| record.chunk_id)
        .collect())
}

pub async fn chapter_rag_records_with_provider(
    chapter: &chapter::Model,
    provider: &dyn EmbeddingProvider,
    mode: ChapterRagChunkingMode,
) -> Result<Vec<RagRecord>, String> {
    if !chapter.has_available_content() {
        return Ok(Vec::new());
    }

    let chunks = chapter_rag_chunks_with_provider(chapter, provider, mode).await?;
    let chunk_count = chunks.len();
    let mut records = Vec::new();
    for (idx, chunk) in chunks.into_iter().enumerate() {
        records.push(RagRecord {
            chunk_id: format!("chapter:{}:{}:p{}", chapter.project_id, chapter.id, idx + 1),
            text: chunk.content,
            metadata: serde_json::json!({
                "data_type": RagDataType::Chapter.as_str(),
                "content_kind": "paragraph",
                "chunk_index": idx + 1,
                "chunk_count": chunk_count,
                "section_index": idx,
                "total_sections": chunk_count,
                "semantic_chunked": mode.is_semantic(),
                "sentence_count": chunk.sentence_count,
                "density_score": chunk.density_score,
                "start_sentence_idx": chunk.start_sentence_idx,
                "end_sentence_idx": chunk.end_sentence_idx,
                "chapter_id": chapter.id,
                "chapter_number": chapter.chapter_number,
                "title": chapter.title,
            }),
        });
    }

    if let Some(summary) = chapter
        .summary
        .as_deref()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
    {
        records.push(RagRecord {
            chunk_id: format!("chapter:{}:{}:summary", chapter.project_id, chapter.id),
            text: summary.to_string(),
            metadata: serde_json::json!({
                "data_type": RagDataType::ChapterSummary.as_str(),
                "content_kind": "summary",
                "chapter_id": chapter.id,
                "chapter_number": chapter.chapter_number,
                "title": chapter.title,
            }),
        });
    }

    if let Some(analysis) = &chapter.analysis_data {
        records.extend(
            analysis_rag_items(analysis)
                .into_iter()
                .map(|item| RagRecord {
                    chunk_id: format!(
                        "chapter:{}:{}:{}",
                        chapter.project_id, chapter.id, item.id_suffix
                    ),
                    text: item.text,
                    metadata: serde_json::json!({
                        "data_type": item.data_type,
                        "content_kind": item.content_kind,
                        "chapter_id": chapter.id,
                        "chapter_number": chapter.chapter_number,
                        "title": chapter.title,
                        "analysis": item.metadata,
                    }),
                }),
        );
    }

    Ok(records)
}

pub fn chapter_rag_record_label(chapter: &chapter::Model, record: &RagRecord) -> String {
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
            format!("第{}章正文分块 {index}/{count}", chapter.chapter_number)
        }
        Some("summary") => format!("第{}章摘要", chapter.chapter_number),
        Some(kind) => format!("第{}章分析：{kind}", chapter.chapter_number),
        None => format!("第{}章 RAG 记录", chapter.chapter_number),
    }
}
