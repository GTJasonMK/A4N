use super::*;

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_run_mode() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut run = source_sync_run_record();
    run.mode = "preview".to_string();
    package.source_sync_runs = vec![run];

    let err =
        validate_package_manifest(&package).expect_err("source sync run mode must use known modes");

    assert!(err.contains("源同步运行模式无效"));
    assert!(err.contains("source_sync_run_id=801"));
    assert!(err.contains("mode=preview"));
}

#[test]
fn validate_package_manifest_rejects_unknown_source_sync_run_status() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut run = source_sync_run_record();
    run.status = "failed".to_string();
    package.source_sync_runs = vec![run];

    let err = validate_package_manifest(&package)
        .expect_err("source sync run status must use known states");

    assert!(err.contains("源同步运行状态无效"));
    assert!(err.contains("source_sync_run_id=801"));
    assert!(err.contains("status=failed"));
}

#[test]
fn validate_package_manifest_rejects_source_sync_run_status_mode_mismatch() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut run = source_sync_run_record();
    run.status = SourceSyncRunStatus::Completed.as_str().to_string();
    package.source_sync_runs = vec![run];

    let err = validate_package_manifest(&package)
        .expect_err("completed source sync runs must have apply mode");

    assert!(err.contains("源同步运行状态与模式不一致"));
    assert!(err.contains("source_sync_run_id=801"));
    assert!(err.contains("status=completed"));
    assert!(err.contains("mode=dry_run"));
}

#[test]
fn validate_package_manifest_accepts_known_source_sync_run_states() {
    for status in SourceSyncRunStatus::VALUES {
        let mut package = package_with_part_outlines(Vec::new());
        let mut run = source_sync_run_record();
        run.status = (*status).to_string();
        run.mode = if *status == SourceSyncRunStatus::Planned.as_str() {
            SourceSyncRunMode::DryRun.as_str().to_string()
        } else {
            SourceSyncRunMode::Apply.as_str().to_string()
        };
        package.source_sync_runs = vec![run];

        validate_package_manifest(&package)
            .expect("service-generated source sync run state should import");
    }
}

#[test]
fn validate_package_manifest_rejects_non_object_source_sync_run_summary() {
    let mut package = package_with_part_outlines(Vec::new());
    let mut run = source_sync_run_record();
    run.summary_json = json!(["not", "an", "object"]);
    package.source_sync_runs = vec![run];

    let err = validate_package_manifest(&package)
        .expect_err("source sync run summary_json must keep service object invariant");

    assert!(err.contains("源同步运行摘要必须是 JSON 对象"));
    assert!(err.contains("source_sync_run_id=801"));
}
