use std::collections::BTreeSet;

use a4n_db_models::models::{
    ai::generated_image, chapter::chapter_visual_plan, project::novel_project, scene::scene_profile,
};
use a4n_domain::chapter_visual_plan::{CHAPTER_VISUAL_PLAN_V3, ChapterVisualPlanV3};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

use super::ChapterVisualPlanDbError;

pub async fn visual_plan_prompt_candidate_record<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    run_id: i64,
) -> Result<Option<a4n_db_models::models::generation::generation_run::Model>, DbErr> {
    use a4n_db_models::models::generation::generation_run;
    generation_run::Entity::find_by_id(run_id)
        .filter(generation_run::Column::ProjectId.eq(project_id))
        .filter(
            generation_run::Column::Workflow
                .eq(a4n_domain::chapter_visual_plan::DIRECTOR_SEGMENT_PROMPT),
        )
        .filter(generation_run::Column::Status.eq("completed"))
        .one(db)
        .await
}

pub(super) async fn validate_resources<C: ConnectionTrait>(
    db: &C,
    root_project_id: i64,
    plan: &ChapterVisualPlanV3,
) -> Result<(), ChapterVisualPlanDbError> {
    let mut query = novel_project::Entity::find()
        .filter(
            novel_project::Column::Id
                .eq(root_project_id)
                .or(novel_project::Column::RootProjectId.eq(root_project_id)),
        )
        .order_by_asc(novel_project::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_shared();
    }
    let family = query
        .all(db)
        .await?
        .into_iter()
        .map(|project| project.id)
        .collect::<Vec<_>>();
    let images = plan.referenced_image_ids();
    if !images.is_empty() {
        let mut query = generated_image::Entity::find()
            .filter(generated_image::Column::Id.is_in(images.iter().copied()))
            .filter(generated_image::Column::ProjectId.is_in(family.clone()))
            .order_by_asc(generated_image::Column::Id);
        if db.get_database_backend() != DbBackend::Sqlite {
            query = query.lock_shared();
        }
        if query.all(db).await?.len() != images.len() {
            return Err(ChapterVisualPlanDbError::InvalidArgument(
                "绑定图片不存在或不属于当前项目族".into(),
            ));
        }
    }
    let scenes = plan.referenced_scene_ids();
    if !scenes.is_empty() {
        let mut query = scene_profile::Entity::find()
            .filter(scene_profile::Column::Id.is_in(scenes.iter().copied()))
            .filter(scene_profile::Column::ProjectId.is_in(family))
            .order_by_asc(scene_profile::Column::Id);
        if db.get_database_backend() != DbBackend::Sqlite {
            query = query.lock_shared();
        }
        if query.all(db).await?.len() != scenes.len() {
            return Err(ChapterVisualPlanDbError::InvalidArgument(
                "绑定场景不存在或不属于当前项目族".into(),
            ));
        }
    }
    Ok(())
}

/// Retained versions and snapshots must remain exportable after a new plan is confirmed.
pub async fn visual_plan_resource_ids<C: ConnectionTrait>(
    db: &C,
) -> Result<(BTreeSet<i64>, BTreeSet<i64>), DbErr> {
    // ponytail: scan retained plans; add a JSON-reference index if deletion becomes a hot path.
    let records = chapter_visual_plan::Entity::find()
        .filter(chapter_visual_plan::Column::ContractVersion.eq(CHAPTER_VISUAL_PLAN_V3))
        .all(db)
        .await?;
    let mut images = BTreeSet::new();
    let mut scenes = BTreeSet::new();
    for record in records {
        let plan: ChapterVisualPlanV3 =
            serde_json::from_value(record.plan_json).map_err(|error| {
                DbErr::Custom(format!("导演编排 {} 引用解析失败: {error}", record.id))
            })?;
        plan.validate_draft()
            .map_err(|error| DbErr::Custom(error.to_string()))?;
        images.extend(plan.referenced_image_ids());
        scenes.extend(plan.referenced_scene_ids());
    }
    Ok((images, scenes))
}

pub async fn ensure_visual_plan_images_unreferenced<C: ConnectionTrait>(
    db: &C,
    ids: &[i64],
) -> Result<(), DbErr> {
    if ids.is_empty() {
        return Ok(());
    }
    let (images, _) = visual_plan_resource_ids(db).await?;
    if ids.iter().any(|id| images.contains(id)) {
        return Err(DbErr::Custom(
            "图片仍被导演编排引用，请先解除分镜图片或参考图绑定".into(),
        ));
    }
    Ok(())
}

pub async fn ensure_visual_plan_scenes_unreferenced<C: ConnectionTrait>(
    db: &C,
    ids: &[i64],
) -> Result<(), DbErr> {
    if ids.is_empty() {
        return Ok(());
    }
    let (_, scenes) = visual_plan_resource_ids(db).await?;
    if ids.iter().any(|id| scenes.contains(id)) {
        return Err(DbErr::Custom(
            "场景仍被导演编排引用，请先调整镜头段的场景绑定".into(),
        ));
    }
    Ok(())
}
