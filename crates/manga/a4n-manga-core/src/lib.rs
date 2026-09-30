//! Pure Manga page layout validation and API projection.

use a4n_api_contract::manga::MangaPagePromptSummary;
use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_domain::manga::{
    MangaPageGenerationLayoutV3, MangaPageGenerationLayoutV4, MangaPortraitReference,
};
use a4n_domain::source_spans::{ChapterSourceContext, source_metadata};
use serde_json::{Value, json};

pub fn manga_reference_assets_json<P: MangaPortraitReference>(refs: &[P]) -> Value {
    json!({
        "character_portraits": refs.iter().map(|portrait| json!({
            "profile_id": portrait.profile_id(),
            "portrait_id": portrait.portrait_id(),
            "character_name": portrait.character_name(),
            "image_path": portrait.image_path(),
            "prompt": portrait.prompt(),
            "portrait_type": portrait.portrait_type(),
            "expression": portrait.expression(),
            "pose": portrait.pose(),
            "is_primary": true,
        })).collect::<Vec<_>>(),
    })
}

pub fn manga_pipeline_fingerprint<P: MangaPortraitReference>(
    content: &str,
    character_portraits: &[P],
) -> Value {
    json!({
        "authoring_contract": a4n_domain::media_authoring::MEDIA_AUTHORING_PIPELINE_VERSION,
        "source": source_metadata(ChapterSourceContext { content }),
        "reference_assets": manga_reference_assets_json(character_portraits),
    })
}

pub fn manga_page_generation_layout(
    prompt: &chapter_manga_prompt::Model,
) -> Result<MangaPageGenerationLayoutV3, String> {
    let page_number = u32::try_from(prompt.page_number)
        .ok()
        .filter(|page_number| *page_number > 0)
        .ok_or_else(|| format!("漫画页面提示词 #{} 的 page_number 必须大于 0", prompt.id))?;
    let value = prompt
        .layout
        .clone()
        .ok_or_else(|| format!("漫画页面提示词 #{} 缺少 layout", prompt.id))?;
    if value.get("schema_version").and_then(Value::as_str) == Some("manga_page_generation_v4") {
        let layout: MangaPageGenerationLayoutV4 = serde_json::from_value(value)
            .map_err(|error| format!("漫画页面提示词 #{} 的 v4 layout 无效: {error}", prompt.id))?;
        layout
            .validate(page_number)
            .map_err(|error| format!("漫画页面提示词 #{} 的 v4 layout 无效: {error}", prompt.id))?;
        let page_plan = layout
            .pipeline
            .storyboard
            .ok_or_else(|| format!("漫画页面提示词 #{} 尚未完成 storyboard", prompt.id))?;
        let drawing_prompt = layout
            .pipeline
            .drawing_prompt
            .ok_or_else(|| format!("漫画页面提示词 #{} 尚未完成 drawing_prompt", prompt.id))?;
        return Ok(MangaPageGenerationLayoutV3 {
            schema_version: a4n_domain::manga::MangaPageGenerationSchemaVersion::V3,
            page_plan,
            page_plan_ref: layout.page_plan_ref,
            drawing_prompt,
            reference_bindings: layout.reference_bindings,
            prompt_fingerprint: layout.prompt_fingerprint,
            source: layout.source,
        });
    }
    let layout: MangaPageGenerationLayoutV3 = serde_json::from_value(value)
        .map_err(|error| format!("漫画页面提示词 #{} 的 v3 layout 无效: {error}", prompt.id))?;
    layout
        .validate(page_number)
        .map_err(|error| format!("漫画页面提示词 #{} 的 v3 layout 无效: {error}", prompt.id))?;
    Ok(layout)
}

pub fn manga_page_pipeline_layout(
    prompt: &chapter_manga_prompt::Model,
) -> Result<MangaPageGenerationLayoutV4, String> {
    let layout: MangaPageGenerationLayoutV4 = serde_json::from_value(
        prompt
            .layout
            .clone()
            .ok_or_else(|| format!("漫画页面提示词 #{} 缺少 v4 layout", prompt.id))?,
    )
    .map_err(|error| format!("漫画页面提示词 #{} 的 v4 layout 无效: {error}", prompt.id))?;
    let page_number = u32::try_from(prompt.page_number)
        .ok()
        .filter(|page_number| *page_number > 0)
        .ok_or_else(|| format!("漫画页面提示词 #{} 的 page_number 必须大于 0", prompt.id))?;
    layout
        .validate(page_number)
        .map_err(|error| format!("漫画页面提示词 #{} 的 v4 layout 无效: {error}", prompt.id))?;
    Ok(layout)
}

pub fn manga_page_prompt_summary(
    prompt: &chapter_manga_prompt::Model,
) -> Result<MangaPagePromptSummary, String> {
    if let Ok(layout) = manga_page_pipeline_layout(prompt) {
        return Ok(MangaPagePromptSummary {
            id: prompt.id,
            chapter_id: prompt.chapter_id,
            project_id: prompt.project_id,
            page_number: prompt.page_number,
            page_id: Some(layout.page_id),
            image_prompt: prompt.image_prompt.clone(),
            dialogue_text: prompt.dialogue_text.clone(),
            page_content: layout.pipeline.page_content,
            page_adaptation: layout.pipeline.page_adaptation,
            page_plan: layout.pipeline.storyboard,
            drawing_prompt: layout.pipeline.drawing_prompt,
            reference_bindings: layout.reference_bindings,
            source: layout.source,
            generated_image_id: prompt.generated_image_id,
            status: prompt.status.clone(),
            pipeline_stage: Some(layout.pipeline.stage),
            pipeline_status: Some(layout.pipeline.status),
            pipeline_error: layout.pipeline.error,
            created_at: prompt.created_at.to_string(),
            updated_at: prompt.updated_at.to_string(),
        });
    }
    let layout = manga_page_generation_layout(prompt)?;
    Ok(MangaPagePromptSummary {
        id: prompt.id,
        chapter_id: prompt.chapter_id,
        project_id: prompt.project_id,
        page_number: prompt.page_number,
        page_id: None,
        image_prompt: prompt.image_prompt.clone(),
        dialogue_text: prompt.dialogue_text.clone(),
        page_content: None,
        page_adaptation: None,
        page_plan: Some(layout.page_plan),
        drawing_prompt: Some(layout.drawing_prompt),
        reference_bindings: layout.reference_bindings,
        source: layout.source,
        generated_image_id: prompt.generated_image_id,
        status: prompt.status.clone(),
        pipeline_stage: None,
        pipeline_status: None,
        pipeline_error: None,
        created_at: prompt.created_at.to_string(),
        updated_at: prompt.updated_at.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_db_models::models::chapter::chapter_manga_prompt::MANGA_PAGE_PROMPT_TYPE;
    use chrono::NaiveDateTime;

    #[test]
    fn rejects_missing_layout() {
        let now =
            NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let prompt = chapter_manga_prompt::Model {
            id: 11,
            chapter_id: 22,
            project_id: 33,
            page_number: 1,
            prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
            image_prompt: "整页提示词".to_string(),
            dialogue_text: None,
            characters_in_frame: None,
            layout: None,
            generated_image_id: None,
            status: "pending".to_string(),
            created_at: now,
            updated_at: now,
        };
        assert_eq!(
            manga_page_prompt_summary(&prompt).unwrap_err(),
            "漫画页面提示词 #11 缺少 layout"
        );
    }
}
