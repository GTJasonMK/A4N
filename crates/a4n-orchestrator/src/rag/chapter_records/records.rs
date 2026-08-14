use a4n_core::{
    chapter_analysis::analysis_rag_items, models::chapter::chapter, rag::RagDataType,
    source_spans::source_text_hash,
};
use a4n_embedding::provider::EmbeddingProvider;

use super::{chunks::chapter_rag_chunks_with_provider, contracts::ChapterRagChunkingMode};
use crate::rag::RagRecord;

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

    let source_content_sha256 = source_text_hash(&chapter.content);
    let source_updated_at = chapter.updated_at.to_string();
    for record in &mut records {
        let metadata = record
            .metadata
            .as_object_mut()
            .expect("chapter RAG metadata is constructed as an object");
        metadata.insert(
            "source_chapter_content_sha256".to_string(),
            source_content_sha256.clone().into(),
        );
        metadata.insert(
            "source_chapter_updated_at".to_string(),
            source_updated_at.clone().into(),
        );
    }

    Ok(records)
}
