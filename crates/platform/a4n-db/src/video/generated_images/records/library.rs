use super::super::{
    VideoProfileMaterialReadModel, VideoProfilePrimaryMaterialRow, VideoProfileRelevanceRow,
    VideoProjectContextReadModel, VideoPromptBlueprintCharacterRecord,
    VideoPromptProtagonistProfileRecord,
};
use crate::video::generated_image_record_from_model;
use a4n_db_models::models::{
    ai::generated_image,
    asset::{object_profile, profile_primary_material},
    character::protagonist_profile,
    project::{blueprint_character, novel_blueprint},
    scene::scene_profile,
};
use a4n_domain::image::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    sea_query::Expr,
};

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
        .filter(Expr::cust(
            "(metadata_json ->> 'chapter_video_material') = 'true'",
        ))
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
    let (scenes_result, objects_result, primary_materials_result, material_images_result) = futures::join!(
        scene_profile::Entity::find()
            .select_only()
            .column(scene_profile::Column::Id)
            .column(scene_profile::Column::Name)
            .column(scene_profile::Column::AliasesJson)
            .column(scene_profile::Column::FirstSeenChapter)
            .column(scene_profile::Column::LastSeenChapter)
            .filter(scene_profile::Column::ProjectId.is_in(project_ids.iter().copied()))
            .into_tuple::<(i64, String, serde_json::Value, i32, i32)>()
            .all(db),
        object_profile::Entity::find()
            .select_only()
            .column(object_profile::Column::Id)
            .column(object_profile::Column::Name)
            .column(object_profile::Column::AliasesJson)
            .column(object_profile::Column::FirstSeenChapter)
            .column(object_profile::Column::LastSeenChapter)
            .filter(object_profile::Column::ProjectId.is_in(project_ids.iter().copied()))
            .into_tuple::<(i64, String, serde_json::Value, i32, i32)>()
            .all(db),
        profile_primary_material::Entity::find()
            .select_only()
            .column(profile_primary_material::Column::ProfileKind)
            .column(profile_primary_material::Column::ProfileId)
            .column(profile_primary_material::Column::PrimaryImageId)
            .filter(profile_primary_material::Column::ProjectId.is_in(project_ids.iter().copied()))
            .into_tuple::<(String, i64, i64)>()
            .all(db),
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.is_in(project_ids.iter().copied()))
            .filter(
                generated_image::Column::ImageType
                    .is_in([IMAGE_TYPE_SCENE_MATERIAL, IMAGE_TYPE_OBJECT_MATERIAL]),
            )
            .order_by_desc(generated_image::Column::CreatedAt)
            .order_by_desc(generated_image::Column::Id)
            .all(db),
    );
    let scenes = scenes_result?
        .into_iter()
        .map(
            |(id, name, aliases_json, first_seen_chapter, last_seen_chapter)| {
                VideoProfileRelevanceRow {
                    id,
                    name,
                    aliases_json,
                    first_seen_chapter,
                    last_seen_chapter,
                }
            },
        )
        .collect();
    let objects = objects_result?
        .into_iter()
        .map(
            |(id, name, aliases_json, first_seen_chapter, last_seen_chapter)| {
                VideoProfileRelevanceRow {
                    id,
                    name,
                    aliases_json,
                    first_seen_chapter,
                    last_seen_chapter,
                }
            },
        )
        .collect();
    let primary_materials = primary_materials_result?
        .into_iter()
        .map(
            |(profile_kind, profile_id, primary_image_id)| VideoProfilePrimaryMaterialRow {
                profile_kind,
                profile_id,
                primary_image_id,
            },
        )
        .collect();
    let material_images = material_images_result?
        .into_iter()
        .map(generated_image_record_from_model)
        .collect();

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
    let (blueprint_result, blueprint_characters_result, protagonist_profiles_result) = futures::join!(
        novel_blueprint::Entity::find()
            .filter(novel_blueprint::Column::ProjectId.eq(project_id))
            .one(db),
        blueprint_character::Entity::find()
            .select_only()
            .column(blueprint_character::Column::Name)
            .column(blueprint_character::Column::CharacterType)
            .column(blueprint_character::Column::Gender)
            .column(blueprint_character::Column::Age)
            .column(blueprint_character::Column::Appearance)
            .column(blueprint_character::Column::Personality)
            .column(blueprint_character::Column::Background)
            .column(blueprint_character::Column::Motivation)
            .column(blueprint_character::Column::RelationshipToProtagonist)
            .filter(blueprint_character::Column::ProjectId.eq(project_id))
            .order_by_asc(blueprint_character::Column::Id)
            .into_model::<VideoPromptBlueprintCharacterRecord>()
            .all(db),
        protagonist_profile::Entity::find()
            .select_only()
            .column(protagonist_profile::Column::Name)
            .column(protagonist_profile::Column::Gender)
            .column(protagonist_profile::Column::Age)
            .column(protagonist_profile::Column::BaseAttributes)
            .column(protagonist_profile::Column::CurrentState)
            .column(protagonist_profile::Column::Appearance)
            .column(protagonist_profile::Column::Background)
            .column(protagonist_profile::Column::Personality)
            .filter(protagonist_profile::Column::ProjectId.eq(project_id))
            .order_by_asc(protagonist_profile::Column::Id)
            .into_model::<VideoPromptProtagonistProfileRecord>()
            .all(db),
    );
    let blueprint = blueprint_result?.map(crate::project::blueprint_record);
    let blueprint_characters = blueprint_characters_result?;
    let protagonist_profiles = protagonist_profiles_result?;

    Ok(VideoProjectContextReadModel {
        blueprint,
        blueprint_characters,
        protagonist_profiles,
    })
}
