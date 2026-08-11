use super::projections::import_review_summary;
use a4n_core::{error::AppError, models::project::import_review_queue};

#[test]
fn import_review_summary_rejects_unknown_review_status() {
    let review = review_with_status(7, "approved");

    let err = import_review_summary(&review).expect_err("legacy status should fail");

    assert!(matches!(err, AppError::InvalidState { .. }));
    assert!(err.to_string().contains("导入审核 #7"));
    assert!(err.to_string().contains("approved"));
}

fn review_with_status(id: i64, status: &str) -> import_review_queue::Model {
    let timestamp = chrono::NaiveDate::from_ymd_opt(2026, 6, 29)
        .expect("valid date")
        .and_hms_opt(23, 30, 0)
        .expect("valid time");
    import_review_queue::Model {
        id,
        project_id: 42,
        filename: "demo.md".to_string(),
        analysis_result: serde_json::json!({"chapter_number": 1}),
        review_status: status.to_string(),
        created_at: timestamp,
        updated_at: timestamp,
    }
}
