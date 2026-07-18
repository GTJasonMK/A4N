use a4n_core::manga::MANGA_PAGE_DRAWING_PROMPT_V2;
use a4n_core::models::chapter::chapter_manga_prompt;

use super::{MangaPageGenerationLayoutV2, MangaPromptSummary};

pub(super) fn to_summary(
    prompt: &chapter_manga_prompt::Model,
) -> Result<MangaPromptSummary, String> {
    let layout: MangaPageGenerationLayoutV2 = serde_json::from_value(
        prompt
            .layout
            .clone()
            .ok_or_else(|| format!("漫画页面提示词 #{} 缺少 v2 layout", prompt.id))?,
    )
    .map_err(|error| format!("漫画页面提示词 #{} 的 v2 layout 无效: {error}", prompt.id))?;
    if layout.schema_version != "manga_page_generation_v2" {
        return Err(format!(
            "漫画页面提示词 #{} 的 schema_version 必须是 manga_page_generation_v2",
            prompt.id
        ));
    }
    layout
        .page_plan
        .validate()
        .map_err(|error| format!("漫画页面提示词 #{} 的 page_plan 无效: {error}", prompt.id))?;
    if layout.page_plan.page_number != u32::try_from(prompt.page_number).unwrap_or_default()
        || prompt.panel_number != 0
        || layout.page_plan.page_number != layout.drawing_prompt.page_number
    {
        return Err(format!("漫画页面提示词 #{} 的页码身份不一致", prompt.id));
    }
    if layout.drawing_prompt.schema_version != MANGA_PAGE_DRAWING_PROMPT_V2
        || layout.source.shooting_script_id != layout.shooting_script_id
        || layout.source.shooting_script_revision != layout.shooting_script_revision
    {
        return Err(format!(
            "漫画页面提示词 #{} 的拍摄剧本身份不一致",
            prompt.id
        ));
    }
    Ok(MangaPromptSummary {
        id: prompt.id,
        chapter_id: prompt.chapter_id,
        project_id: prompt.project_id,
        page_number: prompt.page_number,
        panel_number: prompt.panel_number,
        prompt_type: prompt.prompt_type.clone(),
        image_prompt: prompt.image_prompt.clone(),
        dialogue_text: prompt.dialogue_text.clone(),
        shooting_script_id: layout.shooting_script_id,
        shooting_script_revision: layout.shooting_script_revision,
        page_plan: layout.page_plan,
        drawing_prompt: layout.drawing_prompt,
        reference_bindings: layout.reference_bindings,
        source: layout.source,
        generated_image_id: prompt.generated_image_id,
        status: prompt.status.clone(),
        created_at: Some(prompt.created_at.to_string()),
        updated_at: Some(prompt.updated_at.to_string()),
    })
}
