use a4n_core::{
    error::AppError,
    manga::{MangaPagePlanV3, MangaPanelV2, MangaTextLayerKindV2},
};
use serde_json::json;

use super::super::output_sync::MangaPromptDraft;
use super::super::reference_assets::page_reference_bindings;
use super::super::{MANGA_PAGE_PROMPT_TYPE, MANGA_PANEL_PLAN_TYPE};
use super::contracts::MangaPipelinePersistInput;

pub(super) fn pipeline_drafts(
    input: &MangaPipelinePersistInput<'_>,
) -> Result<Vec<MangaPromptDraft>, AppError> {
    let plan: MangaPagePlanV3 = serde_json::from_value(input.plan_json.clone())?;
    plan.validate()
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
    let mut drafts = Vec::new();
    for page in &plan.pages {
        let page_number = i32::try_from(page.page_number).map_err(|_| {
            AppError::InvalidArgument(format!("漫画页码超出 i32 范围: {}", page.page_number))
        })?;
        let drawing = input
            .page_prompts
            .iter()
            .find(|prompt| prompt.page_number == page_number)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "第 {} 页缺少 manga_page_drawing_prompt_v2 输出",
                    page.page_number
                ))
            })?;
        let generation_trace = input
            .page_traces
            .iter()
            .find(|trace| trace.page_number == page_number)
            .map(|trace| trace.trace.clone());
        let page_json = serde_json::to_value(page)?;
        drafts.push(MangaPromptDraft {
            chapter_id: input.chapter_id,
            project_id: input.project_id,
            page_number,
            panel_number: 0,
            prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
            image_prompt: drawing.image_prompt.clone(),
            dialogue_text: page_dialogue_text(page),
            characters_in_frame: Some(json!(page_characters(page))),
            layout: Some(json!({
                "schema_version": "manga_page_generation_v2",
                "shooting_script_id": plan.shooting_script_id,
                "shooting_script_revision": plan.shooting_script_revision,
                "page_plan": page,
                "drawing_prompt": drawing.raw,
                "reference_assets": input.reference_assets_metadata,
                "reference_bindings": page_reference_bindings(
                    &page_json,
                    input.reference_assets_metadata,
                ),
                "prompt_fingerprint": input.prompt_fingerprint,
                "source": input.source_metadata,
            })),
            camera_angle: None,
            dialogue_position: None,
            status: "pending".to_string(),
            generation_traces: generation_trace.into_iter().collect(),
        });
        for panel in &page.panels {
            drafts.push(panel_draft(input, &plan, page_number, panel)?);
        }
    }
    Ok(drafts)
}

fn panel_draft(
    input: &MangaPipelinePersistInput<'_>,
    plan: &MangaPagePlanV3,
    page_number: i32,
    panel: &MangaPanelV2,
) -> Result<MangaPromptDraft, AppError> {
    let panel_number = i32::try_from(panel.panel_number).map_err(|_| {
        AppError::InvalidArgument(format!("漫画格号超出 i32 范围: {}", panel.panel_number))
    })?;
    let camera_angle_value = serde_json::to_value(panel.camera_angle)?;
    let camera_angle = camera_angle_value.as_str().ok_or_else(|| {
        AppError::InvalidArgument("漫画格 camera_angle 枚举序列化结果不是字符串".to_string())
    })?;
    Ok(MangaPromptDraft {
        chapter_id: input.chapter_id,
        project_id: input.project_id,
        page_number,
        panel_number,
        prompt_type: MANGA_PANEL_PLAN_TYPE.to_string(),
        image_prompt: format!("{}；{}", panel.composition, panel.observable_action),
        dialogue_text: panel_dialogue_text(panel),
        characters_in_frame: Some(json!(panel.participants)),
        layout: Some(json!({
            "schema_version": "manga_panel_plan_v2",
            "shooting_script_id": plan.shooting_script_id,
            "shooting_script_revision": plan.shooting_script_revision,
            "panel_plan": panel,
            "source": input.source_metadata,
        })),
        camera_angle: Some(camera_angle.to_string()),
        dialogue_position: None,
        status: "completed".to_string(),
        generation_traces: Vec::new(),
    })
}

fn page_characters(page: &a4n_core::manga::MangaPageV3) -> Vec<String> {
    let mut names = page
        .panels
        .iter()
        .flat_map(|panel| panel.participants.iter().cloned())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn page_dialogue_text(page: &a4n_core::manga::MangaPageV3) -> Option<String> {
    let text = page
        .panels
        .iter()
        .filter_map(panel_dialogue_text)
        .collect::<Vec<_>>()
        .join("\n");
    (!text.is_empty()).then_some(text)
}

fn panel_dialogue_text(panel: &MangaPanelV2) -> Option<String> {
    let text = panel
        .text_layers
        .iter()
        .map(|layer| match layer.kind {
            MangaTextLayerKindV2::Dialogue | MangaTextLayerKindV2::Thought => {
                format!("{}：{}", layer.speaker.as_deref().unwrap_or(""), layer.text)
            }
            MangaTextLayerKindV2::Narration => format!("旁白：{}", layer.text),
            MangaTextLayerKindV2::Sfx => format!("音效：{}", layer.text),
        })
        .collect::<Vec<_>>()
        .join(" ");
    (!text.is_empty()).then_some(text)
}
