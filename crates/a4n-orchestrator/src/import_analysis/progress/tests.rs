use a4n_api_contract::import::{ImportAnalysisStageStatus, ImportAnalysisStatus};
use serde_json::json;

use super::*;

#[test]
fn progress_uses_one_typed_stage_from_start_to_completion() {
    let mut progress = initial_import_progress(4).unwrap();

    update_import_progress(&mut progress, 2, "已完成第2章分析").unwrap();
    assert_eq!(progress.overall_progress, 50);
    assert_eq!(
        progress.stages.generating_analysis_data.status,
        ImportAnalysisStageStatus::InProgress
    );

    finish_import_progress(&mut progress).unwrap();
    assert_eq!(progress.status, ImportAnalysisStatus::Completed);
    assert_eq!(progress.overall_progress, 100);
    assert!(progress.completed_at.is_some());
}

#[test]
fn resume_clears_terminal_state_without_resetting_completed_chapters() {
    let mut progress = initial_import_progress(4).unwrap();
    update_import_progress(&mut progress, 2, "已完成第2章分析").unwrap();
    fail_import_progress(&mut progress, "provider failed").unwrap();

    resume_import_progress(&mut progress).unwrap();

    assert_eq!(progress.status, ImportAnalysisStatus::Analyzing);
    assert_eq!(progress.stages.generating_analysis_data.completed, 2);
    assert_eq!(progress.error, None);
    assert_eq!(progress.failed_at, None);
}

#[test]
fn strict_progress_contract_rejects_retired_extra_stages() {
    let mut value = serde_json::to_value(initial_import_progress(4).unwrap()).unwrap();
    value["stages"]["extracting_blueprint"] = json!({
        "name": "反推蓝图信息",
        "completed": 1,
        "total": 1,
        "status": "completed",
    });

    assert!(serde_json::from_value::<ProjectImportAnalysisProgress>(value).is_err());
}

#[test]
fn validation_rejects_inconsistent_derived_percent() {
    let mut progress = initial_import_progress(4).unwrap();
    progress.overall_progress = 75;

    let error = validate_import_progress(&progress).unwrap_err();

    assert!(error.to_string().contains("百分比不一致"));
}

#[test]
fn completed_progress_cannot_be_resumed_as_a_hidden_cache_hit() {
    let mut progress = initial_import_progress(1).unwrap();
    finish_import_progress(&mut progress).unwrap();

    assert!(resume_import_progress(&mut progress).is_err());
}
