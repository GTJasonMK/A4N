//! Pure Manga page layout validation and API projection.

use a4n_api_contract::manga::MangaPagePromptSummary;
use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_domain::manga::{
    MangaPageGenerationLayoutV3, MangaPageGenerationLayoutV4, MangaPagePromptRecord,
    MangaPortraitReference,
};
use a4n_domain::source_spans::{ChapterSourceContext, source_metadata};
use serde_json::{Value, json};

struct MangaPagePromptView<'a> {
    id: i64,
    chapter_id: i64,
    project_id: i64,
    page_number: i32,
    image_prompt: &'a str,
    dialogue_text: &'a Option<String>,
    layout: Option<&'a Value>,
    generated_image_id: Option<i64>,
    status: &'a str,
    created_at: String,
    updated_at: String,
}

impl<'a> From<&'a chapter_manga_prompt::Model> for MangaPagePromptView<'a> {
    fn from(prompt: &'a chapter_manga_prompt::Model) -> Self {
        Self {
            id: prompt.id,
            chapter_id: prompt.chapter_id,
            project_id: prompt.project_id,
            page_number: prompt.page_number,
            image_prompt: &prompt.image_prompt,
            dialogue_text: &prompt.dialogue_text,
            layout: prompt.layout.as_ref(),
            generated_image_id: prompt.generated_image_id,
            status: &prompt.status,
            created_at: prompt.created_at.to_string(),
            updated_at: prompt.updated_at.to_string(),
        }
    }
}

impl<'a> From<&'a MangaPagePromptRecord> for MangaPagePromptView<'a> {
    fn from(prompt: &'a MangaPagePromptRecord) -> Self {
        Self {
            id: prompt.id,
            chapter_id: prompt.chapter_id,
            project_id: prompt.project_id,
            page_number: prompt.page_number,
            image_prompt: &prompt.image_prompt,
            dialogue_text: &prompt.dialogue_text,
            layout: prompt.layout.as_ref(),
            generated_image_id: prompt.generated_image_id,
            status: &prompt.status,
            created_at: prompt.created_at.to_string(),
            updated_at: prompt.updated_at.to_string(),
        }
    }
}

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
    manga_page_generation_layout_from_parts(prompt.id, prompt.page_number, prompt.layout.as_ref())
}

