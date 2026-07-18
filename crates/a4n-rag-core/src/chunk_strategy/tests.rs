use super::*;

#[test]
fn chapter_strategy_uses_current_semantic_config() {
    let config = chapter_rag_chunk_config();

    assert_eq!(config.min_chunk_length, 100);
    assert_eq!(config.max_chunk_length, 1200);
    assert_eq!(config.semantic_gate_threshold, 0.3);
    assert_eq!(config.semantic_alpha, 0.1);
    assert_eq!(config.semantic_gamma, 1.1);
    assert_eq!(config.semantic_min_sentences, 3);
    assert_eq!(config.semantic_max_sentences, 15);
}
