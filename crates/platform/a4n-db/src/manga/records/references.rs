use crate::manga::{MangaPromptRecordError, manga_record_database, require_positive_manga_id};
use a4n_db_models::models::{
    ai::generated_image, character::character_portrait, project::novel_project,
};
use a4n_domain::manga::{MangaBoundReferenceAsset, MangaReferenceAssetKind};
use a4n_domain::project::ProjectSourceMode;
use sea_orm::{EntityTrait, QuerySelect};

pub async fn manga_reference_bound_asset(
    db: &impl crate::DbExecutor,
    project_id: i64,
    asset_kind: MangaReferenceAssetKind,
    asset_id: i64,
) -> Result<MangaBoundReferenceAsset, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(asset_id, "asset_id")?;
    let (allowed_project_ids_result, asset_result) = futures::join!(
        allowed_reference_project_ids(db, project_id),
        reference_asset_projection(db, asset_kind, asset_id),
    );
    let allowed_project_ids = allowed_project_ids_result?;
    let (asset_id, asset_project_id, image_path) = asset_result?;
    if !allowed_project_ids.contains(&asset_project_id) {
        let asset_label = match asset_kind {
            MangaReferenceAssetKind::CharacterPortrait => "角色立绘",
            MangaReferenceAssetKind::GeneratedImage => "图片",
        };
        return Err(MangaPromptRecordError::InvalidArgument(format!(
            "{asset_label} #{asset_id} 不属于当前漫画项目或根文本项目"
        )));
    }
    let path_label = match asset_kind {
        MangaReferenceAssetKind::CharacterPortrait => "角色立绘",
        MangaReferenceAssetKind::GeneratedImage => "图片",
    };
    if image_path.trim().is_empty() {
        return Err(MangaPromptRecordError::InvalidArgument(format!(
            "{path_label} #{asset_id} 缺少{}",
            match asset_kind {
                MangaReferenceAssetKind::CharacterPortrait => "图片路径",
                MangaReferenceAssetKind::GeneratedImage => "文件路径",
            }
        )));
    }
    Ok(MangaBoundReferenceAsset {
        asset_kind,
        asset_id,
        image_path,
    })
}

async fn reference_asset_projection(
    db: &impl crate::DbExecutor,
    asset_kind: MangaReferenceAssetKind,
    asset_id: i64,
) -> Result<(i64, i64, String), MangaPromptRecordError> {
    match asset_kind {
        MangaReferenceAssetKind::CharacterPortrait => {
            let portrait = character_portrait::Entity::find_by_id(asset_id)
                .select_only()
                .column(character_portrait::Column::Id)
                .column(character_portrait::Column::ProjectId)
                .column(character_portrait::Column::ImagePath)
                .into_tuple::<(i64, i64, String)>()
                .one(db)
                .await
                .map_err(manga_record_database("读取漫画参考角色立绘失败"))?
                .ok_or_else(|| {
                    MangaPromptRecordError::NotFound(format!("角色立绘 #{asset_id} 不存在"))
                })?;
            Ok(portrait)
        }
        MangaReferenceAssetKind::GeneratedImage => {
            let image = generated_image::Entity::find_by_id(asset_id)
                .select_only()
                .column(generated_image::Column::Id)
                .column(generated_image::Column::ProjectId)
                .column(generated_image::Column::FilePath)
                .into_tuple::<(i64, i64, String)>()
                .one(db)
                .await
                .map_err(manga_record_database("读取漫画参考生成图片失败"))?
                .ok_or_else(|| {
                    MangaPromptRecordError::NotFound(format!("图片 #{asset_id} 不存在"))
                })?;
            Ok(image)
        }
    }
}

async fn allowed_reference_project_ids(
    db: &impl crate::DbExecutor,
    project_id: i64,
) -> Result<Vec<i64>, MangaPromptRecordError> {
    let project = novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::RootProjectId)
        .column(novel_project::Column::SourceSnapshotId)
        .into_tuple::<(i64, String, Option<i64>, Option<i64>)>()
        .one(db)
        .await
        .map_err(manga_record_database("读取漫画参考素材项目失败"))?
        .ok_or_else(|| MangaPromptRecordError::NotFound(format!("项目不存在: {project_id}")))?;
    reference_project_scope_for_identity(project.0, &project.1, project.2, project.3)
}

fn reference_project_scope_for_identity(
    project_id: i64,
    project_kind: &str,
    root_project_id: Option<i64>,
    source_snapshot_id: Option<i64>,
) -> Result<Vec<i64>, MangaPromptRecordError> {
    match project_kind {
        "text" => Ok(vec![project_id]),
        "comic" => {
            let source_mode =
                ProjectSourceMode::from_identity(project_kind, root_project_id, source_snapshot_id)
                    .map_err(|error| {
                        MangaPromptRecordError::InvalidArgument(format!(
                            "项目 {project_id}: {error}"
                        ))
                    })?;
            match source_mode {
                ProjectSourceMode::Standalone => Ok(vec![project_id]),
                ProjectSourceMode::Derived => {
                    let root_project_id = root_project_id.ok_or_else(|| {
                        MangaPromptRecordError::InvalidArgument(format!(
                            "漫画项目缺少根文本项目: {}",
                            project_id
                        ))
                    })?;
                    Ok(vec![project_id, root_project_id])
                }
                _ => Err(MangaPromptRecordError::InvalidArgument(format!(
                    "漫画项目 {} 的来源模式无效",
                    project_id
                ))),
            }
        }
        other => Err(MangaPromptRecordError::InvalidArgument(format!(
            "项目 {project_id} 不是文本项目或漫画项目: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::reference_project_scope_for_identity;

    #[test]
    fn standalone_comic_references_are_limited_to_the_comic_project() {
        assert_eq!(
            reference_project_scope_for_identity(12, "comic", None, None).unwrap(),
            vec![12]
        );
    }

    #[test]
    fn derived_comic_references_include_the_text_root() {
        assert_eq!(
            reference_project_scope_for_identity(12, "comic", Some(3), Some(5)).unwrap(),
            vec![12, 3]
        );
    }

    #[test]
    fn invalid_comic_reference_identity_is_rejected() {
        assert!(reference_project_scope_for_identity(12, "comic", Some(3), None).is_err());
    }
}
