use a4n_core::{
    error::AppError,
    models::{
        asset::object_profile,
        chapter::{chapter, chapter_outline, chapter_shooting_script, part_outline},
        character::protagonist_profile,
        project::{blueprint_version, novel_blueprint},
        scene::scene_profile,
        user::conversation,
    },
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub(super) struct RootAssetGraphRecords {
    pub(super) conversations: Vec<conversation::Model>,
    pub(super) blueprints: Vec<novel_blueprint::Model>,
    pub(super) blueprint_versions: Vec<blueprint_version::Model>,
    pub(super) part_outlines: Vec<part_outline::Model>,
    pub(super) chapter_outlines: Vec<chapter_outline::Model>,
    pub(super) chapters: Vec<chapter::Model>,
    pub(super) shooting_scripts: Vec<chapter_shooting_script::Model>,
    pub(super) character_profiles: Vec<protagonist_profile::Model>,
    pub(super) object_profiles: Vec<object_profile::Model>,
    pub(super) scene_profiles: Vec<scene_profile::Model>,
}

pub(super) async fn load_root_asset_graph_records(
    db: &impl ConnectionTrait,
    root_project_id: i64,
) -> Result<RootAssetGraphRecords, AppError> {
    let conversations = conversation::Entity::find()
        .filter(conversation::Column::ProjectId.eq(root_project_id))
        .order_by_asc(conversation::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let blueprints = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(root_project_id))
        .order_by_asc(novel_blueprint::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let blueprint_versions = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(root_project_id))
        .order_by_asc(blueprint_version::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let part_outlines = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(root_project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let chapter_outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(root_project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(root_project_id))
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let shooting_scripts = chapter_shooting_script::Entity::find()
        .filter(chapter_shooting_script::Column::RootProjectId.eq(root_project_id))
        .order_by_asc(chapter_shooting_script::Column::SourceChapterId)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let character_profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let object_profiles = object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(root_project_id))
        .order_by_asc(object_profile::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let scene_profiles = scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(root_project_id))
        .order_by_asc(scene_profile::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;

    Ok(RootAssetGraphRecords {
        conversations,
        blueprints,
        blueprint_versions,
        part_outlines,
        chapter_outlines,
        chapters,
        shooting_scripts,
        character_profiles,
        object_profiles,
        scene_profiles,
    })
}
