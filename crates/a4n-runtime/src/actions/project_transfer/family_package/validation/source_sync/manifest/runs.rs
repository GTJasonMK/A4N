use a4n_orchestrator::source_sync::{
    SYNC_RUN_MODE_APPLY, SYNC_RUN_MODE_DRY_RUN, SYNC_RUN_STATUS_APPLYING,
    SYNC_RUN_STATUS_COMPLETED, SYNC_RUN_STATUS_PLANNED, is_valid_sync_run_mode,
    is_valid_sync_run_status,
};

use super::super::super::ProjectFamilyPackage;

pub(super) fn validate_source_sync_runs(package: &ProjectFamilyPackage) -> Result<(), String> {
    for run in &package.source_sync_runs {
        ensure_package!(
            is_valid_sync_run_mode(&run.mode),
            "项目族导入包源同步运行模式无效: source_sync_run_id={}, mode={}",
            run.id,
            run.mode
        );
        ensure_package!(
            is_valid_sync_run_status(&run.status),
            "项目族导入包源同步运行状态无效: source_sync_run_id={}, status={}",
            run.id,
            run.status
        );
        match run.status.as_str() {
            SYNC_RUN_STATUS_PLANNED if run.mode != SYNC_RUN_MODE_DRY_RUN => {
                return Err(format!(
                    "项目族导入包源同步运行状态与模式不一致: source_sync_run_id={}, status={}, mode={}",
                    run.id, run.status, run.mode
                ));
            }
            SYNC_RUN_STATUS_APPLYING | SYNC_RUN_STATUS_COMPLETED
                if run.mode != SYNC_RUN_MODE_APPLY =>
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
