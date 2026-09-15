use a4n_db_models::models::chapter::chapter;
use a4n_embedding_provider_contract::EmbeddingProvider;
use a4n_rag_core::{
    chunk_strategy::{CHAPTER_RAG_MAX_CHUNK_CHARS, CHAPTER_RAG_MIN_CHUNK_CHARS},
    semantic_chunker::{split_novel_sentences, split_semantic_dp_text},
};

use a4n_rag_core::ChapterRagChunkingMode;

pub(super) async fn chapter_rag_chunks_with_provider(
    chapter: &chapter::Model,
    provider: &dyn EmbeddingProvider,
    mode: ChapterRagChunkingMode,
) -> Result<Vec<String>, String> {
    if mode == ChapterRagChunkingMode::ClientDesktop {
        return Ok(split_client_desktop_chapter_chunks(&chapter.content));
    }
    split_semantic_dp_text(&chapter.content, provider)
        .await
        .map_err(|error| format!("章节 RAG 分块失败：chapter_id={}：{error}", chapter.id))
}

fn split_client_desktop_chapter_chunks(text: &str) -> Vec<String> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let sentences = split_novel_sentences(text);
    if sentences.is_empty() {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;
    for sentence in &sentences {
        let sentence_len = sentence.chars().count();
        let next_len = current_len + sentence_len;
        if current_len > 0
            && next_len > CHAPTER_RAG_MAX_CHUNK_CHARS
            && current_len >= CHAPTER_RAG_MIN_CHUNK_CHARS
        {
            chunks.push(std::mem::take(&mut current));
            current_len = 0;
        }
        current.push_str(sentence);
        current_len += sentence_len;
    }
    if !current.trim().is_empty() {
        chunks.push(current);
    }
    chunks
}
