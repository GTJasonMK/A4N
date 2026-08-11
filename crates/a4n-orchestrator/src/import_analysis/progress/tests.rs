use super::resume::{
    import_progress_resume_stage, import_progress_totals_match, resume_import_progress,
};
use super::schema::{parse_import_progress, recalc_import_progress};
use super::updates::{
    finish_import_progress, mark_import_progress_failed, progress_stage_completed,
    update_import_progress_stage,
};
use serde_json::{Value, json};

fn sample_progress(status: &str, completed: usize, total: usize) -> Value {
    json!({
        "status": status,
        "current_stage": "generating_analysis_data",
        "stages": {
            "generating_analysis_data": {
                "name": "逐章分析与资料同步",
                "completed": completed,
                "total": total,
                "status": if completed >= total { "completed" } else { "in_progress" },
            }
        }
    })
}

#[test]
fn totals_match_requires_only_the_chapter_sync_stage() {
    let progress = sample_progress("failed", 12, 20);
    assert!(import_progress_totals_match(&progress, 20));
    assert!(!import_progress_totals_match(&progress, 21));

    let mut legacy = progress;
    legacy["stages"]["extracting_blueprint"] = json!({
        "name": "反推蓝图信息",
        "completed": 1,
        "total": 1,
        "status": "completed",
    });
    assert!(!import_progress_totals_match(&legacy, 20));
}

#[test]
fn resume_progress_marks_current_stage_in_progress() {
    let mut progress = sample_progress("failed", 12, 20);
    assert_eq!(
        import_progress_resume_stage(&progress),
        Some("generating_analysis_data".to_string())
    );
    resume_import_progress(&mut progress, "generating_analysis_data").expect("resume succeeds");
    assert_eq!(progress["status"], "analyzing");
    assert_eq!(progress["error"], Value::Null);
    assert_eq!(progress["current_stage"], "generating_analysis_data");
    assert_eq!(
        progress["stages"]["generating_analysis_data"]["status"],
        "in_progress"
    );
}

#[test]
fn failure_progress_records_error_and_timestamp() {
    let mut progress = sample_progress("analyzing", 12, 20);
    mark_import_progress_failed(&mut progress, "boom").expect("failure progress is valid");
    assert_eq!(progress["status"], "failed");
    assert_eq!(progress["error"], "boom");
    assert_eq!(progress["message"], "导入分析失败: boom");
    assert!(progress["failed_at"].as_str().is_some());
    assert_eq!(
        progress["stages"]["generating_analysis_data"]["status"],
        "failed"
    );
}

#[test]
fn invalid_progress_json_is_reported() {
    let err = parse_import_progress(42, Some("{broken")).expect_err("invalid JSON should fail");
    assert!(err.contains("project_id=42"));
}

#[test]
fn stage_completed_reads_completed_count() {
    let progress = sample_progress("analyzing", 12, 20);
    assert_eq!(
        progress_stage_completed(&progress, "generating_analysis_data")
            .expect("completed count is present"),
        12
    );
    let err = progress_stage_completed(&progress, "missing_stage")
        .expect_err("missing stage should be explicit");
    assert!(err.contains("缺少阶段: missing_stage"));
}

#[test]
fn recalc_rejects_malformed_stage_counts() {
    let mut progress = sample_progress("analyzing", 12, 20);
    progress["stages"]["generating_analysis_data"]
        .as_object_mut()
        .expect("stage object")
        .remove("total");

    let err = recalc_import_progress(&mut progress).expect_err("missing total should fail");
    assert!(err.contains("generating_analysis_data 缺少 total"));
}

#[test]
fn recalc_rejects_completed_count_above_total() {
    let mut progress = sample_progress("analyzing", 12, 20);
    progress["stages"]["generating_analysis_data"]["completed"] = json!(21);

    let err = recalc_import_progress(&mut progress).expect_err("completed above total should fail");

    assert!(err.contains("generating_analysis_data.completed 不能大于 total"));
    assert!(err.contains("completed=21"));
    assert!(err.contains("total=20"));
}

#[test]
fn recalc_rejects_zero_total_progress() {
    let mut progress = json!({
        "status": "analyzing",
        "current_stage": "empty",
        "stages": {
            "empty": {
                "name": "空进度",
                "completed": 0,
                "total": 0,
                "status": "pending",
            }
        }
    });

    let err = recalc_import_progress(&mut progress).expect_err("zero total should fail");

    assert!(err.contains("导入分析进度总数必须大于 0"));
}

#[test]
fn update_progress_rejects_completed_count_above_total_without_mutating() {
    let mut progress = sample_progress("analyzing", 12, 20);
    let before = progress.clone();

    let err = update_import_progress_stage(
        &mut progress,
        "generating_analysis_data",
        21,
        20,
        "bad progress",
    )
    .expect_err("completed above total should fail");

    assert!(err.contains("generating_analysis_data.completed 不能大于 total"));
    assert_eq!(progress, before);
}

#[test]
fn finish_rejects_noninteger_stage_total() {
    let mut progress = sample_progress("analyzing", 12, 20);
    progress["stages"]["generating_analysis_data"]["total"] = json!("20");

    let err = finish_import_progress(&mut progress).expect_err("noninteger total should fail");
    assert!(err.contains("generating_analysis_data.total 必须是非负整数"));
}
