use a4n_chapter_analysis::analysis_rag_items;
use a4n_db_models::models::chapter::chapter;
use a4n_domain::rag::RagDataType;
use a4n_embedding::provider::EmbeddingProvider;
use a4n_rag_core::RagRecord;

use super::chunks::chapter_rag_chunks_with_provider;
use crate::rag::chapter_rag_chunk_prefix;
use a4n_rag_core::ChapterRagChunkingMode;

pub(crate) async fn chapter_rag_records_with_provider(
    chapter: &chapter::Model,
    provider: &dyn EmbeddingProvider,
    mode: ChapterRagChunkingMode,
) -> Result<Vec<RagRecord>, String> {
    if !chapter.has_available_content() {
        return Ok(Vec::new());
    }

    let chunks = chapter_rag_chunks_with_provider(chapter, provider, mode).await?;
    let chunk_prefix = chapter_rag_chunk_prefix(chapter.project_id, chapter.id);
    let mut records = Vec::new();
    for (idx, chunk) in chunks.into_iter().enumerate() {
        records.push(RagRecord {
            chunk_id: format!("{chunk_prefix}p{}", idx + 1),
            text: chunk,
            metadata: serde_json::json!({
                "data_type": RagDataType::Chapter.as_str(),
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
            chunk_id: format!("{chunk_prefix}summary"),
            text: summary.to_string(),
            metadata: serde_json::json!({
                "data_type": RagDataType::ChapterSummary.as_str(),
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
                    chunk_id: format!("{chunk_prefix}{}", item.id_suffix),
                    text: item.text,
                    metadata: serde_json::json!({
                        "data_type": item.data_type.as_str(),
                        "chapter_number": chapter.chapter_number,
                        "title": chapter.title,
                    }),
                }),
        );
    }

    Ok(records)
}
