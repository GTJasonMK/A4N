use a4n_domain::source_sync::{SourceSyncRunMode, SourceSyncRunStatus};

use super::super::super::ProjectFamilyPackage;

pub(super) fn validate_source_sync_runs(package: &ProjectFamilyPackage) -> Result<(), String> {
    for run in &package.source_sync_runs {
        let mode = SourceSyncRunMode::parse(&run.mode).ok_or_else(|| {
            format!(
                "项目族导入包源同步运行模式无效: source_sync_run_id={}, mode={}",
                run.id, run.mode
            )
        })?;
        let status = SourceSyncRunStatus::parse(&run.status).ok_or_else(|| {
            format!(
                "项目族导入包源同步运行状态无效: source_sync_run_id={}, status={}",
                run.id, run.status
            )
        })?;
        match status {
            SourceSyncRunStatus::Planned if mode != SourceSyncRunMode::DryRun => {
                return Err(format!(
                    "项目族导入包源同步运行状态与模式不一致: source_sync_run_id={}, status={}, mode={}",
                    run.id, run.status, run.mode
                ));
            }
            SourceSyncRunStatus::Applying | SourceSyncRunStatus::Completed
                if mode != SourceSyncRunMode::Apply =>
            {
                return Err(format!(
                    "项目族导入包源同步运行状态与模式不一致: source_sync_run_id={}, status={}, mode={}",
                    run.id, run.status, run.mode
                ));
            }
            _ => {}
        }
        ensure_package!(
            run.summary_json.is_object(),
            "项目族导入包源同步运行摘要必须是 JSON 对象: source_sync_run_id={}",
            run.id
        );
    }
    Ok(())
}
