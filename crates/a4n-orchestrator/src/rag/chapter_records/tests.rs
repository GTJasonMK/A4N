use a4n_core::{error::AppError, models::chapter::chapter};
use a4n_embedding::provider::{EmbeddingBackend, EmbeddingProvider};
use a4n_test_support::embedding::UnavailableEmbeddingProvider;
use async_trait::async_trait;

use super::{
    chunks::chapter_rag_chunks_with_provider, contracts::ChapterRagChunkingMode,
    records::chapter_rag_records_with_provider,
};

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

fn chapter_for_sync(status: &str, content: &str) -> chapter::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter::Model {
        id: 1,
        project_id: 1,
        outline_id: None,
        chapter_number: 1,
        title: Some("测试章节".to_string()),
        content: content.to_string(),
        word_count: content.chars().count() as i32,
        status: status.to_string(),
        summary: None,
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test]
async fn chapter_rag_chunks_use_semantic_topic_boundaries() {
    let chapter = chapter_for_sync(
        "completed",
        "星门亮起。星门震动。星门关闭。厨房起火。厨房冒烟。厨房沉寂。",
    );
    let chunks = chapter_rag_chunks_with_provider(
        &chapter,
        &TopicEmbeddingProvider,
        ChapterRagChunkingMode::Semantic,
    )
    .await
    .expect("chapter chunks");

    assert_eq!(chunks.len(), 2);
    assert!(chunks[0].contains("星门亮起"));
    assert!(!chunks[0].contains("厨房起火"));
    assert!(chunks[1].contains("厨房起火"));
}

#[tokio::test]
async fn client_desktop_chunks_do_not_call_server_embedding_provider() {
    let chapter = chapter_for_sync("completed", "林岚进入旧城。她发现墙上的暗号。");
    let records = chapter_rag_records_with_provider(
        &chapter,
        &UnavailableEmbeddingProvider::new("provider should not be called"),
        ChapterRagChunkingMode::ClientDesktop,
    )
    .await
    .expect("client desktop records");

    assert!(!records.is_empty());
}

#[tokio::test]
async fn chapter_rag_record_ids_match_generated_chunks() {
    let mut chapter = chapter_for_sync(
        "completed",
        "星门亮起。星门震动。星门关闭。厨房起火。厨房冒烟。厨房沉寂。",
    );
    chapter.summary = Some("主角完成一次关键判断。".to_string());

    let chunks = chapter_rag_chunks_with_provider(
        &chapter,
        &TopicEmbeddingProvider,
        ChapterRagChunkingMode::Semantic,
    )
    .await
    .expect("chapter chunks");
    let ids = chapter_rag_records_with_provider(
        &chapter,
        &TopicEmbeddingProvider,
        ChapterRagChunkingMode::Semantic,
    )
    .await
    .expect("chapter records")
    .into_iter()
    .map(|record| record.chunk_id)
    .collect::<Vec<_>>();

    assert_eq!(chunks.len(), 2);
    assert_eq!(
        ids,
        vec![
            "chapter:1:1:p1".to_string(),
            "chapter:1:1:p2".to_string(),
            "chapter:1:1:summary".to_string(),
        ]
    );
}

#[tokio::test]
async fn chapter_rag_records_include_analysis_outputs() {
    let mut chapter = chapter_for_sync("completed", "林岚在旧城发现暗号。");
    chapter.analysis_data = Some(serde_json::json!({
        "metadata": {
            "characters": ["林岚"],
            "locations": ["旧城"],
            "items": [],
            "tags": ["追查"],
            "tone": "紧张",
            "timeline_marker": null
        },
        "key_events": [{ "description": "林岚发现旧城暗号", "importance": "high" }],
        "foreshadowing": {
            "planted": [{
                "description": "暗号指向后续叛徒身份",
                "priority": "high",
                "category": "plot_twist"
            }]
        }
    }));

    let ids = chapter_rag_records_with_provider(
        &chapter,
        &TopicEmbeddingProvider,
        ChapterRagChunkingMode::Semantic,
    )
    .await
    .expect("chapter records")
    .into_iter()
    .map(|record| record.chunk_id)
    .collect::<Vec<_>>();

    assert!(ids.contains(&"chapter:1:1:p1".to_string()));
    assert!(ids.contains(&"chapter:1:1:metadata".to_string()));
    assert!(ids.contains(&"chapter:1:1:event1".to_string()));
    assert!(ids.contains(&"chapter:1:1:foreshadow1".to_string()));
}
