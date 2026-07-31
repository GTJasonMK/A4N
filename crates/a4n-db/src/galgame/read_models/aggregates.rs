use crate::galgame::{
    GalgameAssetPlanningReadModel, GalgameScriptDetailReadModel, galgame_media_binding_read_model,
};
use a4n_core::models::{
    character::protagonist_profile,
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    project::{source_snapshot_chapter, text_anchor},
};
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder};
use std::collections::BTreeSet;

pub async fn galgame_asset_planning_read_model(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Option<GalgameAssetPlanningReadModel>, DbErr> {
    let Some((script, scenes, lines)) = galgame_script_scene_lines(db, project_id).await? else {
        return Ok(None);
    };
    let source_snapshot_chapter_ids = scenes
        .iter()
        .map(|scene| scene.source_snapshot_chapter_id)
        .collect::<BTreeSet<_>>();
    let source_snapshot_chapters = if source_snapshot_chapter_ids.is_empty() {
        Vec::new()
    } else {
        source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::Id.is_in(source_snapshot_chapter_ids))
            .all(db)
            .await?
    };
    Ok(Some(GalgameAssetPlanningReadModel {
        script,
        scenes,
        lines,
        source_snapshot_chapters,
    }))
}

pub async fn galgame_script_detail_read_model(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Option<GalgameScriptDetailReadModel>, DbErr> {
    let Some((script, scenes, lines)) = galgame_script_scene_lines(db, project_id).await? else {
        return Ok(None);
    };
    let scene_ids = scenes.iter().map(|scene| scene.id).collect::<Vec<_>>();
    let line_ids = lines.iter().map(|line| line.id).collect::<Vec<_>>();
    let choices = if line_ids.is_empty() {
        Vec::new()
    } else {
        galgame_choice::Entity::find()
            .filter(galgame_choice::Column::LineId.is_in(line_ids.clone()))
            .order_by_asc(galgame_choice::Column::LineId)
            .order_by_asc(galgame_choice::Column::OptionIndex)
            .all(db)
            .await?
    };
    let text_anchor_ids = lines
        .iter()
        .filter_map(|line| line.text_anchor_id)
        .collect::<BTreeSet<_>>();
    let text_anchors = if text_anchor_ids.is_empty() {
        Vec::new()
    } else {
        text_anchor::Entity::find()
            .filter(text_anchor::Column::Id.is_in(text_anchor_ids))
            .filter(text_anchor::Column::ProjectId.eq(script.root_project_id))
            .filter(text_anchor::Column::SourceSnapshotId.eq(script.source_snapshot_id))
            .order_by_asc(text_anchor::Column::Id)
            .all(db)
            .await?
    };
    let speaker_profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(script.root_project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await?;
    let media_bindings = galgame_media_binding_read_model(db, &scene_ids, &line_ids).await?;
    Ok(Some(GalgameScriptDetailReadModel {
        script,
        speaker_profiles,
        text_anchors,
        media_bindings,
        scenes,
        lines,
        choices,
    }))
}

async fn galgame_script_scene_lines(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<
    Option<(
        galgame_script::Model,
        Vec<galgame_scene::Model>,
        Vec<galgame_line::Model>,
    )>,
    DbErr,
> {
    let Some(script) = galgame_script::Entity::find()
        .filter(galgame_script::Column::ProjectId.eq(project_id))
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    let scenes = galgame_scene::Entity::find()
        .filter(galgame_scene::Column::ScriptId.eq(script.id))
        .order_by_asc(galgame_scene::Column::SceneIndex)
        .all(db)
        .await?;
    let scene_ids = scenes.iter().map(|scene| scene.id).collect::<Vec<_>>();
    let lines = if scene_ids.is_empty() {
        Vec::new()
    } else {
        galgame_line::Entity::find()
            .filter(galgame_line::Column::SceneId.is_in(scene_ids))
            .order_by_asc(galgame_line::Column::SceneId)
            .order_by_asc(galgame_line::Column::LineIndex)
            .all(db)
            .await?
    };
    Ok(Some((script, scenes, lines)))
}
