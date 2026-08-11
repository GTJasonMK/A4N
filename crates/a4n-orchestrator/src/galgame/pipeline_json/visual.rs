use a4n_core::error::AppError;
use serde_json::Value;

use super::contracts::*;
use super::rules::*;

pub fn visual_plan_from_value(value: Value, context: &str) -> Result<GalgameVisualPlan, AppError> {
    let raw: VisualPlanRaw = from_value(value, context)?;
    require_nonempty(
        raw.backgrounds.as_slice(),
        &format!("{context}.backgrounds"),
    )?;
    let backgrounds = raw
        .backgrounds
        .into_iter()
        .map(|background| {
            require_positive(
                background.scene_index,
                &format!("{context}.backgrounds[].scene_index"),
            )?;
            if let Some(scene_profile_id) = background.scene_profile_id {
                require_positive_i64(
                    scene_profile_id,
                    &format!("{context}.backgrounds[].scene_profile_id"),
                )?;
            }
            Ok(GalgamePipelineBackgroundPlan {
                scene_index: background.scene_index,
                background_note: required_text(
                    background.background_note,
                    &format!("{context}.backgrounds[].background_note"),
                )?,
                scene_profile_id: background.scene_profile_id,
                source_span_ids: normalize_source_span_ids(
                    background.source_span_ids,
                    &format!("{context}.backgrounds[].source_span_ids"),
                    true,
                )?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    ensure_unique_indexes(
        backgrounds.iter().map(|background| background.scene_index),
        &format!("{context}.backgrounds.scene_index"),
    )?;
    let event_cgs = raw
        .event_cgs
        .into_iter()
        .map(|cg| {
            require_positive(
                cg.scene_index,
                &format!("{context}.event_cgs[].scene_index"),
            )?;
            require_positive(cg.line_index, &format!("{context}.event_cgs[].line_index"))?;
            Ok(GalgamePipelineEventCgPlan {
                scene_index: cg.scene_index,
                line_index: cg.line_index,
                cg_note: required_text(cg.cg_note, &format!("{context}.event_cgs[].cg_note"))?,
                importance: cg.importance,
                source_span_ids: normalize_source_span_ids(
                    cg.source_span_ids,
                    &format!("{context}.event_cgs[].source_span_ids"),
                    true,
                )?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(GalgameVisualPlan {
        backgrounds,
        event_cgs,
    })
}
