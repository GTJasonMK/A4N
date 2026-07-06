mod dp;
mod matrix;
mod sentences;

#[cfg(test)]
mod tests;

use a4n_core::error::AppError;

use crate::embedding::provider::EmbeddingProvider;

use super::chunk_strategy::RagChunkConfig;
use dp::{backtrack_cut_points, dynamic_programming, generate_chunks};
use matrix::{apply_structure_enhancement, build_similarity_matrix, compute_prefix_sum};
pub use sentences::split_novel_sentences;

#[derive(Debug, Clone)]
struct SemanticChunkConfig {
    gate_threshold: f64,
    alpha: f64,
    gamma: f64,
    min_chunk_sentences: usize,
    max_chunk_sentences: usize,
    min_chunk_chars: usize,
    max_chunk_chars: usize,
    with_overlap: bool,
    overlap_sentences: usize,
}

impl SemanticChunkConfig {
    fn from_rag_config(config: &RagChunkConfig) -> Self {
        Self {
            gate_threshold: f64::from(config.semantic_gate_threshold),
            alpha: f64::from(config.semantic_alpha),
            gamma: f64::from(config.semantic_gamma),
            min_chunk_sentences: config.semantic_min_sentences.max(1),
            max_chunk_sentences: config
                .semantic_max_sentences
                .max(config.semantic_min_sentences.max(1)),
            min_chunk_chars: config.min_chunk_length,
            max_chunk_chars: config.max_chunk_length.max(1),
            with_overlap: false,
            overlap_sentences: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SemanticChunk {
    pub content: String,
    pub start_sentence_idx: usize,
    pub end_sentence_idx: usize,
    pub sentence_count: usize,
    pub density_score: f64,
}

pub async fn split_semantic_dp_text(
    text: &str,
    rag_config: &RagChunkConfig,
    provider: &dyn EmbeddingProvider,
) -> Result<Vec<SemanticChunk>, AppError> {
    let config = SemanticChunkConfig::from_rag_config(rag_config);
    let chunker = SemanticChunker::new(config);
    chunker.chunk_text(text, provider).await
}

#[derive(Debug, Clone)]
struct SemanticChunker {
    config: SemanticChunkConfig,
}

impl SemanticChunker {
    fn new(config: SemanticChunkConfig) -> Self {
        Self { config }
    }

    async fn chunk_text(
        &self,
        text: &str,
        provider: &dyn EmbeddingProvider,
    ) -> Result<Vec<SemanticChunk>, AppError> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(Vec::new());
        }

        let sentences = split_novel_sentences(text);
        if sentences.len() <= self.config.min_chunk_sentences {
            return Ok(vec![whole_semantic_chunk(
                text.to_string(),
                sentences.len(),
            )]);
        }
        if sentences.len() < 2 {
            return Ok(vec![whole_semantic_chunk(
                text.to_string(),
                sentences.len(),
            )]);
        }

        let (embeddings, embedding_identity) = provider
            .embed_with_identity(sentences.clone())
            .await
            .map_err(|err| AppError::EmbeddingError(err.to_string()))?;
        validate_embeddings(&embeddings, sentences.len(), embedding_identity.dimension)?;

        Ok(self.chunk_sentences_with_embeddings(sentences, embeddings))
    }

    fn chunk_sentences_with_embeddings(
        &self,
        sentences: Vec<String>,
        embeddings: Vec<Vec<f32>>,
    ) -> Vec<SemanticChunk> {
        if sentences.is_empty() {
            return Vec::new();
        }
        if sentences.len() <= self.config.min_chunk_sentences {
            return vec![whole_semantic_chunk(sentences.join(""), sentences.len())];
        }

        let sim_matrix = build_similarity_matrix(&embeddings);
        let enhanced_matrix = apply_structure_enhancement(
            &sim_matrix,
            sentences.len(),
            self.config.gate_threshold,
            self.config.alpha,
        );
        let prefix = compute_prefix_sum(&enhanced_matrix, sentences.len());
        let path = dynamic_programming(&prefix, &sentences, &self.config);
        let cut_points = backtrack_cut_points(sentences.len(), &path);
        generate_chunks(&sentences, &cut_points, &prefix, &self.config)
    }
}

fn validate_embeddings(
    embeddings: &[Vec<f32>],
    expected: usize,
    expected_dimension: usize,
) -> Result<(), AppError> {
    if embeddings.len() != expected {
        return Err(AppError::EmbeddingError(format!(
            "语义分块句子嵌入数量不匹配：expected={expected}, actual={}",
            embeddings.len()
        )));
    }
    if expected_dimension == 0 {
        return Err(AppError::EmbeddingError(
            "语义分块句子嵌入期望维度不能为 0".to_string(),
        ));
    }
    for (index, embedding) in embeddings.iter().enumerate() {
        if embedding.len() != expected_dimension {
            return Err(AppError::EmbeddingError(format!(
                "语义分块句子嵌入维度不匹配：index={index}, expected={expected_dimension}, actual={}",
                embedding.len()
            )));
        }
        if embedding.iter().any(|value| !value.is_finite()) {
            return Err(AppError::EmbeddingError(format!(
                "语义分块句子嵌入包含非有限数：index={index}"
            )));
        }
    }
    Ok(())
}

fn whole_semantic_chunk(content: String, sentence_count: usize) -> SemanticChunk {
    SemanticChunk {
        content,
        start_sentence_idx: 0,
        end_sentence_idx: sentence_count,
        sentence_count,
        density_score: 1.0,
    }
}
