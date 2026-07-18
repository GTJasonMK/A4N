use super::schema::{parse_import_progress, recalc_import_progress};
use super::updates::mark_import_progress_failed;
use crate::OrchestratorContext;
use chrono::Utc;
use serde_json::{Value, json};

pub async fn load_import_progress_optional(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<Option<Value>, String> {
    let raw = context
        .project_import_analysis_progress(project_id)
        .await
        .map_err(|error| error.to_string())?;
    let Some(mut progress) = parse_import_progress(project_id, raw.as_deref())? else {
        return Ok(None);
    };
    recalc_import_progress(&mut progress)?;
    Ok(Some(progress))
}

pub async fn save_import_progress(
    context: &OrchestratorContext,
    project_id: i64,
    progress: &Value,
) -> Result<Value, String> {
    context
        .save_project_import_analysis_progress(project_id, progress.to_string())
        .await
        .map_err(|error| error.to_string())?;
    Ok(progress.clone())
}

pub async fn load_import_progress(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<Value, String> {
    load_import_progress_optional(context, project_id)
        .await?
        .ok_or_else(|| format!("项目尚未创建导入分析进度: project_id={project_id}"))
}

pub async fn save_failed_import_progress(
    context: &OrchestratorContext,
    project_id: i64,
    error: &str,
) -> Result<(), String> {
    let mut progress = load_import_progress(context, project_id).await?;
    mark_import_progress_failed(&mut progress, error)?;
    save_import_progress(context, project_id, &progress).await?;
    Ok(())
}

pub async fn cancel_import_progress(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<(), String> {
    let mut progress = load_import_progress(context, project_id).await?;
    progress["status"] = json!("cancelled");
    progress["message"] = json!("导入分析已取消");
    progress["cancelled_at"] = json!(Utc::now().naive_utc().to_string());
    save_import_progress(context, project_id, &progress).await?;
    Ok(())
}

pub async fn import_analysis_cancelled(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<bool, String> {
    let progress = load_import_progress(context, project_id).await?;
    Ok(progress.get("status").and_then(Value::as_str) == Some("cancelled"))
}
