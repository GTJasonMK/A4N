use super::ProjectChapterSceneWriteError;
use a4n_core::models::chapter::{chapter, chapter_scene};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect,
    TransactionTrait,
    sea_query::{CaseStatement, Expr},
};
use std::collections::{HashMap, HashSet};

struct LoadedChapterSceneReorder {
    chapter_id: i64,
    project_id: i64,
    scenes: Vec<(chapter_scene::Model, i32)>,
    scene_ids: HashSet<i64>,
    target_numbers: HashSet<i32>,
}

pub async fn chapter_scene_reorder_project_id(
    db: &DatabaseConnection,
    order: &[(i64, i32)],
) -> Result<i64, ProjectChapterSceneWriteError> {
    Ok(load_chapter_scene_reorder(db, order).await?.project_id)
}

pub async fn reorder_chapter_scene_records(
    db: &DatabaseConnection,
    order: &[(i64, i32)],
) -> Result<Vec<chapter_scene::Model>, ProjectChapterSceneWriteError> {
    let txn = db.begin().await?;
    let loaded = load_chapter_scene_reorder(&txn, order).await?;

    let all_scenes = chapter_scene::Entity::find()
        .filter(chapter_scene::Column::ChapterId.eq(loaded.chapter_id))
        .all(&txn)
        .await?;
    if let Some(conflict) = all_scenes.iter().find(|scene| {
        loaded.target_numbers.contains(&scene.scene_number) && !loaded.scene_ids.contains(&scene.id)
    }) {
        return Err(ProjectChapterSceneWriteError::InvalidArgument(format!(
            "场景序号 {} 已被场景 #{} 占用",
            conflict.scene_number, conflict.id
        )));
    }
    let min_scene_number = all_scenes
        .iter()
        .map(|scene| scene.scene_number)
        .min()
        .unwrap_or_default();
    let scene_count = i32::try_from(loaded.scenes.len()).map_err(|_| {
        ProjectChapterSceneWriteError::InvalidArgument("章节场景重排数量超出 i32 范围".to_string())
    })?;
    let temp_start = min_scene_number
        .min(0)
        .checked_sub(scene_count)
        .and_then(|value| value.checked_sub(1))
        .ok_or_else(|| {
            ProjectChapterSceneWriteError::InvalidArgument(
                "章节场景重排临时序号超出 i32 范围".to_string(),
            )
        })?;
    let now = Utc::now().naive_utc();

    let scene_ids = loaded.scene_ids.iter().copied().collect::<Vec<_>>();
    let mut temporary_numbers = CaseStatement::new();
    for (idx, (scene, _)) in loaded.scenes.iter().enumerate() {
        let idx = i32::try_from(idx).map_err(|_| {
            ProjectChapterSceneWriteError::InvalidArgument(
                "章节场景重排索引超出 i32 范围".to_string(),
            )
        })?;
        let temporary_number = temp_start.checked_add(idx).ok_or_else(|| {
            ProjectChapterSceneWriteError::InvalidArgument(
                "章节场景重排临时序号超出 i32 范围".to_string(),
            )
        })?;
        temporary_numbers =
            temporary_numbers.case(chapter_scene::Column::Id.eq(scene.id), temporary_number);
    }
    chapter_scene::Entity::update_many()
        .col_expr(
            chapter_scene::Column::SceneNumber,
            temporary_numbers
                .finally(Expr::col(chapter_scene::Column::SceneNumber))
                .into(),
        )
        .col_expr(chapter_scene::Column::UpdatedAt, Expr::value(now))
        .filter(chapter_scene::Column::Id.is_in(scene_ids.iter().copied()))
        .exec(&txn)
        .await?;

    let mut final_numbers = CaseStatement::new();
    for (scene, new_number) in &loaded.scenes {
        final_numbers = final_numbers.case(chapter_scene::Column::Id.eq(scene.id), *new_number);
    }
    chapter_scene::Entity::update_many()
        .col_expr(
            chapter_scene::Column::SceneNumber,
            final_numbers
                .finally(Expr::col(chapter_scene::Column::SceneNumber))
                .into(),
        )
        .col_expr(chapter_scene::Column::UpdatedAt, Expr::value(now))
        .filter(chapter_scene::Column::Id.is_in(scene_ids.iter().copied()))
        .exec(&txn)
        .await?;

    let updated_by_id = chapter_scene::Entity::find()
        .filter(chapter_scene::Column::Id.is_in(scene_ids))
        .all(&txn)
        .await?
        .into_iter()
        .map(|scene| (scene.id, scene))
        .collect::<HashMap<_, _>>();
    let mut updated_scenes = Vec::with_capacity(loaded.scenes.len());
    for (scene, _) in loaded.scenes {
        updated_scenes.push(updated_by_id.get(&scene.id).cloned().ok_or_else(|| {
            ProjectChapterSceneWriteError::NotFound(format!("场景不存在: scene_id={}", scene.id))
        })?);
    }

    txn.commit().await?;
    updated_scenes.sort_by_key(|scene| scene.scene_number);
    Ok(updated_scenes)
}

async fn load_chapter_scene_reorder<C>(
    db: &C,
    order: &[(i64, i32)],
) -> Result<LoadedChapterSceneReorder, ProjectChapterSceneWriteError>
where
    C: ConnectionTrait,
{
    let mut scene_ids = HashSet::new();
    let mut target_numbers = HashSet::new();
    let mut chapter_id = None;

    for (scene_id, new_number) in order {
        if *new_number <= 0 {
            return Err(ProjectChapterSceneWriteError::InvalidArgument(
                "场景序号必须为正数".to_string(),
            ));
        }
        if !scene_ids.insert(*scene_id) {
            return Err(ProjectChapterSceneWriteError::InvalidArgument(format!(
                "场景排序包含重复场景: scene_id={scene_id}"
            )));
        }
        if !target_numbers.insert(*new_number) {
            return Err(ProjectChapterSceneWriteError::InvalidArgument(format!(
                "场景排序包含重复序号: scene_number={new_number}"
            )));
        }
    }
    if order.is_empty() {
        return Err(ProjectChapterSceneWriteError::InvalidArgument(
            "场景排序为空".to_string(),
        ));
    }

    let scenes_by_id = chapter_scene::Entity::find()
        .filter(chapter_scene::Column::Id.is_in(scene_ids.iter().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|scene| (scene.id, scene))
        .collect::<HashMap<_, _>>();
    let mut scenes = Vec::with_capacity(order.len());
    for (scene_id, new_number) in order {
        let scene = scenes_by_id.get(scene_id).cloned().ok_or_else(|| {
            ProjectChapterSceneWriteError::NotFound(format!("场景不存在: scene_id={scene_id}"))
        })?;
        match chapter_id {
            Some(id) if id != scene.chapter_id => {
                return Err(ProjectChapterSceneWriteError::InvalidArgument(
                    "不能跨章节调整场景顺序".to_string(),
                ));
            }
            None => chapter_id = Some(scene.chapter_id),
            _ => {}
        }
        scenes.push((scene, *new_number));
    }

    let chapter_id = chapter_id.ok_or_else(|| {
        ProjectChapterSceneWriteError::InvalidArgument("场景排序为空".to_string())
    })?;
    let project_id = chapter::Entity::find_by_id(chapter_id)
        .select_only()
        .column(chapter::Column::ProjectId)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectChapterSceneWriteError::NotFound(format!("章节不存在: chapter_id={chapter_id}"))
        })?;

    Ok(LoadedChapterSceneReorder {
        chapter_id,
        project_id,
        scenes,
        scene_ids,
        target_numbers,
    })
}
