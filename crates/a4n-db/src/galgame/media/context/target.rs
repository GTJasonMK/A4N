use crate::galgame::{
    GALGAME_ENTITY_LINE, GALGAME_ENTITY_SCENE, GalgameMediaRecordError, GalgameTargetContext,
    require_galgame_media_text, validate_galgame_media_target_ids,
};
use a4n_core::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    project::source_snapshot_chapter,
};
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, QuerySelect};

pub async fn galgame_media_target_context<C>(
    db: &C,
    project_id: i64,
    target_kind: &str,
    target_id: i64,
) -> Result<GalgameTargetContext, GalgameMediaRecordError>
where
    C: ConnectionTrait,
{
    galgame_media_target_context_in(db, project_id, target_kind, target_id, false).await
}

pub(in crate::galgame::media) async fn galgame_media_target_context_in<C>(
    db: &C,
    project_id: i64,
    target_kind: &str,
    target_id: i64,
    lock_for_update: bool,
) -> Result<GalgameTargetContext, GalgameMediaRecordError>
where
    C: ConnectionTrait,
{
    validate_galgame_media_target_ids(project_id, target_id)?;
    let target_kind = require_galgame_media_text(target_kind, "target_kind")?;
    let use_row_lock = lock_for_update && db.get_database_backend() != DbBackend::Sqlite;
    let (project, root_project_id) =
        super::galgame_media_project_context(db, project_id, use_row_lock).await?;
    let source_snapshot_id = project.source_snapshot_id.ok_or_else(|| {
        GalgameMediaRecordError::InvalidArgument(format!(
            "Galgame 项目缺少源文本快照: {}",
            project.id
        ))
    })?;
    let (scene_id, script_id, source_snapshot_chapter_id, line_id) = match target_kind {
        GALGAME_ENTITY_SCENE => {
            let mut scene_query = galgame_scene::Entity::find_by_id(target_id);
            if use_row_lock {
                scene_query = scene_query.lock_exclusive();
            }
            let scene = scene_query
                .select_only()
                .column(galgame_scene::Column::Id)
                .column(galgame_scene::Column::ScriptId)
                .column(galgame_scene::Column::SourceSnapshotChapterId)
                .into_tuple::<(i64, i64, i64)>()
                .one(db)
                .await
                .map_err(GalgameMediaRecordError::Database)?
                .ok_or_else(|| {
                    GalgameMediaRecordError::NotFound(format!("Galgame 场不存在: {target_id}"))
                })?;
            (scene.0, scene.1, scene.2, None)
        }
        GALGAME_ENTITY_LINE => {
            let mut line_query = galgame_line::Entity::find_by_id(target_id);
            if use_row_lock {
                line_query = line_query.lock_exclusive();
            }
            let line = line_query
                .select_only()
                .column(galgame_line::Column::Id)
                .column(galgame_line::Column::SceneId)
                .into_tuple::<(i64, i64)>()
                .one(db)
                .await
                .map_err(GalgameMediaRecordError::Database)?
                .ok_or_else(|| {
                    GalgameMediaRecordError::NotFound(format!("Galgame 行不存在: {target_id}"))
                })?;
            let mut scene_query = galgame_scene::Entity::find_by_id(line.1);
            if use_row_lock {
                scene_query = scene_query.lock_exclusive();
            }
            let scene = scene_query
                .select_only()
                .column(galgame_scene::Column::Id)
                .column(galgame_scene::Column::ScriptId)
                .column(galgame_scene::Column::SourceSnapshotChapterId)
                .into_tuple::<(i64, i64, i64)>()
                .one(db)
                .await
                .map_err(GalgameMediaRecordError::Database)?
                .ok_or_else(|| {
                    GalgameMediaRecordError::NotFound(format!("Galgame 场不存在: {}", line.1))
                })?;
            (scene.0, scene.1, scene.2, Some(line.0))
        }
        other => {
            return Err(GalgameMediaRecordError::InvalidArgument(format!(
                "Galgame 媒体目标类型无效: {other}"
            )));
        }
    };
    let mut script_query = galgame_script::Entity::find_by_id(script_id);
    if use_row_lock {
        script_query = script_query.lock_exclusive();
    }
    let script = script_query
        .select_only()
        .column(galgame_script::Column::ProjectId)
        .column(galgame_script::Column::RootProjectId)
        .column(galgame_script::Column::SourceSnapshotId)
        .into_tuple::<(i64, i64, i64)>()
        .one(db)
        .await
        .map_err(GalgameMediaRecordError::Database)?
        .ok_or_else(|| {
            GalgameMediaRecordError::NotFound(format!("Galgame 剧本不存在: {script_id}"))
        })?;
    if script.0 != project.id || script.1 != root_project_id || script.2 != source_snapshot_id {
        return Err(GalgameMediaRecordError::InvalidArgument(
            "Galgame 媒体目标不属于目标项目族上下文".to_string(),
        ));
    }
    let mut snapshot_chapter_query =
        source_snapshot_chapter::Entity::find_by_id(source_snapshot_chapter_id);
    if use_row_lock {
        snapshot_chapter_query = snapshot_chapter_query.lock_exclusive();
    }
    let snapshot_chapter = snapshot_chapter_query
        .select_only()
        .column(source_snapshot_chapter::Column::SnapshotId)
        .column(source_snapshot_chapter::Column::SourceChapterId)
        .into_tuple::<(i64, i64)>()
        .one(db)
        .await
        .map_err(GalgameMediaRecordError::Database)?
        .ok_or_else(|| {
            GalgameMediaRecordError::NotFound(format!(
                "Galgame 场引用的源快照章节不存在: {}",
                source_snapshot_chapter_id
            ))
        })?;
    if snapshot_chapter.0 != script.2 {
        return Err(GalgameMediaRecordError::InvalidArgument(
            "Galgame 场源快照章节与剧本源快照不一致".to_string(),
        ));
    }
    Ok(GalgameTargetContext {
        project_id: project.id,
        root_project_id,
        source_snapshot_id,
        source_chapter_id: snapshot_chapter.1,
        source_snapshot_chapter_id,
        scene_id,
        line_id,
    })
}
