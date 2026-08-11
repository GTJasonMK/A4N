use a4n_core::{error::AppError, models::chapter::chapter};
use a4n_embedding::provider::{EmbeddingBackend, EmbeddingProvider};
use a4n_test_support::embedding::UnavailableEmbeddingProvider;
use async_trait::async_trait;

use super::{
    chunks::chapter_rag_chunks_with_provider,
    contracts::ChapterRagChunkingMode,
    records::{chapter_rag_record_label, chapter_rag_records_with_provider},
};
use crate::rag::RagRecord;

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
async fn chapter_rag_chunks_use_semantic_dp_without_overlap() {
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
    assert!(chunks[0].content.contains("星门亮起"));
    assert!(!chunks[0].content.contains("厨房起火"));
    assert!(chunks[1].content.contains("厨房起火"));
    assert!(!chunks[1].content.starts_with("[...]"));
    assert_eq!(chunks[0].sentence_count, 3);
    assert!(chunks[0].density_score > 0.0);
}

#[tokio::test]
async fn chapter_rag_records_include_semantic_chunk_metadata() {
    let chapter = chapter_for_sync(
        "completed",
        "星门亮起。星门震动。星门关闭。厨房起火。厨房冒烟。厨房沉寂。",
    );
    let records = chapter_rag_records_with_provider(
        &chapter,
        &TopicEmbeddingProvider,
        ChapterRagChunkingMode::Semantic,
    )
    .await
    .expect("chapter records");
    let paragraph_records = records
        .iter()
        .filter(|record| {
            record
                .metadata
                .get("content_kind")
                .and_then(serde_json::Value::as_str)
                == Some("paragraph")
        })
        .collect::<Vec<_>>();

    assert_eq!(paragraph_records.len(), 2);
    assert_eq!(paragraph_records[0].metadata["semantic_chunked"], true);
    assert_eq!(paragraph_records[0].metadata["chunk_count"], 2);
    assert_eq!(paragraph_records[0].metadata["sentence_count"], 3);
    assert_eq!(paragraph_records[0].metadata["start_sentence_idx"], 0);
    assert_eq!(paragraph_records[0].metadata["end_sentence_idx"], 3);
    assert_eq!(
        paragraph_records[0].metadata["source_chapter_content_sha256"],
        a4n_core::source_spans::source_text_hash(&chapter.content)
    );
    assert_eq!(
        paragraph_records[0].metadata["source_chapter_updated_at"],
        chapter.updated_at.to_string()
    );
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
    assert_eq!(records[0].metadata["semantic_chunked"], false);
}

#[tokio::test]
async fn chapter_rag_record_ids_match_strategy_chunk_count() {
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

#[test]
fn chapter_rag_record_label_rejects_incomplete_paragraph_metadata() {
    let chapter = chapter_for_sync("completed", "正文");
    let valid = RagRecord {
        chunk_id: "chapter:1:1:p1".to_string(),
        text: "正文".to_string(),
        metadata: serde_json::json!({
            "content_kind": "paragraph",
            "chunk_index": 1,
            "chunk_count": 2,
        }),
    };
    assert_eq!(
        chapter_rag_record_label(&chapter, &valid).expect("valid paragraph label"),
        "第1章正文分块 1/2"
    );

    for metadata in [
        serde_json::json!({}),
        serde_json::json!({
            "content_kind": "paragraph",
            "chunk_index": 0,
            "chunk_count": 2,
        }),
        serde_json::json!({
            "content_kind": "paragraph",
            "chunk_index": 1,
        }),
    ] {
        let record = RagRecord {
            metadata,
            ..valid.clone()
        };
        chapter_rag_record_label(&chapter, &record)
            .expect_err("incomplete paragraph label metadata must fail");
    }
}
