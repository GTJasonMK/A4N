use a4n_domain::manga::{
    MangaPageGenerationLayoutV4, MangaPageGenerationSchemaVersion, MangaPagePipelineStateV1,
    MangaPagePipelineStatus, MangaPageV4, MangaPanelV2, MangaReferenceBindingSummary,
    MangaTextLayerKindV2,
};
use a4n_error::AppError;
use serde_json::json;

use super::super::super::output_sync::MangaPagePromptDraft;
use super::super::super::reference_assets::page_reference_bindings;
use super::contracts::MangaPipelinePersistInput;

pub(in crate::manga) fn pipeline_page_draft(
    input: &MangaPipelinePersistInput<'_>,
    page_number: u32,
    pipeline: MangaPagePipelineStateV1,
    generation_traces: Vec<super::super::prompt::MangaPromptGenerationTrace>,
    existing_layout: Option<&MangaPageGenerationLayoutV4>,
) -> Result<MangaPagePromptDraft, AppError> {
    pipeline
        .validate(page_number)
        .map_err(AppError::InvalidArgument)?;
    let page_number_db = i32::try_from(page_number)
        .map_err(|_| AppError::InvalidArgument(format!("漫画页码超出 i32 范围: {page_number}")))?;
    let generated_reference_bindings = pipeline.storyboard.as_ref().map_or_else(
        || Ok(Vec::new()),
        |page| {
            page_reference_bindings(page, input.reference_assets_metadata)
                .map_err(AppError::InvalidArgument)
        },
    )?;
    let reference_bindings = merge_reference_bindings(
        existing_layout.map(|layout| layout.reference_bindings.as_slice()),
        generated_reference_bindings,
    );
    let page_plan_ref = input
        .page_plan_ref
        .or_else(|| existing_layout.and_then(|layout| layout.page_plan_ref.as_ref()));
    let dialogue_text = pipeline
        .storyboard
        .as_ref()
        .map(page_dialogue_text)
        .transpose()?
        .flatten();
    let characters_in_frame = pipeline
        .storyboard
        .as_ref()
        .map(|page| json!(page_characters(page)));
    let image_prompt = pipeline
        .drawing_prompt
        .as_ref()
        .map_or_else(String::new, |prompt| prompt.image_prompt.clone());
    let status = manga_page_prompt_record_status(pipeline.status);
    Ok(MangaPagePromptDraft {
        chapter_id: input.chapter_id,
        project_id: input.project_id,
        page_number: page_number_db,
        image_prompt,
        dialogue_text,
        characters_in_frame,
        layout: Some(serde_json::to_value(MangaPageGenerationLayoutV4 {
            schema_version: MangaPageGenerationSchemaVersion::V4,
            page_id: pipeline.page_id.clone(),
            page_number,
            page_plan_ref: page_plan_ref.cloned(),
            pipeline,
            reference_bindings,
            prompt_fingerprint: input.prompt_fingerprint.clone(),
            source: input.source_metadata.clone(),
        })?),
        status: status.to_string(),
        generation_traces,
    })
}

fn manga_page_prompt_record_status(status: MangaPagePipelineStatus) -> &'static str {
    match status {
        MangaPagePipelineStatus::Pending => "pending",
        MangaPagePipelineStatus::Running => "generating",
        MangaPagePipelineStatus::Failed => "failed",
        MangaPagePipelineStatus::Cancelled => "cancelled",
        MangaPagePipelineStatus::Completed => "completed",
    }
}

fn merge_reference_bindings(
    existing: Option<&[MangaReferenceBindingSummary]>,
    mut generated: Vec<MangaReferenceBindingSummary>,
) -> Vec<MangaReferenceBindingSummary> {
    let Some(existing) = existing else {
        return generated;
    };
    for binding in &mut generated {
        if let Some(previous) = existing
            .iter()
            .find(|previous| previous.character_name == binding.character_name)
            && previous.bound_asset.is_some()
        {
            binding.bound_asset = previous.bound_asset.clone();
        }
    }
    generated
}

fn page_characters(page: &MangaPageV4) -> Vec<String> {
    let mut names = page
        .panels
        .iter()
        .flat_map(|panel| panel.participants.iter().cloned())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn page_dialogue_text(page: &MangaPageV4) -> Result<Option<String>, AppError> {
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

#[cfg(test)]
mod tests {
    use super::{manga_page_prompt_record_status, merge_reference_bindings};
    use a4n_domain::manga::MangaPagePipelineStatus;
    use a4n_domain::manga::{MangaBoundReferenceAsset, MangaReferenceAssetKind};

    #[test]
    fn running_pipeline_state_uses_existing_generating_record_status() {
        assert_eq!(
            manga_page_prompt_record_status(MangaPagePipelineStatus::Running),
            "generating"
        );
    }

    #[test]
    fn merge_reference_bindings_preserves_manual_asset() {
        let existing = [super::MangaReferenceBindingSummary {
            character_name: "hero".into(),
            bound_asset: Some(MangaBoundReferenceAsset {
                asset_kind: MangaReferenceAssetKind::GeneratedImage,
                asset_id: 99,
                image_path: "manual.png".into(),
            }),
        }];
        let generated = vec![super::MangaReferenceBindingSummary {
            character_name: "hero".into(),
            bound_asset: Some(MangaBoundReferenceAsset {
                asset_kind: MangaReferenceAssetKind::CharacterPortrait,
                asset_id: 7,
                image_path: "portrait.png".into(),
            }),
        }];

        let merged = merge_reference_bindings(Some(&existing), generated);

        assert_eq!(merged[0].bound_asset.as_ref().unwrap().asset_id, 99);
        assert_eq!(
            merged[0].bound_asset.as_ref().unwrap().image_path,
            "manual.png"
        );
    }
}
