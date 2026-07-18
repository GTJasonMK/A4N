use super::super::{PROJECT_KIND_COMIC, PROJECT_KIND_TEXT};
use super::context::find_project_for_family;
use super::contracts::{MangaCharacterPortraitRefSummary, ProjectFamilyDbError};
use a4n_core::models::character::{character_portrait, protagonist_profile};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

pub async fn manga_character_portrait_refs(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<MangaCharacterPortraitRefSummary>, ProjectFamilyDbError> {
    let profile_project_id = manga_profile_project_id(db, project_id).await?;
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(profile_project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await?;
    if profiles.is_empty() {
        return Ok(Vec::new());
    }

    let portraits = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(profile_project_id))
        .order_by_desc(character_portrait::Column::IsPrimary)
        .order_by_desc(character_portrait::Column::CreatedAt)
        .all(db)
        .await?;
    if portraits.is_empty() {
        return Ok(Vec::new());
    }

    let mut character_portraits = Vec::new();
    for profile in profiles {
        let Some(portrait) = portraits
            .iter()
            .find(|portrait| portrait.profile_id == profile.id && portrait.is_primary)
            .or_else(|| {
                portraits
                    .iter()
                    .find(|portrait| portrait.profile_id == profile.id)
            })
        else {
            continue;
        };
        if portrait.image_path.trim().is_empty() {
            continue;
        }
        character_portraits.push(MangaCharacterPortraitRefSummary {
            profile_id: profile.id,
            portrait_id: portrait.id,
            character_name: profile.name.clone(),
            image_path: portrait.image_path.clone(),
            prompt: portrait.prompt.clone(),
            portrait_type: portrait.portrait_type.clone(),
            expression: portrait.expression.clone(),
            pose: portrait.pose.clone(),
            is_primary: portrait.is_primary,
        });
    }

    Ok(character_portraits)
}

async fn manga_profile_project_id(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<i64, ProjectFamilyDbError> {
    let project = find_project_for_family(db, project_id).await?;
    match project.project_kind.as_str() {
        PROJECT_KIND_TEXT => Ok(project.id),
        PROJECT_KIND_COMIC => project.root_project_id.ok_or_else(|| {
            ProjectFamilyDbError::InvalidArgument(format!("漫画项目缺少根文本项目: {}", project.id))
        }),
        other => Err(ProjectFamilyDbError::InvalidArgument(format!(
            "项目 {project_id} 不是文本项目或漫画项目: {other}"
        ))),
    }
}
