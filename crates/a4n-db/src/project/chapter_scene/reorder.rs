use super::{
    ProjectChapterSceneReorderError, ProjectChapterSceneReorderInput,
    ProjectChapterSceneReorderScope,
};
use a4n_core::models::chapter::{chapter, chapter_scene};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set, TransactionTrait,
};
use std::collections::BTreeSet;

struct LoadedChapterSceneReorder {
    scope: ProjectChapterSceneReorderScope,
    scenes: Vec<(chapter_scene::Model, i32)>,
    scene_ids: BTreeSet<i64>,
    target_numbers: Vec<i32>,
}

pub async fn chapter_scene_reorder_scope(
    db: &DatabaseConnection,
    input: ProjectChapterSceneReorderInput,
) -> Result<ProjectChapterSceneReorderScope, ProjectChapterSceneReorderError> {
    Ok(load_chapter_scene_reorder(db, &input.order).await?.scope)
}

pub async fn reorder_chapter_scene_records(
    db: &DatabaseConnection,
    input: ProjectChapterSceneReorderInput,
) -> Result<Vec<chapter_scene::Model>, ProjectChapterSceneReorderError> {
    let txn = db.begin().await?;
    let loaded = load_chapter_scene_reorder(&txn, &input.order).await?;

    let conflicts = chapter_scene::Entity::find()
        .filter(chapter_scene::Column::ChapterId.eq(loaded.scope.chapter_id))
        .filter(chapter_scene::Column::SceneNumber.is_in(loaded.target_numbers))
        .all(&txn)
        .await?
        .into_iter()
        .filter(|scene| !loaded.scene_ids.contains(&scene.id))
        .collect::<Vec<_>>();
    if let Some(conflict) = conflicts.first() {
        return Err(ProjectChapterSceneReorderError::InvalidArgument(format!(
            "场景序号 {} 已被场景 #{} 占用",
            conflict.scene_number, conflict.id
        )));
    }

    let all_scenes = chapter_scene::Entity::find()
        .filter(chapter_scene::Column::ChapterId.eq(loaded.scope.chapter_id))
        .all(&txn)
        .await?;
    let min_scene_number = all_scenes
        .iter()
        .map(|scene| scene.scene_number)
        .min()
        .unwrap_or_default();
    let scene_count = i32::try_from(loaded.scenes.len()).map_err(|_| {
        ProjectChapterSceneReorderError::InvalidArgument(
            "章节场景重排数量超出 i32 范围".to_string(),
        )
    })?;
    let temp_start = min_scene_number
        .min(0)
        .checked_sub(scene_count)
        .and_then(|value| value.checked_sub(1))
        .ok_or_else(|| {
            ProjectChapterSceneReorderError::InvalidArgument(
                "章节场景重排临时序号超出 i32 范围".to_string(),
            )
        })?;
    let now = Utc::now().naive_utc();

    for (idx, (scene, _)) in loaded.scenes.iter().enumerate() {
        let mut active: chapter_scene::ActiveModel = scene.clone().into();
        let idx = i32::try_from(idx).map_err(|_| {
            ProjectChapterSceneReorderError::InvalidArgument(
                "章节场景重排索引超出 i32 范围".to_string(),
            )
        })?;
        active.scene_number = Set(temp_start.checked_add(idx).ok_or_else(|| {
            ProjectChapterSceneReorderError::InvalidArgument(
                "章节场景重排临时序号超出 i32 范围".to_string(),
            )
        })?);
        active.updated_at = Set(now);
        active.update(&txn).await?;
    }

    let mut updated_scenes = Vec::with_capacity(loaded.scenes.len());
    for (scene, new_number) in loaded.scenes {
        let mut active: chapter_scene::ActiveModel = scene.into();
        active.scene_number = Set(new_number);
        active.updated_at = Set(now);
        updated_scenes.push(active.update(&txn).await?);
    }

    txn.commit().await?;
    updated_scenes.sort_by_key(|scene| scene.scene_number);
    Ok(updated_scenes)
}

async fn load_chapter_scene_reorder<C>(
    db: &C,
    order: &[(i64, i32)],
) -> Result<LoadedChapterSceneReorder, ProjectChapterSceneReorderError>
where
    C: ConnectionTrait,
{
    let mut scenes = Vec::with_capacity(order.len());
    let mut scene_ids = BTreeSet::new();
    let mut target_numbers = BTreeSet::new();
    let mut chapter_id = None;

    for (scene_id, new_number) in order {
        if *new_number <= 0 {
            return Err(ProjectChapterSceneReorderError::InvalidArgument(
                "场景序号必须为正数".to_string(),
            ));
        }
        if !scene_ids.insert(*scene_id) {
            return Err(ProjectChapterSceneReorderError::InvalidArgument(format!(
                "场景排序包含重复场景: scene_id={scene_id}"
            )));
        }
        if !target_numbers.insert(*new_number) {
            return Err(ProjectChapterSceneReorderError::InvalidArgument(format!(
                "场景排序包含重复序号: scene_number={new_number}"
            )));
        }

        let scene = chapter_scene::Entity::find_by_id(*scene_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                ProjectChapterSceneReorderError::NotFound(format!(
                    "场景不存在: scene_id={scene_id}"
                ))
            })?;
        match chapter_id {
            Some(id) if id != scene.chapter_id => {
                return Err(ProjectChapterSceneReorderError::InvalidArgument(
                    "不能跨章节调整场景顺序".to_string(),
                ));
            }
            None => chapter_id = Some(scene.chapter_id),
            _ => {}
        }
        scenes.push((scene, *new_number));
    }

    let chapter_id = chapter_id.ok_or_else(|| {
        ProjectChapterSceneReorderError::InvalidArgument("场景排序为空".to_string())
    })?;
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectChapterSceneReorderError::NotFound(format!(
                "章节不存在: chapter_id={chapter_id}"
            ))
        })?;

    Ok(LoadedChapterSceneReorder {
        scope: ProjectChapterSceneReorderScope {
            chapter_id,
            project_id: chapter.project_id,
        },
        scenes,
        scene_ids,
        target_numbers: target_numbers.into_iter().collect(),
    })
}
