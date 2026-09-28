mod fact_feedback;
mod profiles;
mod projects;
mod scenes;
mod story;

use sea_orm::DatabaseTransaction;

use self::{profiles::insert_profile_rows, scenes::insert_scene_rows, story::insert_story_rows};
use crate::project::family::import::{
    ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage, map_required,
};

pub(in crate::project::family::import) use self::profiles::insert_scene_profiles;
pub(super) use self::projects::{insert_derived_projects, insert_root_project};

pub(super) async fn insert_text_core(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let root_project_id = map_required(
        &maps.project_ids,
        package.root_project_id,
        "文本核心 root_project_id",
    )?;
    insert_story_rows(txn, root_project_id, package, maps).await?;
    insert_profile_rows(txn, root_project_id, package, maps).await?;
    insert_scene_rows(txn, root_project_id, package, maps).await?;
    fact_feedback::insert_feedback_rows(txn, root_project_id, package, maps).await
}
