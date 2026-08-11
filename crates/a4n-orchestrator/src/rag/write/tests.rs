use a4n_rag_core::{memory_store::MemoryVectorStore, vector_store::VectorStore};
use a4n_test_support::{
    embedding::{
        MatchingEmbeddingProvider, ShortEmbeddingProvider, WrongDimensionEmbeddingProvider,
    },
    rag::{ShortWriteVectorStore, root_text_metadata, test_embedding_identity},
};

use super::contracts::{RagEmbeddedInput, RagEmbeddingInput};
use super::embedded::store_embedded_rag_records_in_store;
use super::provider::{embed_and_store_rag_record_in_store, embed_and_store_rag_records_in_store};

#[tokio::test]
async fn direct_write_rejects_embedding_dimension_mismatch_without_writing_records() {
    let store = MemoryVectorStore::new();
    let error = embed_and_store_rag_record_in_store(
        &store,
        1,
        "rag:wrong-dimension",
        "正文",
        &WrongDimensionEmbeddingProvider,
        root_text_metadata(1),
    )
    .await
    .expect_err("wrong embedding dimension should reject");

    assert!(error.to_string().contains("嵌入返回维度不匹配"));
    assert!(
        store
            .metadata_by_ids("1", &["rag:wrong-dimension".to_string()])
            .await
            .expect("record state")
            .is_empty()
    );
}

#[tokio::test]
async fn direct_write_rejects_vector_store_short_write() {
    let store = ShortWriteVectorStore::new(0);
    let error = embed_and_store_rag_record_in_store(
        &store,
        1,
        "rag:1",
        "正文",
        &MatchingEmbeddingProvider,
        root_text_metadata(1),
    )
    .await
    .expect_err("short vector-store write should reject");

    assert!(error.to_string().contains("向量存储写入数量不匹配"));
    assert!(error.to_string().contains("expected=1"));
    assert!(error.to_string().contains("actual=0"));
}

#[tokio::test]
async fn batch_write_rejects_provider_count_mismatch_without_writing_records() {
    let store = MemoryVectorStore::new();
    let error = embed_and_store_rag_records_in_store(
        &store,
        1,
        vec![
            RagEmbeddingInput {
                chunk_id: "rag:1".to_string(),
                text: "第一条".to_string(),
                metadata: root_text_metadata(1),
            },
            RagEmbeddingInput {
                chunk_id: "rag:2".to_string(),
                text: "第二条".to_string(),
                metadata: root_text_metadata(1),
            },
        ],
        &ShortEmbeddingProvider,
    )
    .await
    .expect_err("short provider response should reject");

    assert!(error.to_string().contains("嵌入返回数量不匹配"));
    assert!(
        store
            .metadata_by_ids("1", &["rag:1".to_string(), "rag:2".to_string()])
            .await
            .expect("record state")
            .is_empty()
    );
}

#[tokio::test]
async fn batch_write_rejects_vector_store_short_write() {
    let store = ShortWriteVectorStore::new(1);
    let error = embed_and_store_rag_records_in_store(
        &store,
        1,
        vec![
            RagEmbeddingInput {
                chunk_id: "rag:1".to_string(),
                text: "第一条".to_string(),
                metadata: root_text_metadata(1),
            },
            RagEmbeddingInput {
                chunk_id: "rag:2".to_string(),
                text: "第二条".to_string(),
                metadata: root_text_metadata(1),
            },
        ],
        &MatchingEmbeddingProvider,
    )
    .await
    .expect_err("short vector-store write should reject");

    assert!(error.to_string().contains("向量存储写入数量不匹配"));
    assert!(error.to_string().contains("expected=2"));
    assert!(error.to_string().contains("actual=1"));
}

#[tokio::test]
async fn batch_write_preserves_per_record_metadata() {
    let store = MemoryVectorStore::new();
    let written = embed_and_store_rag_records_in_store(
        &store,
        9,
        vec![RagEmbeddingInput {
            chunk_id: "outline:9:1".to_string(),
            text: "正文内容".to_string(),
            metadata: serde_json::json!({
                "root_project_id": 9,
                "source_snapshot_id": null,
                "source_chapter_id": null,
                "source_snapshot_chapter_id": null,
                "derived_project_id": null,
                "data_type": "outline",
                "chapter_number": 1,
            }),
        }],
        &MatchingEmbeddingProvider,
    )
    .await
    .expect("batch store");

    assert_eq!(written, 1);
    let records = store
        .query("9", vec![0.0, 1.0], 1, None)
        .await
        .expect("stored records should be queryable");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].content, "正文内容");
    assert_eq!(records[0].metadata["project_id"], serde_json::json!(9));
    assert_eq!(records[0].metadata["root_project_id"], serde_json::json!(9));
    assert_eq!(
        records[0].metadata["data_type"],
        serde_json::json!("outline")
    );
    assert_eq!(records[0].metadata["content_length"], serde_json::json!(4));
    assert_eq!(
        records[0].metadata["embedding_dimension"],
        serde_json::json!(2)
    );
}

#[tokio::test]
async fn stored_embedding_write_validates_and_persists_caller_vectors() {
    let store = MemoryVectorStore::new();
    let written = store_embedded_rag_records_in_store(
        &store,
        5,
        vec![RagEmbeddedInput {
            chunk_id: "chapter:5:10:p1".to_string(),
            text: "客户端向量正文".to_string(),
            metadata: root_text_metadata(5),
            embedding: vec![1.0, 0.0],
        }],
        test_embedding_identity("client-test"),
    )
    .await
    .expect("stored embedding write");

    assert_eq!(written, 1);
    let records = store
        .query("5", vec![1.0, 0.0], 1, None)
        .await
        .expect("stored embedding should be queryable");
    assert_eq!(records[0].id, "chapter:5:10:p1");
    assert_eq!(records[0].metadata["embedding_model"], "client-test");
}
