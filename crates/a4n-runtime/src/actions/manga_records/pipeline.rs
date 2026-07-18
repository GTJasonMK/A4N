use crate::{AppState, actions::access};
use a4n_orchestrator::manga::{
    MANGA_PAGE_PROMPT_TYPE, PIPELINE_STATUS_PROMPT_TYPE, ensure_manga_source_can_write,
    manga_checkpoint_matches_source,
};

use super::{
    PipelineStatusSummary,
    layout::{
        optional_layout_string, required_layout_bool, required_layout_i32, required_layout_string,
    },
};

pub async fn get_manga_pipeline_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<PipelineStatusSummary, String> {
    access::verify_project_or_derived_family_access(state, user_id, project_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    let status_rows = state
        .orchestrator
        .manga_pipeline_status_rows(
            project_id,
            chapter_id,
            MANGA_PAGE_PROMPT_TYPE,
            PIPELINE_STATUS_PROMPT_TYPE,
        )
        .await
        .map_err(|err| err.to_string())?;

    let mut pending = 0;
    let mut generating = 0;
    let mut completed = 0;
    let mut failed = 0;
    let mut cancelled = 0;
    for prompt in &status_rows.page_prompts {
        match prompt.status.as_str() {
            "pending" => pending += 1,
            "generating" => generating += 1,
            "completed" => completed += 1,
            "failed" => failed += 1,
            "cancelled" => cancelled += 1,
            _ => {}
        }
    }

    let layout = match status_rows.status_row.as_ref() {
        Some(row) => Some(
            row.layout
                .as_ref()
                .ok_or_else(|| format!("漫画流水线状态 #{} 缺少 layout", row.id))?,
        ),
        None => None,
    };
    let pipeline_status = layout
        .map(|value| required_layout_string(value, "pipeline_status"))
        .transpose()?;
    let stage = layout
        .map(|value| required_layout_string(value, "stage"))
        .transpose()?;
    let current = layout
        .map(|value| required_layout_i32(value, "current"))
        .transpose()?;
    let total = layout
        .map(|value| required_layout_i32(value, "total"))
        .transpose()?;
    let message = layout
        .map(|value| required_layout_string(value, "message"))
        .transpose()?;
    let task_id = match layout {
        Some(value) => optional_layout_string(value, "task_id")?,
        None => None,
    };
    let has_checkpoint = match layout.and_then(|value| value.get("checkpoint_data")) {
        Some(value) => value
            .as_object()
            .map(|map| !map.is_empty())
            .ok_or_else(|| "漫画流水线 checkpoint_data 必须是对象".to_string())?,
        None => false,
    };
    let checkpoint_valid = if has_checkpoint {
        let source = state
            .orchestrator
            .chapter_manga_source_by_id(chapter_id, project_id, false)
            .await
            .map_err(|err| err.to_string())?;
        let layout =
            layout.ok_or_else(|| "漫画流水线断点缺少来源元数据所在的 layout".to_string())?;
        manga_checkpoint_matches_source(layout, &source).map_err(|err| err.to_string())?
    } else {
        false
    };
    let can_cancel = match layout {
        Some(value) => required_layout_bool(value, "can_cancel")?,
        None => false,
    };
    let can_resume = match layout {
        Some(value) => required_layout_bool(value, "can_resume")?,
        None => false,
    };

    Ok(PipelineStatusSummary {
        chapter_id,
        pending,
        generating,
        completed,
        failed,
        cancelled,
        pipeline_status,
        stage,
        current,
        total,
        message,
        task_id,
        can_cancel,
        can_resume: can_resume && checkpoint_valid,
        has_checkpoint,
        checkpoint_valid,
    })
}

pub async fn cancel_manga_pipeline(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<i64, String> {
    access::verify_project_or_derived_family_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    let source = state
        .orchestrator
        .chapter_manga_source_by_id(chapter_id, project_id, false)
        .await
        .map_err(|err| err.to_string())?;
    ensure_manga_source_can_write(&source, "取消漫画流水线").map_err(|err| err.to_string())?;
    state
        .cancel_manga_pipeline(project_id, chapter_id)
        .await
        .map_err(|err| err.to_string())
}
