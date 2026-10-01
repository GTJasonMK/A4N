//! Pure Manga page layout validation and API projection.

use a4n_api_contract::manga::MangaPagePromptSummary;
use a4n_domain::manga::{
    MangaPageGenerationLayoutV2, MangaPagePromptRecord, MangaPortraitReference,
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
        "source": source_metadata(ChapterSourceContext { content }),
        "reference_assets": manga_reference_assets_json(character_portraits),
    })
}

pub fn manga_page_generation_layout(
    prompt_id: i64,
    page_number: i32,
    layout_json: Option<&Value>,
) -> Result<MangaPageGenerationLayoutV2, String> {
    let layout: MangaPageGenerationLayoutV2 = serde_json::from_value(
        layout_json
            .cloned()
            .ok_or_else(|| format!("漫画页面提示词 #{prompt_id} 缺少 v2 layout"))?,
    )
    .map_err(|error| format!("漫画页面提示词 #{prompt_id} 的 v2 layout 无效: {error}"))?;
    let page_number = u32::try_from(page_number)
        .ok()
        .filter(|page_number| *page_number > 0)
        .ok_or_else(|| format!("漫画页面提示词 #{prompt_id} 的 page_number 必须大于 0"))?;
    layout
        .validate(page_number)
        .map_err(|error| format!("漫画页面提示词 #{prompt_id} 的 v2 layout 无效: {error}"))?;
    Ok(layout)
}

pub fn manga_page_prompt_summary(
    prompt: &MangaPagePromptRecord,
) -> Result<MangaPagePromptSummary, String> {
    let layout =
        manga_page_generation_layout(prompt.id, prompt.page_number, prompt.layout.as_ref())?;
    Ok(MangaPagePromptSummary {
        id: prompt.id,
        chapter_id: prompt.chapter_id,
        project_id: prompt.project_id,
        page_number: prompt.page_number,
        image_prompt: prompt.image_prompt.clone(),
        dialogue_text: prompt.dialogue_text.clone(),
        page_plan: layout.page_plan,
        drawing_prompt: layout.drawing_prompt,
        reference_bindings: layout.reference_bindings,
        source: layout.source,
        generated_image_id: prompt.generated_image_id,
        status: prompt.status.clone(),
        created_at: prompt.created_at.to_string(),
        updated_at: prompt.updated_at.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::manga::MangaPagePromptRecord;
    use chrono::NaiveDateTime;

    #[test]
    fn rejects_missing_layout() {
        let now =
            NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let prompt = MangaPagePromptRecord {
            id: 11,
            chapter_id: 22,
            project_id: 33,
            prompt_type: a4n_domain::manga::MANGA_PAGE_PROMPT_TYPE.to_string(),
            page_number: 1,
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
            "漫画页面提示词 #11 缺少 v2 layout"
        );
    }
}
