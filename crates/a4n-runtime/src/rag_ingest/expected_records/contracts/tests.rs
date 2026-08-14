use a4n_orchestrator::rag::RagRecord;
use serde_json::json;

use super::ExpectedRagRecords;

#[test]
fn expected_rag_records_reject_duplicate_chunk_ids() {
    let mut expected = ExpectedRagRecords::default();
    let record = |text: &str| RagRecord {
        chunk_id: "chunk:1".to_string(),
        text: text.to_string(),
        metadata: json!({}),
    };

    expected
        .add_rag_record(record("正文"))
        .expect("first record");
    let error = expected
        .add_rag_record(record("另一段正文"))
        .expect_err("duplicate chunk id should reject");

    assert!(error.contains("chunk_id 重复"));
    assert_eq!(expected.total(), 1);
    assert!(expected.ids.contains("chunk:1"));
}
