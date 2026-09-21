use super::execution::{all_records_indexed, merged_rag_error};

#[test]
fn character_sync_rag_error_contract_preserves_delete_and_ingest_order() {
    assert_eq!(merged_rag_error(None, &[]), None);
    assert_eq!(
        merged_rag_error(Some("delete".to_string()), &[]),
        Some("delete".to_string())
    );
    assert_eq!(
        merged_rag_error(None, &["first".to_string(), "second".to_string()]),
        Some("入库失败：first；second".to_string())
    );
    assert_eq!(
        merged_rag_error(
            Some("delete".to_string()),
            &["first".to_string(), "second".to_string()]
        ),
        Some("清理失败：delete；入库失败：first；second".to_string())
    );
}

#[test]
fn character_sync_rag_indexed_flag_requires_exact_count() {
    assert!(all_records_indexed(0, 0));
    assert!(all_records_indexed(3, 3));
    assert!(!all_records_indexed(2, 3));
    assert!(!all_records_indexed(4, 3));
}
