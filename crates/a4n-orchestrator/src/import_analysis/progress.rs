use a4n_api_contract::import::{
    ImportAnalysisStageKey, ImportAnalysisStageProgress, ImportAnalysisStageStatus,
    ImportAnalysisStages, ImportAnalysisStatus, ProjectImportAnalysisProgress,
};
use a4n_core::error::AppError;
use chrono::Utc;

use crate::OrchestratorContext;

const ANALYSIS_STAGE_NAME: &str = "逐章分析与资料同步";

pub(super) fn initial_import_progress(
    total_chapters: usize,
) -> Result<ProjectImportAnalysisProgress, AppError> {
    if total_chapters == 0 {
        return Err(invalid_progress("导入分析章节总数必须大于 0"));
    }
    Ok(ProjectImportAnalysisProgress {
        status: ImportAnalysisStatus::Analyzing,
        current_stage: ImportAnalysisStageKey::GeneratingAnalysisData,
        stages: ImportAnalysisStages {
            generating_analysis_data: ImportAnalysisStageProgress {
                name: ANALYSIS_STAGE_NAME.to_string(),
                completed: 0,
                total: total_chapters,
                status: ImportAnalysisStageStatus::Pending,
            },
        },
        message: "正在初始化导入分析...".to_string(),
        error: None,
        overall_progress: 0,
        started_at: timestamp(),
        completed_at: None,
        failed_at: None,
        cancelled_at: None,
    })
}

pub(super) fn completed_import_chapters(
    progress: &ProjectImportAnalysisProgress,
) -> Result<usize, AppError> {
    validate_import_progress(progress)?;
    Ok(progress.stages.generating_analysis_data.completed)
}

pub(super) fn resume_import_progress(
    progress: &mut ProjectImportAnalysisProgress,
) -> Result<(), AppError> {
    validate_import_progress(progress)?;
    if progress.status == ImportAnalysisStatus::Completed {
        return Err(invalid_progress("已完成的导入分析必须作为新任务重新运行"));
    }
    let stage = &mut progress.stages.generating_analysis_data;
    stage.status = if stage.completed == stage.total {
        ImportAnalysisStageStatus::Completed
    } else {
        ImportAnalysisStageStatus::InProgress
    };
    progress.status = ImportAnalysisStatus::Analyzing;
    progress.message = "正在恢复导入分析...".to_string();
    progress.error = None;
    progress.completed_at = None;
    progress.failed_at = None;
    progress.cancelled_at = None;
    Ok(())
}

pub(super) fn update_import_progress(
    progress: &mut ProjectImportAnalysisProgress,
    completed: usize,
    message: impl Into<String>,
) -> Result<(), AppError> {
    validate_import_progress(progress)?;
    let stage = &mut progress.stages.generating_analysis_data;
    if completed > stage.total {
        return Err(invalid_progress(format!(
            "导入分析已完成章节数不能大于总数: completed={completed}, total={}",
            stage.total
        )));
    }
    stage.completed = completed;
    stage.status = if completed == stage.total {
        ImportAnalysisStageStatus::Completed
    } else {
        ImportAnalysisStageStatus::InProgress
    };
    progress.status = ImportAnalysisStatus::Analyzing;
    progress.message = message.into();
    progress.error = None;
    progress.overall_progress = progress_percent(completed, stage.total)?;
    progress.completed_at = None;
    progress.failed_at = None;
    progress.cancelled_at = None;
    Ok(())
}

pub(super) fn finish_import_progress(
    progress: &mut ProjectImportAnalysisProgress,
) -> Result<(), AppError> {
    validate_import_progress(progress)?;
    let stage = &mut progress.stages.generating_analysis_data;
    stage.completed = stage.total;
    stage.status = ImportAnalysisStageStatus::Completed;
    progress.status = ImportAnalysisStatus::Completed;
    progress.message = "导入分析完成".to_string();
    progress.error = None;
    progress.overall_progress = 100;
    progress.completed_at = Some(timestamp());
    progress.failed_at = None;
    progress.cancelled_at = None;
    Ok(())
}

fn fail_import_progress(
    progress: &mut ProjectImportAnalysisProgress,
    error: impl Into<String>,
) -> Result<(), AppError> {
    validate_import_progress(progress)?;
    let error = error.into();
    progress.stages.generating_analysis_data.status = ImportAnalysisStageStatus::Failed;
    progress.status = ImportAnalysisStatus::Failed;
    progress.message = format!("导入分析失败: {error}");
    progress.error = Some(error);
    progress.completed_at = None;
    progress.failed_at = Some(timestamp());
    progress.cancelled_at = None;
    Ok(())
}

fn cancel_progress(progress: &mut ProjectImportAnalysisProgress) -> Result<(), AppError> {
    validate_import_progress(progress)?;
    if progress.status != ImportAnalysisStatus::Analyzing {
        return Err(AppError::InvalidState {
            current: format!("{:?}", progress.status),
            operation: "取消导入分析".to_string(),
        });
    }
    progress.status = ImportAnalysisStatus::Cancelled;
    progress.message = "导入分析已取消".to_string();
    progress.error = None;
    progress.completed_at = None;
    progress.failed_at = None;
    progress.cancelled_at = Some(timestamp());
    Ok(())
}

