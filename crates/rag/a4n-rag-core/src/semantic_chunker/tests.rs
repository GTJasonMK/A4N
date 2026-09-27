use super::*;
use a4n_embedding_provider_contract::{EmbeddingBackend, EmbeddingProvider};
use async_trait::async_trait;

struct TopicEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for TopicEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        Ok(texts
            .into_iter()
            .map(|text| {
                if text.contains("星门") {
                    vec![1.0, 0.0]
                } else {
                    vec![0.0, 1.0]
                }
            })
            .collect())
    }

    fn dimension(&self) -> usize {
        2
    }

    fn model_name(&self) -> &str {
        "topic-test"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::LocalHash
    }
}

struct WrongDimensionEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for WrongDimensionEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, AppError> {
        Ok(texts.into_iter().map(|_| vec![1.0, 0.0]).collect())
    }

    fn dimension(&self) -> usize {
        3
    }

    fn model_name(&self) -> &str {
        "wrong-dimension-test"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::LocalHash
    }
}

fn semantic_config() -> SemanticChunkConfig {
    SemanticChunkConfig {
        gate_threshold: 0.3,
        alpha: 0.1,
        gamma: 1.1,
        min_chunk_sentences: 2,
        max_chunk_sentences: 4,
        min_chunk_chars: 1,
        max_chunk_chars: 500,
    }
}

#[test]
fn semantic_prefix_reuses_allocation_and_preserves_block_scores() {
    let embeddings = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 0.0]];
    let matrix = build_similarity_matrix(&embeddings);
    let allocation = matrix.as_ptr();
    let matrix = apply_structure_enhancement(matrix, 3, 0.3, 0.0);
    let prefix = compute_prefix_sum(matrix, 3);
    assert_eq!(prefix.as_ptr(), allocation);
    assert_eq!(
        prefix,
        vec![
            0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 0.0, 1.0, 2.0, 3.0, 0.0, 2.0, 3.0, 5.0
        ]
    );
    assert_eq!(matrix::block_score(&prefix, 3, 0, 3, 0.0), 5.0);
    assert_eq!(matrix::block_score(&prefix, 3, 1, 3, 0.0), 2.0);
    let zero = build_similarity_matrix(&[vec![0.0], vec![1.0]]);
    assert_eq!(
        compute_prefix_sum(zero, 2),
        vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]
    );
    assert_eq!(
        compute_prefix_sum(build_similarity_matrix(&[]), 0),
        vec![0.0]
    );
}

#[test]
#[ignore = "manual CPU/RSS measurement; run alone with --release --ignored --nocapture"]
fn performance_semantic_chunking() {
    let count: usize = std::env::var("A4N_PERF_SENTENCES")
        .unwrap_or_else(|_| "1600".into())
        .parse()
        .expect("A4N_PERF_SENTENCES must be an integer");
    assert!(count > 0);
    let sentences = (0..count)
        .map(|index| format!("第{index}句描写星门附近的人物与环境。"))
        .collect::<Vec<_>>();
    let expected_text = sentences.join("");
    let embeddings = (0..count)
        .map(|row| {
            (0..768)
                .map(|col| ((row * 31 + col * 17) % 101) as f32 / 50.0 - 1.0)
                .collect::<Vec<_>>()
        })
        .collect();
    let start = std::time::Instant::now();
    let chunks =
        chunk_sentences_with_embeddings(sentences, embeddings, &SemanticChunkConfig::chapter())
            .expect("semantic chunking");
    let elapsed = start.elapsed();
    assert_eq!(chunks.concat(), expected_text);
    let lengths = chunks.iter().map(String::len).collect::<Vec<_>>();
    println!(
        "PERF semantic sentences={count} dimensions=768 elapsed_ms={:.3} chunk_bytes={lengths:?}",
        elapsed.as_secs_f64() * 1000.0
    );
}

#[test]
fn splits_novel_sentences_with_quotes_and_long_sentence_secondary_boundaries() {
    let text = "他说：「星门开了。」她没有回答……只是看向远处。\n这是一个很长的句子，带着许多逗号，继续描述星门边缘的光，继续描述风声，继续描述人群，继续描述远处城墙，继续描述未完成的预兆，继续描述暗线，继续描述疑问。";

    let sentences = split_novel_sentences(text);

    assert!(
        sentences
            .iter()
            .any(|sentence| sentence == "他说：「星门开了。」")
    );
    assert!(
        sentences
            .iter()
            .any(|sentence| sentence.contains("她没有回答"))
    );
    assert!(sentences.len() >= 3);
}

#[tokio::test]
async fn semantic_dp_uses_sentence_embeddings_for_topic_cut_points() {
    let config = semantic_config();
    let text = "星门亮起。星门震动。星门关闭。厨房起火。厨房冒烟。厨房沉寂。";

    let chunks = chunk_text(text, &TopicEmbeddingProvider, &config)
        .await
        .expect("semantic chunking should succeed");

    assert_eq!(chunks.len(), 2);
    assert!(chunks[0].contains("星门亮起"));
    assert!(!chunks[0].contains("厨房起火"));
    assert!(chunks[1].contains("厨房起火"));
}

#[tokio::test]
async fn semantic_dp_rejects_embedding_dimension_mismatch() {
    let config = semantic_config();
    let text = "星门亮起。星门震动。厨房起火。厨房冒烟。";

    let err = chunk_text(text, &WrongDimensionEmbeddingProvider, &config)
        .await
        .expect_err("wrong embedding dimension should reject");

    assert!(err.to_string().contains("嵌入返回维度不匹配"));
}
