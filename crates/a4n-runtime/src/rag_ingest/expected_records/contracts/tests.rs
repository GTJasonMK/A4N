use a4n_rag_core::metadata::content_hash;

use super::ExpectedRagRecords;

#[test]
fn expected_rag_records_reject_duplicate_chunk_ids_and_store_hashes() {
    let mut expected = ExpectedRagRecords::default();

    expected
        .add_record("chunk:1".to_string(), "正文")
        .expect("first record");
    let err = expected
        .add_record("chunk:1".to_string(), "另一段正文")
        .expect_err("duplicate chunk id should reject");

    assert!(err.contains("chunk_id 重复"));
    assert_eq!(expected.ids, vec!["chunk:1".to_string()]);
    assert_eq!(expected.hashes.len(), 1);
    let expected_hash = content_hash("正文").expect("hash");
    assert_eq!(
        expected.hashes.get("chunk:1").map(String::as_str),
        Some(expected_hash.as_str())
    );
}