pub(super) async fn load_import_progress_optional(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<Option<ProjectImportAnalysisProgress>, AppError> {
    let Some(raw) = context.project_import_analysis_progress(project_id).await? else {
        return Ok(None);
    };
    let progress =
        serde_json::from_str::<ProjectImportAnalysisProgress>(&raw).map_err(|error| {
            AppError::InvalidState {
                current: format!("project_id={project_id}"),
                operation: format!("读取导入分析进度合同失败: {error}"),
            }
        })?;
    validate_import_progress(&progress)?;
    Ok(Some(progress))
}

pub(super) async fn load_import_progress(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<ProjectImportAnalysisProgress, AppError> {
    load_import_progress_optional(context, project_id)
        .await?
        .ok_or_else(|| AppError::InvalidState {
            current: format!("project_id={project_id}"),
            operation: "读取尚未创建的导入分析进度".to_string(),
        })
}

pub(super) async fn save_import_progress(
    context: &OrchestratorContext,
    project_id: i64,
    progress: &ProjectImportAnalysisProgress,
) -> Result<(), AppError> {
    validate_import_progress(progress)?;
    let serialized = serde_json::to_string(progress).map_err(AppError::Serialization)?;
    context
        .save_project_import_analysis_progress(project_id, serialized)
        .await
}

pub(super) async fn save_failed_import_progress(
    context: &OrchestratorContext,
    project_id: i64,
    error: &str,
) -> Result<(), AppError> {
    let mut progress = load_import_progress(context, project_id).await?;
    fail_import_progress(&mut progress, error)?;
    save_import_progress(context, project_id, &progress).await
}

pub(super) async fn cancel_import_progress(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<(), AppError> {
    let mut progress = load_import_progress(context, project_id).await?;
    cancel_progress(&mut progress)?;
    save_import_progress(context, project_id, &progress).await
}

pub(super) async fn import_analysis_cancelled(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<bool, AppError> {
    Ok(load_import_progress(context, project_id).await?.status == ImportAnalysisStatus::Cancelled)
}

fn validate_import_progress(progress: &ProjectImportAnalysisProgress) -> Result<(), AppError> {
    let stage = &progress.stages.generating_analysis_data;
    if stage.name != ANALYSIS_STAGE_NAME {
        return Err(invalid_progress(format!(
            "导入分析阶段名称无效: {:?}",
            stage.name
        )));
    }
    if stage.total == 0 {
        return Err(invalid_progress("导入分析章节总数必须大于 0"));
    }
    if stage.completed > stage.total {
        return Err(invalid_progress(format!(
            "导入分析已完成章节数不能大于总数: completed={}, total={}",
            stage.completed, stage.total
        )));
    }
    let expected_percent = progress_percent(stage.completed, stage.total)?;
    if progress.overall_progress != expected_percent {
        return Err(invalid_progress(format!(
            "导入分析进度百分比不一致: expected={expected_percent}, actual={}",
            progress.overall_progress
        )));
    }
    if progress.message.trim().is_empty() || progress.started_at.trim().is_empty() {
        return Err(invalid_progress("导入分析进度缺少消息或开始时间"));
    }
    match progress.status {
        ImportAnalysisStatus::Analyzing => {
            if !matches!(
                stage.status,
                ImportAnalysisStageStatus::Pending
                    | ImportAnalysisStageStatus::InProgress
                    | ImportAnalysisStageStatus::Completed
            ) || progress.error.is_some()
                || progress.completed_at.is_some()
                || progress.failed_at.is_some()
                || progress.cancelled_at.is_some()
            {
                return Err(invalid_progress("导入分析运行态字段不一致"));
            }
        }
        ImportAnalysisStatus::Completed => {
            if stage.status != ImportAnalysisStageStatus::Completed
                || stage.completed != stage.total
                || progress.completed_at.is_none()
                || progress.error.is_some()
                || progress.failed_at.is_some()
                || progress.cancelled_at.is_some()
            {
                return Err(invalid_progress("导入分析完成态字段不一致"));
            }
        }
        ImportAnalysisStatus::Failed => {
            if stage.status != ImportAnalysisStageStatus::Failed
                || progress.error.as_deref().is_none_or(str::is_empty)
                || progress.failed_at.is_none()
                || progress.completed_at.is_some()
                || progress.cancelled_at.is_some()
            {
                return Err(invalid_progress("导入分析失败态字段不一致"));
            }
        }
        ImportAnalysisStatus::Cancelled => {
            if matches!(stage.status, ImportAnalysisStageStatus::Failed)
                || progress.cancelled_at.is_none()
                || progress.error.is_some()
                || progress.completed_at.is_some()
                || progress.failed_at.is_some()
            {
                return Err(invalid_progress("导入分析取消态字段不一致"));
            }
        }
    }
    Ok(())
}

fn progress_percent(completed: usize, total: usize) -> Result<u8, AppError> {
    if total == 0 || completed > total {
        return Err(invalid_progress(format!(
            "导入分析计数无效: completed={completed}, total={total}"
        )));
    }
    Ok(((completed as u128 * 100) / total as u128) as u8)
}

fn invalid_progress(operation: impl Into<String>) -> AppError {
    AppError::InvalidState {
        current: "import_project_analysis_progress".to_string(),
        operation: operation.into(),
    }
}

fn timestamp() -> String {
    Utc::now().naive_utc().to_string()
}

#[cfg(test)]
mod tests;