fn manga_page_generation_layout_from_parts(
    prompt_id: i64,
    page_number: i32,
    layout: Option<&Value>,
) -> Result<MangaPageGenerationLayoutV3, String> {
    let page_number = u32::try_from(page_number)
        .ok()
        .filter(|page_number| *page_number > 0)
        .ok_or_else(|| format!("漫画页面提示词 #{prompt_id} 的 page_number 必须大于 0"))?;
    let value = layout
        .cloned()
        .ok_or_else(|| format!("漫画页面提示词 #{prompt_id} 缺少 layout"))?;
    if value.get("schema_version").and_then(Value::as_str) == Some("manga_page_generation_v4") {
        let layout: MangaPageGenerationLayoutV4 = serde_json::from_value(value)
            .map_err(|error| format!("漫画页面提示词 #{prompt_id} 的 v4 layout 无效: {error}"))?;
        layout
            .validate(page_number)
            .map_err(|error| format!("漫画页面提示词 #{prompt_id} 的 v4 layout 无效: {error}"))?;
        let page_plan = layout
            .pipeline
            .storyboard
            .ok_or_else(|| format!("漫画页面提示词 #{prompt_id} 尚未完成 storyboard"))?;
        let drawing_prompt = layout
            .pipeline
            .drawing_prompt
            .ok_or_else(|| format!("漫画页面提示词 #{prompt_id} 尚未完成 drawing_prompt"))?;
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
        .map_err(|error| format!("漫画页面提示词 #{prompt_id} 的 v3 layout 无效: {error}"))?;
    layout
        .validate(page_number)
        .map_err(|error| format!("漫画页面提示词 #{prompt_id} 的 v3 layout 无效: {error}"))?;
    Ok(layout)
}

pub fn manga_page_pipeline_layout(
    prompt: &chapter_manga_prompt::Model,
) -> Result<MangaPageGenerationLayoutV4, String> {
    manga_page_pipeline_layout_from_parts(prompt.id, prompt.page_number, prompt.layout.as_ref())
}

fn manga_page_pipeline_layout_from_parts(
    prompt_id: i64,
    page_number: i32,
    layout: Option<&Value>,
) -> Result<MangaPageGenerationLayoutV4, String> {
    let layout: MangaPageGenerationLayoutV4 = serde_json::from_value(
        layout
            .cloned()
            .ok_or_else(|| format!("漫画页面提示词 #{prompt_id} 缺少 v4 layout"))?,
    )
    .map_err(|error| format!("漫画页面提示词 #{prompt_id} 的 v4 layout 无效: {error}"))?;
    let page_number = u32::try_from(page_number)
        .ok()
        .filter(|page_number| *page_number > 0)
        .ok_or_else(|| format!("漫画页面提示词 #{prompt_id} 的 page_number 必须大于 0"))?;
    layout
        .validate(page_number)
        .map_err(|error| format!("漫画页面提示词 #{prompt_id} 的 v4 layout 无效: {error}"))?;
    Ok(layout)
}

pub fn manga_page_prompt_summary(
    prompt: &chapter_manga_prompt::Model,
) -> Result<MangaPagePromptSummary, String> {
    manga_page_prompt_summary_from_view(prompt.into())
}

pub fn manga_page_prompt_record_summary(
    prompt: &MangaPagePromptRecord,
) -> Result<MangaPagePromptSummary, String> {
    manga_page_prompt_summary_from_view(prompt.into())
}

fn manga_page_prompt_summary_from_view(
    prompt: MangaPagePromptView<'_>,
) -> Result<MangaPagePromptSummary, String> {
    if let Ok(layout) =
        manga_page_pipeline_layout_from_parts(prompt.id, prompt.page_number, prompt.layout)
    {
        return Ok(MangaPagePromptSummary {
            id: prompt.id,
            chapter_id: prompt.chapter_id,
            project_id: prompt.project_id,
            page_number: prompt.page_number,
            page_id: Some(layout.page_id),
            image_prompt: prompt.image_prompt.to_string(),
            dialogue_text: (*prompt.dialogue_text).clone(),
            page_content: layout.pipeline.page_content,
            page_adaptation: layout.pipeline.page_adaptation,
            page_plan: layout.pipeline.storyboard,
            drawing_prompt: layout.pipeline.drawing_prompt,
            reference_bindings: layout.reference_bindings,
            source: layout.source,
            generated_image_id: prompt.generated_image_id,
            status: prompt.status.to_string(),
            pipeline_stage: Some(layout.pipeline.stage),
            pipeline_status: Some(layout.pipeline.status),
            pipeline_error: layout.pipeline.error,
            created_at: prompt.created_at,
            updated_at: prompt.updated_at,
        });
    }
    let layout =
        manga_page_generation_layout_from_parts(prompt.id, prompt.page_number, prompt.layout)?;
    Ok(MangaPagePromptSummary {
        id: prompt.id,
        chapter_id: prompt.chapter_id,
        project_id: prompt.project_id,
        page_number: prompt.page_number,
        page_id: None,
        image_prompt: prompt.image_prompt.to_string(),
        dialogue_text: (*prompt.dialogue_text).clone(),
        page_content: None,
        page_adaptation: None,
        page_plan: Some(layout.page_plan),
        drawing_prompt: Some(layout.drawing_prompt),
        reference_bindings: layout.reference_bindings,
        source: layout.source,
        generated_image_id: prompt.generated_image_id,
        status: prompt.status.to_string(),
        pipeline_stage: None,
        pipeline_status: None,
        pipeline_error: None,
        created_at: prompt.created_at,
        updated_at: prompt.updated_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_db_models::models::chapter::chapter_manga_prompt::MANGA_PAGE_PROMPT_TYPE;
    use a4n_domain::manga::{MangaPagePipelineStage, MangaPagePipelineStatus};
    use chrono::NaiveDateTime;
    use serde_json::json;

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

    #[test]
    fn record_summary_preserves_v4_cancelled_pipeline_and_source() {
        let now =
            NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let prompt = MangaPagePromptRecord {
            id: 11,
            chapter_id: 22,
            project_id: 33,
            prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
            page_number: 1,
            image_prompt: "整页提示词".to_string(),
            dialogue_text: Some("旁白".to_string()),
            characters_in_frame: None,
            layout: Some(json!({
                "schema_version": "manga_page_generation_v4",
                "page_id": "page-0001",
                "page_number": 1,
                "page_plan_ref": null,
                "pipeline": {
                    "schema_version": "manga_page_pipeline_state_v1",
                    "page_id": "page-0001",
                    "status": "cancelled",
                    "stage": "adaptation",
                    "attempt": 1,
                    "source_fingerprint": "content-sha256",
                    "stage_fingerprints": {},
                    "page_content": {
                        "summary": "本页内容",
                        "source_span_ids": [],
                        "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
                        "action": "角色抬头",
                        "reaction": null,
                        "dialogue_intent": null,
                        "entry_state": "平静",
                        "exit_state": "紧张"
                    },
                    "page_adaptation": null,
                    "storyboard": null,
                    "drawing_prompt": null,
                    "image": null,
                    "error": {
                        "stage": "adaptation",
                        "code": "cancelled",
                        "message": "用户取消",
                        "retryable": false
                    }
                },
                "reference_bindings": [],
                "prompt_fingerprint": {},
                "source": {
                    "root_project_id": 33,
                    "source_chapter_id": 22,
                    "source_snapshot_id": null,
                    "source_snapshot_chapter_id": null,
                    "source_revision_key": "live:22:content-sha256",
                    "content_sha256": "content-sha256",
                    "adaptation_plan_id": 3,
                    "adaptation_plan_revision": 1,
                    "adaptation_plan_sha256": "plan-sha256"
                }
            })),
            generated_image_id: None,
            status: "cancelled".to_string(),
            created_at: now,
            updated_at: now,
        };

        let summary = manga_page_prompt_record_summary(&prompt).unwrap();
        assert_eq!(summary.page_id.as_deref(), Some("page-0001"));
        assert_eq!(
            summary.pipeline_stage,
            Some(MangaPagePipelineStage::Adaptation)
        );
        assert_eq!(
            summary.pipeline_status,
            Some(MangaPagePipelineStatus::Cancelled)
        );
        assert_eq!(
            summary
                .pipeline_error
                .as_ref()
                .map(|error| error.code.as_str()),
            Some("cancelled")
        );
        assert_eq!(summary.source.root_project_id, 33);
        assert_eq!(summary.source.source_chapter_id, 22);
        assert_eq!(summary.page_content.as_ref().unwrap().summary, "本页内容");

        let model = chapter_manga_prompt::Model {
            id: prompt.id,
            chapter_id: prompt.chapter_id,
            project_id: prompt.project_id,
            page_number: prompt.page_number,
            prompt_type: prompt.prompt_type.clone(),
            image_prompt: prompt.image_prompt.clone(),
            dialogue_text: prompt.dialogue_text.clone(),
            characters_in_frame: prompt.characters_in_frame.clone(),
            layout: prompt.layout.clone(),
            generated_image_id: prompt.generated_image_id,
            status: prompt.status.clone(),
            created_at: prompt.created_at,
            updated_at: prompt.updated_at,
        };
        assert_eq!(summary, manga_page_prompt_summary(&model).unwrap());
    }
}
