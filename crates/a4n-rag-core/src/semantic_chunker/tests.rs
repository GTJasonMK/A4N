use super::*;
use a4n_embedding::provider::{EmbeddingBackend, EmbeddingProvider};
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
