use std::collections::BTreeSet;

use a4n_db::galgame::{GalgameAssetPlanningSceneRecord, GalgameAssetPlanningSourceChapterRecord};
use a4n_error::AppError;
use serde_json::json;

use crate::galgame::{GALGAME_ENTITY_LINE, GALGAME_TARGET_CG, GalgameLine};
use a4n_db::project::ProjectFamilyContext;

use super::super::contracts::{GalgameAssetPlanGroup, GalgameAssetPlanItemDraft, GalgameSpecInput};
use super::super::drafts::galgame_asset_plan_item_draft;
use super::required_json_string;

pub(super) fn event_cg_asset_plan_draft(
    context: &ProjectFamilyContext,
    scene: &GalgameAssetPlanningSceneRecord,
    chapter: &GalgameAssetPlanningSourceChapterRecord,
    line: &GalgameLine,
    scene_anchor_ids: &BTreeSet<i64>,
) -> Result<Option<GalgameAssetPlanItemDraft>, AppError> {
    if line.presentation_json.cg != Some(true) {
        return Ok(None);
    }
    let cg_note =
        line.presentation_json
            .cg_note
            .as_deref()
            .ok_or_else(|| AppError::InvalidState {
                current: "galgame_asset_plan".to_string(),
                operation: format!("Galgame 行 {} 缺少 cg_note", line.id),
            })?;
    let cg_importance =
        line.presentation_json
            .cg_importance
            .ok_or_else(|| AppError::InvalidState {
                current: "galgame_asset_plan".to_string(),
                operation: format!("Galgame 行 {} 缺少 cg_importance", line.id),
            })?;
    let canonical_key = format!("galgame:{}:cg:line:{}", context.project.id, line.id);
    let line_anchor_ids = line.text_anchor_id.into_iter().collect::<BTreeSet<_>>();
    let anchor_ids = if line_anchor_ids.is_empty() {
        scene_anchor_ids
    } else {
        &line_anchor_ids
    };
    let params_json = json!({
        "canonical_key": canonical_key,
        "asset_kind": "event_cg",
        "line": {
            "id": line.id,
            "line_index": line.line_index,
            "line_type": line.line_type,
            "speaker_profile_id": line.speaker_profile_id,
            "text": line.text,
            "expression": line.expression,
            "pose": line.pose,
            "position": line.position,
        },
        "scene": {
            "id": scene.id,
            "title": scene.title,
            "location": scene.location,
            "time_of_day": scene.time_of_day,
        },
        "cg_note": cg_note,
        "cg_importance": cg_importance,
    });
    let canonical_key = required_json_string(&params_json, "canonical_key")?;
    galgame_asset_plan_item_draft(
        GalgameAssetPlanGroup::EventCg,
        GalgameSpecInput {
            canonical_key,
            project_id: context.project.id,
            root_project_id: context.root_project.id,
            source_snapshot_id: context.source_snapshot.id,
            source_chapter_id: Some(chapter.source_chapter_id),
            source_snapshot_chapter_id: Some(chapter.id),
            target_kind: GALGAME_TARGET_CG,
            target_entity_kind: GALGAME_ENTITY_LINE,
            target_entity_id: Some(line.id),
            params_json,
        },
        anchor_ids,
        true,
        line.speaker_profile_id,
        format!("{} #{} CG", scene.title, line.line_index),
    )
    .map(Some)
}
