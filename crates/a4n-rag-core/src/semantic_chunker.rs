mod dp;
mod matrix;
mod sentences;

#[cfg(test)]
mod tests;

use a4n_domain::ProviderError as AppError;

use a4n_embedding::provider::EmbeddingProvider;

use super::{
    chunk_strategy::{CHAPTER_RAG_MAX_CHUNK_CHARS, CHAPTER_RAG_MIN_CHUNK_CHARS},
    validation::validate_embedding_batch,
};
use dp::{backtrack_cut_points, dynamic_programming, generate_chunks};
use matrix::{apply_structure_enhancement, build_similarity_matrix, compute_prefix_sum};
pub use sentences::split_novel_sentences;

struct SemanticChunkConfig {
    gate_threshold: f64,
    alpha: f64,
    gamma: f64,
    min_chunk_sentences: usize,
    max_chunk_sentences: usize,
    min_chunk_chars: usize,
    max_chunk_chars: usize,
}

impl SemanticChunkConfig {
    fn chapter() -> Self {
        Self {
            gate_threshold: 0.3,
            alpha: 0.1,
            gamma: 1.1,
            min_chunk_sentences: 3,
            max_chunk_sentences: 15,
            min_chunk_chars: CHAPTER_RAG_MIN_CHUNK_CHARS,
            max_chunk_chars: CHAPTER_RAG_MAX_CHUNK_CHARS,
        }
    }
}

pub async fn split_semantic_dp_text(
    text: &str,
    provider: &dyn EmbeddingProvider,
) -> Result<Vec<String>, AppError> {
    let config = SemanticChunkConfig::chapter();
    chunk_text(text, provider, &config).await
}

async fn chunk_text(
    text: &str,
    provider: &dyn EmbeddingProvider,
    config: &SemanticChunkConfig,
) -> Result<Vec<String>, AppError> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(Vec::new());
    }

    let sentences = split_novel_sentences(text);
    if sentences.len() <= config.min_chunk_sentences {
        return Ok(vec![text.to_string()]);
    }

    let (embeddings, embedding_identity) = provider
        .embed_with_identity(sentences.clone())
        .await
        .map_err(|err| AppError::Embedding(err.to_string()))?;
    validate_embedding_batch(&embeddings, sentences.len(), embedding_identity.dimension)?;

    chunk_sentences_with_embeddings(sentences, embeddings, config)
}

fn chunk_sentences_with_embeddings(
    sentences: Vec<String>,
    embeddings: Vec<Vec<f32>>,
    config: &SemanticChunkConfig,
) -> Result<Vec<String>, AppError> {
    let sim_matrix = build_similarity_matrix(&embeddings);
    let enhanced_matrix = apply_structure_enhancement(
        sim_matrix,
        sentences.len(),
        config.gate_threshold,
        config.alpha,
    );
    let prefix = compute_prefix_sum(&enhanced_matrix, sentences.len());
    let path = dynamic_programming(&prefix, &sentences, config)?;
    let cut_points = backtrack_cut_points(sentences.len(), &path);
    Ok(generate_chunks(&sentences, &cut_points))
}
