use super::contracts::{MangaCharacterPortraitRefSummary, ProjectFamilyDbError};
use super::project_family_access_row;
use a4n_db_models::models::character::{character_portrait, protagonist_profile};
use a4n_domain::project::{ProjectKind, ProjectSourceMode};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::HashMap;

pub async fn manga_character_portrait_refs(
    db: &impl crate::DbExecutor,
    project_id: i64,
) -> Result<Vec<MangaCharacterPortraitRefSummary>, ProjectFamilyDbError> {
    let profile_project_id = manga_profile_project_id(db, project_id).await?;
    let (profiles_result, portraits_result) = futures::join!(
        protagonist_profile::Entity::find()
            .select_only()
            .column(protagonist_profile::Column::Id)
            .column(protagonist_profile::Column::Name)
            .filter(protagonist_profile::Column::ProjectId.eq(profile_project_id))
            .order_by_asc(protagonist_profile::Column::Id)
            .into_tuple::<(i64, String)>()
            .all(db),
        character_portrait::Entity::find()
            .filter(character_portrait::Column::ProjectId.eq(profile_project_id))
            .all(db),
    );
    let profiles = profiles_result?;
    if profiles.is_empty() {
        return Ok(Vec::new());
    }

    let portraits = portraits_result?;
    if portraits.is_empty() {
        return Ok(Vec::new());
    }

    let portrait_by_profile = portrait_index(&portraits);

    let mut character_portraits = Vec::new();
    for (profile_id, profile_name) in profiles {
        let Some(portrait) = portrait_by_profile.get(&profile_id) else {
            continue;
        };
        let portrait = portrait.as_ref().ok_or_else(|| {
            ProjectFamilyDbError::InvalidArgument(format!(
                "角色档案 #{} 已有立绘但没有主立绘",
                profile_id
            ))
        })?;
        if portrait.image_path.trim().is_empty() {
            return Err(ProjectFamilyDbError::InvalidArgument(format!(
                "角色档案 #{} 的主立绘 #{} 缺少图片路径",
                profile_id, portrait.id
            )));
        }
        if profile_name.trim().is_empty() {
            return Err(ProjectFamilyDbError::InvalidArgument(format!(
                "角色档案 #{} 名称为空",
                profile_id
            )));
        }
        if portrait.prompt.trim().is_empty() {
            return Err(ProjectFamilyDbError::InvalidArgument(format!(
                "角色档案 #{} 的主立绘 #{} 缺少提示词",
                profile_id, portrait.id
            )));
        }
        if portrait.portrait_type.trim().is_empty() {
            return Err(ProjectFamilyDbError::InvalidArgument(format!(
                "角色档案 #{} 的主立绘 #{} 缺少立绘类型",
                profile_id, portrait.id
            )));
        }
        character_portraits.push(MangaCharacterPortraitRefSummary {
            profile_id,
            portrait_id: portrait.id,
            character_name: profile_name,
            image_path: portrait.image_path.clone(),
            prompt: portrait.prompt.clone(),
            portrait_type: portrait.portrait_type.clone(),
            expression: portrait.expression.clone(),
            pose: portrait.pose.clone(),
        });
    }

    Ok(character_portraits)
}

fn portrait_index(
    portraits: &[character_portrait::Model],
) -> HashMap<i64, Option<&character_portrait::Model>> {
    let mut index: HashMap<i64, Option<&character_portrait::Model>> =
        HashMap::with_capacity(portraits.len());
    for portrait in portraits {
        let current = index.entry(portrait.profile_id).or_insert(None);
        if portrait.is_primary
            && current
                .as_ref()
                .is_none_or(|current| portrait.created_at > current.created_at)
        {
            *current = Some(portrait);
        }
    }
    index
}

async fn manga_profile_project_id(
    db: &impl crate::DbExecutor,
    project_id: i64,
) -> Result<i64, ProjectFamilyDbError> {
    let project = project_family_access_row(db, project_id).await?;
    manga_profile_project_id_for_identity(
        project.id,
        &project.project_kind,
        project.root_project_id,
        project.source_snapshot_id,
    )
}

fn manga_profile_project_id_for_identity(
    project_id: i64,
    project_kind: &str,
    root_project_id: Option<i64>,
    source_snapshot_id: Option<i64>,
) -> Result<i64, ProjectFamilyDbError> {
    match ProjectKind::parse(project_kind) {
        Some(ProjectKind::Text) => Ok(project_id),
        Some(ProjectKind::Comic) => {
            let source_mode =
                ProjectSourceMode::from_identity(project_kind, root_project_id, source_snapshot_id)
                    .map_err(|error| {
                        ProjectFamilyDbError::InvalidArgument(format!("项目 {project_id}: {error}"))
                    })?;
            match source_mode {
                ProjectSourceMode::Standalone => Ok(project_id),
                ProjectSourceMode::Derived => root_project_id.ok_or_else(|| {
                    ProjectFamilyDbError::InvalidArgument(format!(
                        "漫画项目缺少根文本项目: {}",
                        project_id
                    ))
                }),
                _ => Err(ProjectFamilyDbError::InvalidArgument(format!(
                    "漫画项目 {} 的来源模式无效",
                    project_id
                ))),
            }
        }
        _ => Err(ProjectFamilyDbError::InvalidArgument(format!(
            "项目 {project_id} 不是文本项目或漫画项目: {project_kind}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{manga_profile_project_id_for_identity, portrait_index};
    use a4n_db_models::models::character::character_portrait;
    use chrono::{Duration, Utc};

    fn portrait(id: i64, profile_id: i64, is_primary: bool) -> character_portrait::Model {
        character_portrait::Model {
            id,
            profile_id,
            project_id: 1,
            portrait_type: String::new(),
            image_path: String::new(),
            prompt: String::new(),
            expression: None,
            pose: None,
            is_primary,
            created_at: Utc::now().naive_utc(),
        }
    }

    #[test]
    fn portrait_index_keeps_presence_and_newest_primary_without_input_order() {
        let now = Utc::now().naive_utc();
        let mut older_primary = portrait(2, 10, true);
        older_primary.created_at = now;
        let mut newer_primary = portrait(3, 10, true);
        newer_primary.created_at = now + Duration::seconds(1);
        let portraits = vec![
            portrait(1, 10, false),
            newer_primary,
            older_primary,
            portrait(4, 11, false),
        ];
        let index = portrait_index(&portraits);

        assert_eq!(
            index
                .get(&10)
                .and_then(Option::as_ref)
                .map(|portrait| portrait.id),
            Some(3)
        );
        assert!(matches!(index.get(&11), Some(None)));
        assert!(!index.contains_key(&12));
    }

    #[test]
    fn manga_profile_project_id_respects_comic_source_mode() {
        assert_eq!(
            manga_profile_project_id_for_identity(12, "comic", None, None).unwrap(),
            12
        );
        assert_eq!(
            manga_profile_project_id_for_identity(12, "comic", Some(3), Some(5)).unwrap(),
            3
        );
        assert!(manga_profile_project_id_for_identity(12, "comic", Some(3), None).is_err());
    }
}
