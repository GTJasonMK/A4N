use super::super::{VideoProfileMaterialReadModel, VideoProjectContextReadModel};
use a4n_core::models::{
    ai::generated_image,
    asset::{object_profile, profile_primary_material},
    character::protagonist_profile,
    project::{blueprint_character, novel_blueprint},
    scene::scene_profile,
};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub async fn video_library_explicit_image_records<C>(
    db: &C,
    project_ids: &[i64],
    image_ids: &[i64],
) -> Result<Vec<generated_image::Model>, DbErr>
where
    C: ConnectionTrait,
{
    if project_ids.is_empty() || image_ids.is_empty() {
        return Ok(Vec::new());
    }
    generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(generated_image::Column::Id.is_in(image_ids.iter().copied()))
        .order_by_asc(generated_image::Column::CreatedAt)
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
}

pub async fn video_library_chapter_image_records<C>(
    db: &C,
    project_ids: &[i64],
    chapter_id: i64,
) -> Result<Vec<generated_image::Model>, DbErr>
where
    C: ConnectionTrait,
{
    if project_ids.is_empty() {
        return Ok(Vec::new());
    }
    generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .order_by_asc(generated_image::Column::CreatedAt)
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
}

pub async fn video_profile_material_read_model<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<VideoProfileMaterialReadModel, DbErr>
where
    C: ConnectionTrait,
{
    if project_ids.is_empty() {
        return Ok(VideoProfileMaterialReadModel::default());
    }
    let scenes = scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.is_in(project_ids.iter().copied()))
        .all(db)
        .await?;
    let objects = object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.is_in(project_ids.iter().copied()))
        .all(db)
        .await?;
    let primary_materials = profile_primary_material::Entity::find()
        .filter(profile_primary_material::Column::ProjectId.is_in(project_ids.iter().copied()))
        .all(db)
        .await?;
    let material_images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
        .order_by_desc(generated_image::Column::CreatedAt)
        .order_by_desc(generated_image::Column::Id)
        .all(db)
        .await?;

    Ok(VideoProfileMaterialReadModel {
        scenes,
        objects,
        primary_materials,
        material_images,
    })
}

pub async fn video_project_context_read_model<C>(
    db: &C,
    project_id: i64,
) -> Result<VideoProjectContextReadModel, DbErr>
where
    C: ConnectionTrait,
{
    let blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await?;
    let blueprint_characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(db)
        .await?;
    let protagonist_profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await?;

    Ok(VideoProjectContextReadModel {
        blueprint,
        blueprint_characters,
        protagonist_profiles,
    })
}
