use super::chapter::required_import_chapter_number;
use super::*;
use serde_json::json;

#[test]
fn normalize_import_review_status_accepts_defined_statuses() {
    for status in ALLOWED_IMPORT_REVIEW_STATUSES {
        assert_eq!(
            normalize_import_review_status(status).expect("status should be valid"),
            status
        );
        assert_eq!(
            normalize_import_review_status(&format!(" {status}\n"))
                .expect("trimmed status should be valid"),
            status
        );
    }
}

#[test]
fn normalize_import_review_status_rejects_unknown_status() {
    let err =
        normalize_import_review_status("approved").expect_err("legacy status should be rejected");
    assert!(matches!(err, ImportReviewMutationError::InvalidArgument(_)));
    assert!(err.to_string().contains("不支持的导入审核状态"));
}

#[test]
fn normalize_import_review_status_rejects_blank_status() {
    let err = normalize_import_review_status(" \n\t").expect_err("blank status should be rejected");
    assert!(matches!(err, ImportReviewMutationError::InvalidArgument(_)));
    assert!(err.to_string().contains("不能为空"));
}

#[test]
fn required_import_chapter_number_rejects_non_positive_number() {
    let err = required_import_chapter_number(&json!({"chapter_number": 0}), 42)
        .expect_err("chapter_number 0 must fail before writing chapters");
    assert!(err.to_string().contains("章节序号必须为正整数"));
    assert!(err.to_string().contains("review_id=42"));
}
