use super::super::{SourceSnapshotChapterChangeRecord, SourceSyncDbError, validate_positive_id};
use super::types::{
    SourceSyncGalgameAnchorConflictCandidateRecord, SourceSyncGalgameAnchorConflictLineRecord,
    SourceSyncGalgameSceneConflictCandidateRecord, SourceSyncGalgameSceneConflictSceneRecord,
};
use a4n_db_models::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    project::{source_snapshot_chapter, text_anchor},
    source_sync::source_sync_run,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::HashMap;

pub async fn galgame_scene_conflict_candidates_for_chapter_changes<C>(
    db: &C,
    run: &source_sync_run::Model,
    changes: &[SourceSnapshotChapterChangeRecord],
) -> Result<HashMap<i64, Vec<SourceSyncGalgameSceneConflictCandidateRecord>>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.root_project_id, "root_project_id")?;
    validate_positive_id(run.from_snapshot_id, "from_snapshot_id")?;
    for change in changes {
        validate_positive_id(change.source_chapter_id, "source_chapter_id")?;
    }
    if changes.is_empty() {
        return Ok(HashMap::new());
    }

    let Some(script_id) = galgame_script::Entity::find()
        .select_only()
        .column(galgame_script::Column::Id)
        .filter(galgame_script::Column::ProjectId.eq(run.derived_project_id))
        .filter(galgame_script::Column::RootProjectId.eq(run.root_project_id))
        .into_tuple::<i64>()
        .one(db)
        .await?
    else {
        return Ok(HashMap::new());
    };
    let scenes = galgame_scene::Entity::find()
        .select_only()
        .column(galgame_scene::Column::Id)
        .column(galgame_scene::Column::SceneIndex)
        .column(galgame_scene::Column::Title)
        .column(galgame_scene::Column::SourceSnapshotChapterId)
        .filter(galgame_scene::Column::ScriptId.eq(script_id))
        .order_by_asc(galgame_scene::Column::SceneIndex)
        .into_tuple::<(i64, i32, String, i64)>()
        .all(db)
        .await?;
    if scenes.is_empty() {
        return Ok(HashMap::new());
    }
    let source_chapter_by_snapshot = source_snapshot_chapter::Entity::find()
        .select_only()
        .column(source_snapshot_chapter::Column::Id)
        .column(source_snapshot_chapter::Column::SourceChapterId)
        .filter(source_snapshot_chapter::Column::Id.is_in(scenes.iter().map(|scene| scene.3)))
        .filter(source_snapshot_chapter::Column::RootProjectId.eq(run.root_project_id))
        .filter(
            source_snapshot_chapter::Column::SourceChapterId
                .is_in(changes.iter().map(|change| change.source_chapter_id)),
        )
        .into_tuple::<(i64, i64)>()
        .all(db)
        .await?
        .into_iter()
        .collect::<HashMap<_, _>>();
    let scenes = scenes
        .into_iter()
        .filter(|scene| source_chapter_by_snapshot.contains_key(&scene.3))
        .collect::<Vec<_>>();
    if scenes.is_empty() {
        return Ok(HashMap::new());
    }
    let scene_ids = scenes.iter().map(|scene| scene.0).collect::<Vec<_>>();
    let mut line_counts = HashMap::<i64, (u64, u64)>::new();
    let lines = galgame_line::Entity::find()
        .select_only()
        .column(galgame_line::Column::SceneId)
        .column(galgame_line::Column::TextAnchorId)
        .filter(galgame_line::Column::SceneId.is_in(scene_ids))
        .into_tuple::<(i64, Option<i64>)>()
        .all(db)
        .await?;
    for (scene_id, text_anchor_id) in lines {
        let counts = line_counts.entry(scene_id).or_default();
        counts.0 += 1;
        if text_anchor_id.is_some() {
            counts.1 += 1;
        }
    }

    let mut records_by_chapter =
        HashMap::<i64, Vec<SourceSyncGalgameSceneConflictCandidateRecord>>::new();
    for (scene_id, scene_index, title, old_snapshot_chapter_id) in scenes {
        let Some(source_chapter_id) = source_chapter_by_snapshot.get(&old_snapshot_chapter_id)
        else {
            continue;
        };
        let (line_count, anchored_line_count) =
            line_counts.get(&scene_id).copied().unwrap_or_default();
        records_by_chapter
            .entry(*source_chapter_id)
            .or_default()
            .push(SourceSyncGalgameSceneConflictCandidateRecord {
                scene: SourceSyncGalgameSceneConflictSceneRecord {
                    id: scene_id,
                    scene_index,
                    title,
                },
                old_source_snapshot_chapter_id: old_snapshot_chapter_id,
                line_count,
                anchored_line_count,
            });
    }
    Ok(records_by_chapter)
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

    let Some(script_id) = galgame_script::Entity::find()
        .select_only()
        .column(galgame_script::Column::Id)
        .filter(galgame_script::Column::ProjectId.eq(run.derived_project_id))
        .filter(galgame_script::Column::RootProjectId.eq(run.root_project_id))
        .into_tuple::<i64>()
        .one(db)
        .await?
    else {
        return Ok(Vec::new());
    };
    let scenes = galgame_scene::Entity::find()
        .select_only()
        .column(galgame_scene::Column::Id)
        .column(galgame_scene::Column::SceneIndex)
        .column(galgame_scene::Column::Title)
        .filter(galgame_scene::Column::ScriptId.eq(script_id))
        .order_by_asc(galgame_scene::Column::SceneIndex)
        .into_model::<SourceSyncGalgameSceneConflictSceneRecord>()
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
        .select_only()
        .column(galgame_line::Column::Id)
        .column(galgame_line::Column::SceneId)
        .column(galgame_line::Column::LineIndex)
        .column(galgame_line::Column::TextAnchorId)
        .column(galgame_line::Column::LineType)
        .column(galgame_line::Column::Text)
        .filter(galgame_line::Column::SceneId.is_in(scene_ids))
        .filter(galgame_line::Column::TextAnchorId.is_not_null())
        .order_by_asc(galgame_line::Column::SceneId)
        .order_by_asc(galgame_line::Column::LineIndex)
        .into_model::<SourceSyncGalgameAnchorConflictLineRecord>()
        .all(db)
        .await?;
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    let anchor_ids = lines
        .iter()
        .filter_map(|line| line.text_anchor_id)
        .collect::<Vec<_>>();
    let anchors_by_id = text_anchor::Entity::find()
        .filter(text_anchor::Column::Id.is_in(anchor_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|anchor| (anchor.id, anchor))
        .collect::<HashMap<_, _>>();
    let source_quote_hashes = lines
        .iter()
        .filter_map(|line| line.text_anchor_id)
        .filter_map(|anchor_id| anchors_by_id.get(&anchor_id))
        .filter(|anchor| anchor.source_snapshot_id == Some(run.from_snapshot_id))
        .map(|anchor| anchor.quote_hash.clone())
        .collect::<Vec<_>>();
    let candidates_by_quote_hash = if source_quote_hashes.is_empty() {
        HashMap::new()
    } else {
        text_anchor::Entity::find()
            .filter(text_anchor::Column::ProjectId.eq(run.root_project_id))
            .filter(text_anchor::Column::SourceSnapshotId.eq(run.to_snapshot_id))
            .filter(text_anchor::Column::QuoteHash.is_in(source_quote_hashes))
            .all(db)
            .await?
            .into_iter()
            .fold(
                HashMap::<String, Vec<text_anchor::Model>>::new(),
                |mut candidates_by_quote_hash, candidate| {
                    candidates_by_quote_hash
                        .entry(candidate.quote_hash.clone())
                        .or_default()
                        .push(candidate);
                    candidates_by_quote_hash
                },
            )
    };

    let mut records = Vec::new();
    for line in lines {
        let text_anchor_id = line.text_anchor_id.ok_or_else(|| {
            SourceSyncDbError::Internal(format!(
                "非空文本锚点查询返回空值: galgame_line_id={}",
                line.id
            ))
        })?;
        let old_anchor = anchors_by_id.get(&text_anchor_id).cloned().ok_or_else(|| {
            SourceSyncDbError::NotFound(format!("文本锚点不存在: {text_anchor_id}"))
        })?;
        if old_anchor.source_snapshot_id != Some(run.from_snapshot_id) {
            continue;
        }
        let candidates = candidates_by_quote_hash
            .get(&old_anchor.quote_hash)
            .cloned()
            .unwrap_or_default();
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
