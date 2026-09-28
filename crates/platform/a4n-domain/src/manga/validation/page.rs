use std::collections::BTreeSet;

use super::super::contracts::{
    MangaPageV4, MangaPanelSizeV2, MangaPanelV2, MangaTextLayerKindV2, MangaTextLayerV2,
};
use super::MangaPlanValidationError;
use super::rules::{
    index_u32, invalid, optional_text, required_text, unique_required_beat_refs, unique_texts,
    validation_error,
};
use crate::media_authoring::AdaptationBeatRef;

pub(super) fn validate_page(
    page: &MangaPageV4,
    path: &str,
) -> Result<(), MangaPlanValidationError> {
    unique_required_beat_refs(&page.beat_refs, format!("{path}.beat_refs"))?;
    required_text(&page.page_goal, format!("{path}.page_goal"))?;
    required_text(&page.visual_thesis, format!("{path}.visual_thesis"))?;
    required_text(&page.entry_state, format!("{path}.entry_state"))?;
    required_text(&page.exit_state, format!("{path}.exit_state"))?;
    if page.panels.is_empty() {
        return invalid(format!("{path}.panels"), "至少需要一个画格");
    }
    if page.dominant_panel_number == 0 || page.dominant_panel_number as usize > page.panels.len() {
        return invalid(
            format!("{path}.dominant_panel_number"),
            "必须指向本页现有画格",
        );
    }
    let mut dominant_count = 0usize;
    for (panel_offset, panel) in page.panels.iter().enumerate() {
        let expected = index_u32(panel_offset, &format!("{path}.panels"))?;
        let panel_path = format!("{path}.panels[{panel_offset}]");
        if panel.panel_number != expected {
            return invalid(
                format!("{panel_path}.panel_number"),
                format!("必须从 1 连续递增，期望 {expected}"),
            );
        }
        if panel.size == MangaPanelSizeV2::Dominant {
            dominant_count += 1;
            if panel.panel_number != page.dominant_panel_number {
                return invalid(
                    format!("{panel_path}.size"),
                    "dominant 画格必须与 dominant_panel_number 一致",
                );
            }
        }
        validate_panel(panel, &panel_path, &page.beat_refs)?;
    }
    if dominant_count != 1 {
        return invalid(
            format!("{path}.panels"),
            "每页必须且只能有一个 dominant 画格",
        );
    }
    Ok(())
}

fn validate_panel(
    panel: &MangaPanelV2,
    path: &str,
    page_beat_refs: &[AdaptationBeatRef],
) -> Result<(), MangaPlanValidationError> {
    let panel_refs = unique_required_beat_refs(&panel.beat_refs, format!("{path}.beat_refs"))?;
    let page_refs = page_beat_refs.iter().cloned().collect::<BTreeSet<_>>();
    for beat_ref in panel_refs {
        if !page_refs.contains(&beat_ref) {
            return invalid(
                format!("{path}.beat_refs"),
                format!(
                    "改编单元 {} 的节拍 {} 不属于所在页",
                    beat_ref.unit_id, beat_ref.beat_id
                ),
            );
        }
    }
    unique_texts(&panel.participants, format!("{path}.participants"), false)?;
    required_text(&panel.composition, format!("{path}.composition"))?;
    required_text(
        &panel.observable_action,
        format!("{path}.observable_action"),
    )?;
    optional_text(panel.reaction.as_deref(), format!("{path}.reaction"))?;
    required_text(
        &panel.spatial_relationship,
        format!("{path}.spatial_relationship"),
    )?;
    optional_text(
        panel.continuity_from_previous.as_deref(),
        format!("{path}.continuity_from_previous"),
    )?;
    for (layer_offset, layer) in panel.text_layers.iter().enumerate() {
        let expected = index_u32(layer_offset, &format!("{path}.text_layers"))?;
        let layer_path = format!("{path}.text_layers[{layer_offset}]");
        if layer.layer_index != expected {
            return invalid(
                format!("{layer_path}.layer_index"),
                format!("必须从 1 连续递增，期望 {expected}"),
            );
        }
        validate_text_layer(layer, &layer_path, &panel.participants)?;
    }
    Ok(())
}

fn validate_text_layer(
    layer: &MangaTextLayerV2,
    path: &str,
    participants: &[String],
) -> Result<(), MangaPlanValidationError> {
    required_text(&layer.text, format!("{path}.text"))?;
    match layer.kind {
        MangaTextLayerKindV2::Dialogue | MangaTextLayerKindV2::Thought => {
            let speaker = layer.speaker.as_deref().ok_or_else(|| {
                validation_error(format!("{path}.speaker"), "对白或想法必须指定 speaker")
            })?;
            if !participants
                .iter()
                .any(|participant| participant.trim() == speaker.trim())
            {
                return invalid(
                    format!("{path}.speaker"),
                    "speaker 必须属于所在画格 participants",
                );
            }
        }
        MangaTextLayerKindV2::Narration | MangaTextLayerKindV2::Sfx => {
            if layer.speaker.is_some() {
                return invalid(
                    format!("{path}.speaker"),
                    "旁白和音效的 speaker 必须为 null",
                );
            }
        }
    }
    unique_texts(
        &layer.avoid_covering,
        format!("{path}.avoid_covering"),
        false,
    )?;
    Ok(())
}
