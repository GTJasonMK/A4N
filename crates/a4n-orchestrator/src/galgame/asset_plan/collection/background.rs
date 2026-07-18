use std::collections::BTreeSet;

use a4n_core::error::AppError;
use a4n_core::models::{galgame::galgame_scene, project::source_snapshot_chapter};
use serde_json::json;

use crate::galgame::{GALGAME_ENTITY_SCENE, GALGAME_TARGET_BACKGROUND};
use a4n_db::project::ProjectFamilyContext;

use super::super::contracts::{GalgameAssetPlanGroup, GalgameAssetPlanItemDraft, GalgameSpecInput};
use super::super::drafts::galgame_asset_plan_item_draft;
use super::required_json_string;

pub(super) fn background_asset_plan_draft(
    context: &ProjectFamilyContext,
    scene: &galgame_scene::Model,
    chapter: &source_snapshot_chapter::Model,
    scene_anchor_ids: &BTreeSet<i64>,
) -> Result<GalgameAssetPlanItemDraft, AppError> {
    let params_json = json!({
        "canonical_key": format!("galgame:{}:background:scene:{}", context.project.id, scene.id),
        "asset_kind": "background",
        "scene": {
            "id": scene.id,
            "index": scene.scene_index,
            "title": scene.title,
            "location": scene.location,
            "time_of_day": scene.time_of_day,
            "background_note": scene.background_note,
        },
        "source": {
            "source_snapshot_chapter_id": chapter.id,
            "source_chapter_id": chapter.source_chapter_id,
            "chapter_number": chapter.chapter_number,
            "content_hash": chapter.content_hash,
        },
    });
    let canonical_key = required_json_string(&params_json, "canonical_key")?;
    galgame_asset_plan_item_draft(
        GalgameAssetPlanGroup::Background,
        GalgameSpecInput {
            canonical_key,
            project_id: context.project.id,
            root_project_id: context.root_project.id,
            source_snapshot_id: context.source_snapshot.id,
            source_chapter_id: Some(chapter.source_chapter_id),
            source_snapshot_chapter_id: Some(chapter.id),
            target_kind: GALGAME_TARGET_BACKGROUND,
            target_entity_kind: GALGAME_ENTITY_SCENE,
            target_entity_id: Some(scene.id),
            params_json,
        },
        scene_anchor_ids,
        false,
        None,
        format!("{} 背景", scene.title),
    )
}
