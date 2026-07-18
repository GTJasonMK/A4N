use a4n_core::models::chapter::chapter;
use a4n_embedding::provider::EmbeddingProvider;
use a4n_rag_core::{
    chunk_strategy::chapter_rag_chunk_config,
    semantic_chunker::{SemanticChunk, split_novel_sentences, split_semantic_dp_text},
};

use super::contracts::ChapterRagChunkingMode;

pub(super) async fn chapter_rag_chunks_with_provider(
    chapter: &chapter::Model,
    provider: &dyn EmbeddingProvider,
    mode: ChapterRagChunkingMode,
) -> Result<Vec<SemanticChunk>, String> {
    if mode == ChapterRagChunkingMode::ClientDesktop {
        return Ok(split_client_desktop_chapter_chunks(&chapter.content));
    }
    split_semantic_dp_text(&chapter.content, &chapter_rag_chunk_config(), provider)
        .await
        .map_err(|error| format!("章节 RAG 分块失败：chapter_id={}：{error}", chapter.id))
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
                sentence_count: idx - start_idx,
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
            sentence_count: end_idx - start_idx,
            density_score: 1.0,
        });
    }
    chunks
}
