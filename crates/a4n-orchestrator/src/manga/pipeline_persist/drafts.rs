use a4n_core::error::AppError;
use a4n_domain::manga::{
    MANGA_PAGE_GENERATION_V2, MangaPageV3, MangaPanelV2, MangaTextLayerKindV2,
};
use serde_json::json;

use super::super::output_sync::MangaPagePromptDraft;
use super::super::reference_assets::page_reference_bindings;
use super::contracts::MangaPipelinePersistInput;

pub(super) fn pipeline_drafts(
    input: &MangaPipelinePersistInput<'_>,
) -> Result<Vec<MangaPagePromptDraft>, AppError> {
    let plan = input.plan;
    let mut drafts = Vec::new();
    for page in &plan.pages {
        let page_number = database_index(page.page_number, "漫画页码")?;
        let drawing = input
            .page_prompts
            .iter()
            .find(|prompt| prompt.page_number == page.page_number)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "第 {} 页缺少 manga_page_drawing_prompt_v2 输出",
                    page.page_number
                ))
            })?;
        let generation_trace = input
            .page_traces
            .iter()
            .find(|trace| trace.page_number == page.page_number)
            .map(|trace| trace.trace.clone());
        drafts.push(MangaPagePromptDraft {
            chapter_id: input.chapter_id,
            project_id: input.project_id,
            page_number,
            image_prompt: drawing.image_prompt.clone(),
            dialogue_text: page_dialogue_text(page)?,
            characters_in_frame: Some(json!(page_characters(page))),
            layout: Some(json!({
                "schema_version": MANGA_PAGE_GENERATION_V2,
                "shooting_script_id": plan.shooting_script_id,
                "shooting_script_revision": plan.shooting_script_revision,
                "page_plan": page,
                "drawing_prompt": drawing,
                "reference_bindings": page_reference_bindings(
                    page,
                    input.reference_assets_metadata,
                ).map_err(AppError::InvalidArgument)?,
                "prompt_fingerprint": input.prompt_fingerprint,
                "source": input.source_metadata,
            })),
            status: "pending".to_string(),
            generation_traces: generation_trace.into_iter().collect(),
        });
    }
    Ok(drafts)
}

fn database_index(value: u32, label: &str) -> Result<i32, AppError> {
    i32::try_from(value)
        .map_err(|_| AppError::InvalidArgument(format!("{label}超出 i32 范围: {value}")))
}

fn page_characters(page: &MangaPageV3) -> Vec<String> {
    let mut names = page
        .panels
        .iter()
        .flat_map(|panel| panel.participants.iter().cloned())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn page_dialogue_text(page: &MangaPageV3) -> Result<Option<String>, AppError> {
    let mut lines = Vec::new();
    for panel in &page.panels {
        if let Some(text) = panel_dialogue_text(panel)? {
            lines.push(text);
        }
    }
    let text = lines.join("\n");
    Ok((!text.is_empty()).then_some(text))
}

fn panel_dialogue_text(panel: &MangaPanelV2) -> Result<Option<String>, AppError> {
    let mut layers = Vec::new();
    for layer in &panel.text_layers {
        layers.push(match layer.kind {
            MangaTextLayerKindV2::Dialogue | MangaTextLayerKindV2::Thought => {
                let speaker = layer.speaker.as_deref().ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "漫画第 {} 格的对白/想法文本缺少 speaker",
                        panel.panel_number
                    ))
                })?;
                format!("{}：{}", speaker, layer.text)
            }
            MangaTextLayerKindV2::Narration => format!("旁白：{}", layer.text),
            MangaTextLayerKindV2::Sfx => format!("音效：{}", layer.text),
        });
    }
    let text = layers.join(" ");
    Ok((!text.is_empty()).then_some(text))
}
