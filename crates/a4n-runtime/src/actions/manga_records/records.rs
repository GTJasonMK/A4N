use crate::{
    AppState,
    actions::{
        access,
        dependency_conflict::{DownstreamConflictStrategy, delete_downstream_images_if_requested},
    },
};
use a4n_orchestrator::manga::{MANGA_PAGE_PROMPT_TYPE, ensure_manga_source_can_write};

use super::{
    BindReferenceImageInput, MangaPromptSummary, UpdateMangaPromptInput,
    access::{load_manga_page_prompt, required_trimmed, resolve_manga_prompt_downstream_conflict},
    layout::bind_reference_asset_in_layout,
    projections::to_summary,
    reference::resolve_reference_bound_asset,
};

pub async fn list_manga_panels(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<MangaPromptSummary>, String> {
    access::verify_project_or_derived_family_access(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    let panels = state
        .orchestrator
        .manga_page_prompts_by_chapter(project_id, chapter_id, MANGA_PAGE_PROMPT_TYPE)
        .await
        .map_err(crate::actions::app_error_message)?;
    panels.iter().map(to_summary).collect()
}

pub async fn bind_manga_reference_image(
    state: &AppState,
    user_id: i64,
    input: BindReferenceImageInput,
) -> Result<MangaPromptSummary, String> {
    let slot_id = required_trimmed(&input.slot_id, "参考图槽位")?;
    let panel = load_manga_page_prompt(state, input.prompt_id, "整页漫画提示词不存在").await?;
    access::verify_project_or_derived_family_editor(state, user_id, panel.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let source = state
        .orchestrator
        .chapter_manga_source_by_id(panel.chapter_id, panel.project_id, false)
        .await
        .map_err(|err| err.to_string())?;
    ensure_manga_source_can_write(&source, "绑定漫画参考图").map_err(|err| err.to_string())?;
    let downstream_image_id =
        resolve_manga_prompt_downstream_conflict(&panel, input.downstream_strategy, "绑定参考图")?;
    let bound_asset = resolve_reference_bound_asset(
        state,
        panel.project_id,
        input.asset_kind.trim(),
        input.asset_id,
    )
    .await?;
    let layout = bind_reference_asset_in_layout(panel.layout.clone(), slot_id, bound_asset)?;
    let updated = state
        .orchestrator
        .update_manga_page_prompt_layout(panel, layout)
        .await
        .map_err(crate::actions::app_error_message)?;
    delete_downstream_images_if_requested(state, user_id, downstream_image_id).await?;
    to_summary(&updated)
}

pub async fn update_manga_prompt(
    state: &AppState,
    user_id: i64,
    input: UpdateMangaPromptInput,
) -> Result<MangaPromptSummary, String> {
    let image_prompt = required_trimmed(&input.image_prompt, "整页漫画提示词")?;
    let panel = load_manga_page_prompt(state, input.prompt_id, "整页漫画提示词不存在").await?;
    access::verify_project_or_derived_family_editor(state, user_id, panel.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let source = state
        .orchestrator
        .chapter_manga_source_by_id(panel.chapter_id, panel.project_id, false)
        .await
        .map_err(|err| err.to_string())?;
    ensure_manga_source_can_write(&source, "修改整页漫画提示词").map_err(|err| err.to_string())?;
    if panel.image_prompt.trim() == image_prompt {
        return to_summary(&panel);
    }
    let downstream_image_id =
        resolve_manga_prompt_downstream_conflict(&panel, input.downstream_strategy, "修改提示词")?;
    let updated = state
        .orchestrator
        .update_manga_page_prompt_image_prompt(panel, image_prompt.to_string())
        .await
        .map_err(crate::actions::app_error_message)?;
    delete_downstream_images_if_requested(state, user_id, downstream_image_id).await?;
    to_summary(&updated)
}

pub async fn delete_manga_by_chapter(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<(), String> {
    access::verify_project_or_derived_family_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    let source = state
        .orchestrator
        .chapter_manga_source_by_id(chapter_id, project_id, false)
        .await
        .map_err(|err| err.to_string())?;
    ensure_manga_source_can_write(&source, "删除漫画产物").map_err(|err| err.to_string())?;
    state
        .orchestrator
        .delete_manga_prompts_by_chapter(project_id, chapter_id)
        .await
        .map(|_| ())
        .map_err(crate::actions::app_error_message)
}

pub async fn delete_manga_prompt(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    downstream_strategy: DownstreamConflictStrategy,
) -> Result<(), String> {
    let panel = load_manga_page_prompt(state, prompt_id, "整页漫画提示词不存在").await?;
    access::verify_project_or_derived_family_editor(state, user_id, panel.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let source = state
        .orchestrator
        .chapter_manga_source_by_id(panel.chapter_id, panel.project_id, false)
        .await
        .map_err(|err| err.to_string())?;
    ensure_manga_source_can_write(&source, "删除漫画提示词").map_err(|err| err.to_string())?;
    let downstream_image_id =
        resolve_manga_prompt_downstream_conflict(&panel, downstream_strategy, "删除提示词")?;
    delete_downstream_images_if_requested(state, user_id, downstream_image_id).await?;
    state
        .orchestrator
        .delete_manga_prompt_record(prompt_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    Ok(())
}
