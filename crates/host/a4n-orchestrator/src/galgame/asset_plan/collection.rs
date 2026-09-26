mod background;
mod event_cg;
mod portrait;

use std::collections::{BTreeMap, BTreeSet};

use a4n_db::galgame::GalgameAssetPlanningReadModel;
use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::{GalgameLine, galgame_line_from_model};
use a4n_db::project::ProjectFamilyContext;

use super::contracts::GalgameAssetPlanItemDraft;
use super::drafts::GalgamePortraitAssetDraft;

pub(super) fn collect_galgame_asset_plan_drafts(
    context: &ProjectFamilyContext,
    read_model: GalgameAssetPlanningReadModel,
) -> Result<Vec<GalgameAssetPlanItemDraft>, AppError> {
    let snapshot_chapters = read_model
        .source_snapshot_chapters
        .into_iter()
        .map(|chapter| (chapter.id, chapter))
        .collect::<BTreeMap<_, _>>();
    let mut lines_by_scene = BTreeMap::<i64, Vec<GalgameLine>>::new();
    for line in read_model.lines {
        let line = galgame_line_from_model(line)?;
        lines_by_scene.entry(line.scene_id).or_default().push(line);
    }

    let mut item_drafts = Vec::new();
    let mut portrait_drafts = BTreeMap::<(i64, String), GalgamePortraitAssetDraft>::new();

    for scene in read_model.scenes {
        let chapter = snapshot_chapters
            .get(&scene.source_snapshot_chapter_id)
            .ok_or_else(|| AppError::InvalidState {
                current: format!(
                    "source_snapshot_chapter_{}_missing_for_galgame_scene_{}",
                    scene.source_snapshot_chapter_id, scene.id
                ),
                operation: "plan_galgame_assets".to_string(),
            })?;
        let scene_lines = lines_by_scene.remove(&scene.id).unwrap_or_default();
        let scene_anchor_ids = scene_lines
            .iter()
            .filter_map(|line| line.text_anchor_id)
            .collect::<BTreeSet<_>>();
        if scene_anchor_ids.is_empty() {
            return Err(AppError::InvalidState {
                current: format!("galgame_scene_{}_has_no_text_anchor_context", scene.id),
                operation: "plan_galgame_assets".to_string(),
            });
        }

        item_drafts.push(background::background_asset_plan_draft(
            context,
            &scene,
            chapter,
            &scene_anchor_ids,
        )?);

        for line in &scene_lines {
            portrait::collect_portrait_asset_sample(
                context,
                chapter.snapshot_id,
                &scene.title,
                line,
                &scene_anchor_ids,
                &mut portrait_drafts,
            );
            if let Some(draft) = event_cg::event_cg_asset_plan_draft(
                context,
                &scene,
                chapter,
                line,
                &scene_anchor_ids,
            )? {
                item_drafts.push(draft);
            }
        }
    }

    item_drafts.extend(portrait::portrait_asset_plan_drafts(
        context,
        portrait_drafts,
    )?);

    Ok(item_drafts)
}

fn required_json_string(value: &Value, field: &str) -> Result<String, AppError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::InvalidArgument(format!("params_json.{field} 不能为空")))
}
