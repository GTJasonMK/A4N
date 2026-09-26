use super::super::{SourceSyncDbError, generation_specs, validate_positive_id};
use super::types::{SourceSyncGalgameAnchorRecord, SourceSyncGalgameSceneRegenerationSourceRecord};
use a4n_db_models::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    project::{source_snapshot_chapter, text_anchor},
    source_sync::{source_sync_conflict, source_sync_run},
};
use a4n_domain::source_sync::GALGAME_REGENERATABLE_ENTRY_KIND;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::{HashMap, HashSet};

pub(in crate::source_sync) async fn galgame_scene_regeneration_targets<C>(
    db: &C,
    project_id: i64,
    conflicts: &[source_sync_conflict::Model],
) -> Result<HashMap<i64, i64>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let candidates = conflicts
        .iter()
        .filter(|conflict| {
            conflict.status == crate::source_sync::CONFLICT_STATUS_PENDING
                && conflict.conflict_scope == "text_anchor"
                && matches!(
                    conflict.target_kind.as_str(),
                    "galgame_scene" | "galgame_line"
                )
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Ok(HashMap::new());
    }
    let script_id = galgame_script::Entity::find()
        .select_only()
        .column(galgame_script::Column::Id)
        .filter(galgame_script::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .one(db)
        .await?;
    let Some(script_id) = script_id else {
        return Ok(HashMap::new());
    };
    let scene_ids = galgame_scene::Entity::find()
        .select_only()
        .column(galgame_scene::Column::Id)
        .filter(galgame_scene::Column::ScriptId.eq(script_id))
        .filter(galgame_scene::Column::EntryKind.eq(GALGAME_REGENERATABLE_ENTRY_KIND.as_str()))
        .filter(galgame_scene::Column::ShootingSceneIdentity.is_not_null())
        .into_tuple::<i64>()
        .all(db)
        .await?
        .into_iter()
        .collect::<HashSet<_>>();
    let line_scenes = galgame_line::Entity::find()
        .select_only()
        .column(galgame_line::Column::Id)
        .column(galgame_line::Column::SceneId)
        .filter(
            galgame_line::Column::Id.is_in(
                candidates
                    .iter()
                    .filter(|conflict| conflict.target_kind == "galgame_line")
                    .map(|conflict| conflict.target_id),
            ),
        )
        .filter(galgame_line::Column::SceneId.is_in(scene_ids.iter().copied()))
        .into_tuple::<(i64, i64)>()
        .all(db)
        .await?
        .into_iter()
        .collect::<HashMap<_, _>>();
    Ok(candidates
        .into_iter()
        .filter_map(|conflict| {
            let scene_id = if conflict.target_kind == "galgame_scene" {
                scene_ids.get(&conflict.target_id)
            } else {
                line_scenes.get(&conflict.target_id)
            };
            scene_id.map(|scene_id| (conflict.id, *scene_id))
        })
        .collect())
}

pub async fn galgame_scene_regeneration_source_record<C>(
    db: &C,
    run: &source_sync_run::Model,
    conflict: &source_sync_conflict::Model,
    expected_scene_id: i64,
) -> Result<SourceSyncGalgameSceneRegenerationSourceRecord, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.root_project_id, "root_project_id")?;
    validate_positive_id(run.from_snapshot_id, "from_snapshot_id")?;
    validate_positive_id(run.to_snapshot_id, "to_snapshot_id")?;
    validate_positive_id(conflict.id, "source_sync_conflict_id")?;
    validate_positive_id(expected_scene_id, "galgame_scene_id")?;
    if conflict.conflict_scope != "text_anchor"
        || !matches!(
            conflict.target_kind.as_str(),
            "galgame_scene" | "galgame_line"
        )
    {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "regenerate_scene 只支持 Galgame 场/行文本锚点冲突，当前为 {}/{}",
            conflict.conflict_scope, conflict.target_kind
        )));
    }

    let scene_id = match conflict.target_kind.as_str() {
        "galgame_scene" => conflict.target_id,
        "galgame_line" => galgame_line::Entity::find_by_id(conflict.target_id)
            .select_only()
            .column(galgame_line::Column::SceneId)
            .into_tuple::<i64>()
            .one(db)
            .await?
            .ok_or_else(|| {
                SourceSyncDbError::NotFound(format!("Galgame 行不存在: {}", conflict.target_id))
            })?,
        other => {
            return Err(SourceSyncDbError::InvalidArgument(format!(
                "regenerate_scene 冲突目标类型不受支持: {other}"
            )));
        }
    };
    if expected_scene_id != scene_id {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "regenerate_scene payload 指向 Galgame 场 {}，但冲突目标属于场 {}",
            expected_scene_id, scene_id
        )));
    }

    let scene = galgame_scene::Entity::find_by_id(scene_id)
        .one(db)
        .await?
        .ok_or_else(|| SourceSyncDbError::NotFound(format!("Galgame 场不存在: {scene_id}")))?;
    if scene.entry_kind != GALGAME_REGENERATABLE_ENTRY_KIND.as_str() {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "regenerate_scene 当前只支持 main 场，Galgame 场 {} 是 {}",
            scene.id, scene.entry_kind
        )));
    }
    let script = galgame_script::Entity::find_by_id(scene.script_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            SourceSyncDbError::NotFound(format!("Galgame 剧本不存在: {}", scene.script_id))
        })?;
    if script.project_id != run.derived_project_id || script.root_project_id != run.root_project_id
    {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "Galgame 场 {} 不属于本次源同步运行",
            scene.id
        )));
    }

    let old_snapshot_chapter =
        source_snapshot_chapter::Entity::find_by_id(scene.source_snapshot_chapter_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                SourceSyncDbError::NotFound(format!(
                    "旧源快照章节不存在: {}",
                    scene.source_snapshot_chapter_id
                ))
            })?;
    if old_snapshot_chapter.root_project_id != run.root_project_id {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "Galgame 场 {} 的源快照章节不属于本次根项目",
            scene.id
        )));
    }
    let new_snapshot_chapter = generation_specs::source_snapshot_chapter_for_source(
        db,
        run.to_snapshot_id,
        old_snapshot_chapter.source_chapter_id,
    )
    .await?;
    let anchors = text_anchor::Entity::find()
        .select_only()
        .column(text_anchor::Column::Id)
        .column(text_anchor::Column::ParagraphIndex)
        .column(text_anchor::Column::StartOffset)
        .column(text_anchor::Column::EndOffset)
        .column(text_anchor::Column::Quote)
        .filter(text_anchor::Column::ProjectId.eq(run.root_project_id))
        .filter(text_anchor::Column::SourceSnapshotId.eq(run.to_snapshot_id))
        .filter(text_anchor::Column::SourceSnapshotChapterId.eq(new_snapshot_chapter.id))
        .order_by_asc(text_anchor::Column::ParagraphIndex)
        .order_by_asc(text_anchor::Column::StartOffset)
        .order_by_asc(text_anchor::Column::EndOffset)
        .into_tuple::<(i64, i32, i32, i32, String)>()
        .all(db)
        .await?;
    if anchors.is_empty() {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "目标源快照章节 {} 没有可用于重建 Galgame 场的文本锚点",
            new_snapshot_chapter.id
        )));
    }

    Ok(SourceSyncGalgameSceneRegenerationSourceRecord {
        scene,
        script,
        old_snapshot_chapter,
        new_snapshot_chapter,
        anchors: anchors
            .into_iter()
            .map(|(id, paragraph_index, start_offset, end_offset, quote)| {
                SourceSyncGalgameAnchorRecord {
                    id,
                    paragraph_index,
                    start_offset,
                    end_offset,
                    quote,
                }
            })
            .collect(),
    })
}
