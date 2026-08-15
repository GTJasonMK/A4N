use super::projections::source_sync_run_summary;
use a4n_core::models::source_sync::source_sync_run;
use a4n_domain::source_sync::{SourceSyncRunMode, SourceSyncRunStatus};
use chrono::NaiveDate;
use serde_json::json;

#[test]
fn source_sync_run_summary_preserves_public_contract_fields() {
    let created_at = NaiveDate::from_ymd_opt(2026, 6, 1)
        .unwrap()
        .and_hms_opt(2, 3, 4)
        .unwrap();
    let updated_at = NaiveDate::from_ymd_opt(2026, 6, 2)
        .unwrap()
        .and_hms_opt(3, 4, 5)
        .unwrap();
    let run = source_sync_run::Model {
        id: 1,
        derived_project_id: 2,
        root_project_id: 3,
        from_snapshot_id: 4,
        to_snapshot_id: 5,
        mode: "dry_run".to_string(),
        status: "planned".to_string(),
        summary_json: json!({"conflicts": 6}),
        error_message: Some("err".to_string()),
        created_at,
        updated_at,
    };

    let summary = source_sync_run_summary(run).expect("valid source sync run should serialize");

    assert_eq!(summary.id, 1);
    assert_eq!(summary.derived_project_id, 2);
    assert_eq!(summary.root_project_id, 3);
    assert_eq!(summary.from_snapshot_id, 4);
    assert_eq!(summary.to_snapshot_id, 5);
    assert_eq!(summary.mode, SourceSyncRunMode::DryRun);
    assert_eq!(summary.status, SourceSyncRunStatus::Planned);
    assert_eq!(summary.summary_json.get("conflicts"), Some(&json!(6)));
    assert_eq!(summary.error_message.as_deref(), Some("err"));
    assert_eq!(summary.created_at, "2026-06-01 02:03:04");
    assert_eq!(summary.updated_at, "2026-06-02 03:04:05");
}
