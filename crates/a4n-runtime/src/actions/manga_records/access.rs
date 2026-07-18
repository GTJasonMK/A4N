use crate::{AppState, actions::dependency_conflict::DownstreamConflictStrategy};
use a4n_core::models::chapter::chapter_manga_prompt;
use a4n_orchestrator::manga::MANGA_PAGE_PROMPT_TYPE;

pub(super) async fn load_manga_page_prompt(
    state: &AppState,
    prompt_id: i64,
    not_found_message: &str,
) -> Result<chapter_manga_prompt::Model, String> {
    let panel = state
        .orchestrator
        .manga_page_prompt_by_id(prompt_id)
        .await
        .map_err(crate::actions::app_error_message)?
        .ok_or_else(|| not_found_message.to_string())?;
    if panel.prompt_type != MANGA_PAGE_PROMPT_TYPE {
        return Err("只能操作整页漫画提示词".to_string());
    }
    Ok(panel)
}

pub(super) fn resolve_manga_prompt_downstream_conflict(
    panel: &chapter_manga_prompt::Model,
    strategy: DownstreamConflictStrategy,
    operation: &str,
) -> Result<Option<i64>, String> {
    let Some(image_id) = panel.generated_image_id else {
        return Ok(None);
    };
    match strategy {
        DownstreamConflictStrategy::Reject => Err(format!(
            "整页漫画提示词 #{} 已派生成图片 #{image_id}，{operation}前需要选择：删除下游派生产物，或保留下游只覆盖源提示词",
            panel.id
        )),
        DownstreamConflictStrategy::KeepDownstream => Ok(None),
        DownstreamConflictStrategy::DeleteDownstream => Ok(Some(image_id)),
    }
}

pub(super) fn required_trimmed<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}不能为空"));
    }
    Ok(trimmed)
}
