use crate::AppState;
use crate::actions::access as project_access;
use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter_outline, part_outline};
use std::collections::HashSet;

pub(in crate::actions::outline_records) fn ensure_chapter_outline_belongs_to_project(
    outline: &chapter_outline::Model,
    project_id: i64,
) -> Result<(), AppError> {
    if outline.project_id == project_id {
        return Ok(());
    }
    Err(AppError::Forbidden(format!(
        "章节大纲 {} 不属于项目 {}",
        outline.id, project_id
    )))
}

pub(in crate::actions::outline_records) async fn ensure_chapter_outline_access(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    outline_id: i64,
) -> Result<chapter_outline::Model, AppError> {
    project_access::verify_project_editor(state, user_id, project_id).await?;
    let outline = state
        .orchestrator
        .chapter_outline_by_id(outline_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("章节大纲不存在: {outline_id}")))?;
    ensure_chapter_outline_belongs_to_project(&outline, project_id)?;
    Ok(outline)
}

pub(in crate::actions::outline_records) async fn ensure_part_access(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    part_id: i64,
) -> Result<part_outline::Model, AppError> {
    project_access::verify_project_editor(state, user_id, project_id).await?;
    let part = state
        .orchestrator
        .part_outline_by_id(part_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("分部大纲不存在: {part_id}")))?;
    if part.project_id != project_id {
        return Err(AppError::Forbidden(format!(
            "分部大纲 {part_id} 不属于项目 {project_id}"
        )));
    }
    Ok(part)
}

pub(in crate::actions::outline_records) async fn collect_unique_parts(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    part_ids: &[i64],
) -> Result<Vec<part_outline::Model>, AppError> {
    if part_ids.is_empty() {
        return Err(AppError::InvalidArgument("part_ids 不能为空".to_string()));
    }
    let mut seen = HashSet::new();
    let mut parts = Vec::with_capacity(part_ids.len());
    for part_id in part_ids {
        if !seen.insert(*part_id) {
            return Err(AppError::InvalidArgument(format!(
                "part_ids 包含重复分部: {part_id}"
            )));
        }
        parts.push(ensure_part_access(state, user_id, project_id, *part_id).await?);
    }
    order_parts_for_batch_generation(&mut parts);
    Ok(parts)
}

pub(in crate::actions::outline_records) fn order_parts_for_batch_generation(
    parts: &mut [part_outline::Model],
) {
    parts.sort_by_key(|part| (part.part_number, part.id));
}
