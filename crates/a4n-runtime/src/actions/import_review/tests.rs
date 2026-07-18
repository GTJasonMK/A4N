use super::projections::{import_analysis_status_summaries, import_review_summary};
use a4n_core::{error::AppError, models::project::import_review_queue};

#[test]
fn import_review_summary_rejects_unknown_review_status() {
    let review = review_with_status(7, "approved");

    let err = import_review_summary(&review).expect_err("legacy status should fail");

    assert!(matches!(err, AppError::InvalidState { .. }));
    assert!(err.to_string().contains("导入审核 #7"));
    assert!(err.to_string().contains("approved"));
}

#[test]
fn import_analysis_status_summaries_rejects_unknown_review_status() {
    let err = import_analysis_status_summaries(vec![review_with_status(8, "archived")])
        .expect_err("unknown status should fail");

    assert!(matches!(err, AppError::InvalidState { .. }));
    assert!(err.to_string().contains("导入审核 #8"));
    assert!(err.to_string().contains("archived"));
}

#[test]
fn import_analysis_status_summaries_normalizes_trimmed_statuses() {
    let summaries = import_analysis_status_summaries(vec![
        review_with_status(1, "pending"),
        review_with_status(2, " pending\n"),
        review_with_status(3, "rejected"),
    ])
    .expect("valid statuses should summarize");

    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].status, "pending");
    assert_eq!(summaries[0].count, 2);
    assert_eq!(summaries[1].status, "rejected");
    assert_eq!(summaries[1].count, 1);
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
        file_type: "markdown".to_string(),
        detected_encoding: Some("utf-8".to_string()),
        parsed_chapters: 1,
        dedup_paragraphs: 1,
        dedup_rate: Some(0.0),
        analysis_result: Some(serde_json::json!({"chapter_number": 1})),
        review_status: status.to_string(),
        created_at: timestamp,
        updated_at: timestamp,
    }
}
