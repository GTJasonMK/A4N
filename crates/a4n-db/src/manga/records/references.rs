use crate::manga::{MangaPromptRecordError, manga_record_database, require_positive_manga_id};
use a4n_core::models::{
    ai::generated_image, character::character_portrait, project::novel_project,
};
use a4n_domain::manga::{MangaBoundReferenceAsset, MangaReferenceAssetKind};
use sea_orm::{DatabaseConnection, EntityTrait};

pub async fn manga_reference_bound_asset(
    db: &DatabaseConnection,
    project_id: i64,
    asset_kind: MangaReferenceAssetKind,
    asset_id: i64,
) -> Result<MangaBoundReferenceAsset, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(asset_id, "asset_id")?;
    let allowed_project_ids = allowed_reference_project_ids(db, project_id).await?;
    match asset_kind {
        MangaReferenceAssetKind::CharacterPortrait => {
            let portrait = character_portrait::Entity::find_by_id(asset_id)
                .one(db)
                .await
                .map_err(manga_record_database("读取漫画参考角色立绘失败"))?
                .ok_or_else(|| {
                    MangaPromptRecordError::NotFound(format!("角色立绘 #{asset_id} 不存在"))
                })?;
            if !allowed_project_ids.contains(&portrait.project_id) {
                return Err(MangaPromptRecordError::InvalidArgument(format!(
                    "角色立绘 #{asset_id} 不属于当前漫画项目或根文本项目"
                )));
            }
            if portrait.image_path.trim().is_empty() {
                return Err(MangaPromptRecordError::InvalidArgument(format!(
                    "角色立绘 #{asset_id} 缺少图片路径"
                )));
            }
            Ok(MangaBoundReferenceAsset {
                asset_kind,
                asset_id: portrait.id,
                image_path: portrait.image_path,
            })
        }
        MangaReferenceAssetKind::GeneratedImage => {
            let image = generated_image::Entity::find_by_id(asset_id)
                .one(db)
                .await
                .map_err(manga_record_database("读取漫画参考生成图片失败"))?
                .ok_or_else(|| {
                    MangaPromptRecordError::NotFound(format!("图片 #{asset_id} 不存在"))
                })?;
            if !allowed_project_ids.contains(&image.project_id) {
                return Err(MangaPromptRecordError::InvalidArgument(format!(
                    "图片 #{asset_id} 不属于当前漫画项目或根文本项目"
                )));
            }
            if image.file_path.trim().is_empty() {
                return Err(MangaPromptRecordError::InvalidArgument(format!(
                    "图片 #{asset_id} 缺少文件路径"
                )));
            }
            Ok(MangaBoundReferenceAsset {
                asset_kind,
                asset_id: image.id,
                image_path: image.file_path,
            })
        }
    }
}

async fn allowed_reference_project_ids(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<i64>, MangaPromptRecordError> {
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(manga_record_database("读取漫画参考素材项目失败"))?
        .ok_or_else(|| MangaPromptRecordError::NotFound(format!("项目不存在: {project_id}")))?;
    match project.project_kind.as_str() {
        "text" => Ok(vec![project.id]),
        "comic" => {
            let root_project_id = project.root_project_id.ok_or_else(|| {
                MangaPromptRecordError::InvalidArgument(format!(
                    "漫画项目缺少根文本项目: {}",
                    project.id
                ))
            })?;
            Ok(vec![project.id, root_project_id])
        }
        other => Err(MangaPromptRecordError::InvalidArgument(format!(
            "项目 {project_id} 不是文本项目或漫画项目: {other}"
        ))),
    }
}
