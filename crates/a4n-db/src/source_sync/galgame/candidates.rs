use super::super::{SourceSnapshotChapterChangeRecord, SourceSyncDbError, validate_positive_id};
use super::types::{
    SourceSyncGalgameAnchorConflictCandidateRecord, SourceSyncGalgameSceneConflictCandidateRecord,
};
use a4n_core::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    project::{source_snapshot_chapter, text_anchor},
    source_sync::source_sync_run,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder};
use std::collections::HashMap;

pub async fn galgame_scene_conflict_candidates_for_chapter_change<C>(
    db: &C,
    run: &source_sync_run::Model,
    change: &SourceSnapshotChapterChangeRecord,
) -> Result<Vec<SourceSyncGalgameSceneConflictCandidateRecord>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.root_project_id, "root_project_id")?;
    validate_positive_id(run.from_snapshot_id, "from_snapshot_id")?;
    validate_positive_id(change.source_chapter_id, "source_chapter_id")?;

    let Some(script) = galgame_script::Entity::find()
        .filter(galgame_script::Column::ProjectId.eq(run.derived_project_id))
        .filter(galgame_script::Column::RootProjectId.eq(run.root_project_id))
        .one(db)
        .await?
    else {
        return Ok(Vec::new());
    };
    let Some(old_snapshot_chapter) = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(run.from_snapshot_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(change.source_chapter_id))
        .one(db)
        .await?
    else {
        return Ok(Vec::new());
    };
    let scenes = galgame_scene::Entity::find()
        .filter(galgame_scene::Column::ScriptId.eq(script.id))
        .filter(galgame_scene::Column::SourceSnapshotChapterId.eq(old_snapshot_chapter.id))
        .order_by_asc(galgame_scene::Column::SceneIndex)
        .all(db)
        .await?;

    let mut records = Vec::new();
    for scene in scenes {
        let line_count = galgame_line::Entity::find()
            .filter(galgame_line::Column::SceneId.eq(scene.id))
            .count(db)
            .await?;
        let anchored_line_count = galgame_line::Entity::find()
            .filter(galgame_line::Column::SceneId.eq(scene.id))
            .filter(galgame_line::Column::TextAnchorId.is_not_null())
            .count(db)
            .await?;
        records.push(SourceSyncGalgameSceneConflictCandidateRecord {
            scene,
            old_source_snapshot_chapter_id: old_snapshot_chapter.id,
            line_count,
            anchored_line_count,
        });
    }
    Ok(records)
}

pub async fn galgame_anchor_conflict_candidates_for_run<C>(
    db: &C,
    run: &source_sync_run::Model,
) -> Result<Vec<SourceSyncGalgameAnchorConflictCandidateRecord>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.root_project_id, "root_project_id")?;
    validate_positive_id(run.from_snapshot_id, "from_snapshot_id")?;
    validate_positive_id(run.to_snapshot_id, "to_snapshot_id")?;

    let Some(script) = galgame_script::Entity::find()
        .filter(galgame_script::Column::ProjectId.eq(run.derived_project_id))
        .filter(galgame_script::Column::RootProjectId.eq(run.root_project_id))
        .one(db)
        .await?
    else {
        return Ok(Vec::new());
    };
    let scenes = galgame_scene::Entity::find()
        .filter(galgame_scene::Column::ScriptId.eq(script.id))
        .order_by_asc(galgame_scene::Column::SceneIndex)
        .all(db)
        .await?;
    if scenes.is_empty() {
        return Ok(Vec::new());
    }
    let scene_by_id = scenes
        .iter()
        .map(|scene| (scene.id, scene))
        .collect::<HashMap<_, _>>();
    let scene_ids = scenes.iter().map(|scene| scene.id).collect::<Vec<_>>();
    let lines = galgame_line::Entity::find()
        .filter(galgame_line::Column::SceneId.is_in(scene_ids))
        .filter(galgame_line::Column::TextAnchorId.is_not_null())
        .order_by_asc(galgame_line::Column::SceneId)
        .order_by_asc(galgame_line::Column::LineIndex)
        .all(db)
        .await?;

    let mut records = Vec::new();
    for line in lines {
        let Some(text_anchor_id) = line.text_anchor_id else {
            continue;
        };
        let old_anchor = text_anchor::Entity::find_by_id(text_anchor_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                SourceSyncDbError::NotFound(format!("文本锚点不存在: {text_anchor_id}"))
            })?;
        if old_anchor.source_snapshot_id != Some(run.from_snapshot_id) {
            continue;
        }
        let candidates = text_anchor::Entity::find()
            .filter(text_anchor::Column::ProjectId.eq(run.root_project_id))
            .filter(text_anchor::Column::SourceSnapshotId.eq(run.to_snapshot_id))
            .filter(text_anchor::Column::QuoteHash.eq(&old_anchor.quote_hash))
            .all(db)
            .await?;
        records.push(SourceSyncGalgameAnchorConflictCandidateRecord {
            scene_title: scene_by_id
                .get(&line.scene_id)
                .map(|item| item.title.clone()),
            line,
            old_anchor,
            candidates,
        });
    }
    Ok(records)
}
