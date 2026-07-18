#[derive(Debug, Clone, PartialEq)]
pub struct RagChunkConfig {
    pub min_chunk_length: usize,
    pub max_chunk_length: usize,
    pub semantic_gate_threshold: f32,
    pub semantic_alpha: f32,
    pub semantic_gamma: f32,
    pub semantic_min_sentences: usize,
    pub semantic_max_sentences: usize,
}

impl Default for RagChunkConfig {
    fn default() -> Self {
        Self {
            min_chunk_length: 100,
            max_chunk_length: 1200,
            semantic_gate_threshold: 0.3,
            semantic_alpha: 0.1,
            semantic_gamma: 1.1,
            semantic_min_sentences: 3,
            semantic_max_sentences: 15,
        }
    }
}

pub fn chapter_rag_chunk_config() -> RagChunkConfig {
    RagChunkConfig::default()
}
