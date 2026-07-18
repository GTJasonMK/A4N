use crate::manga::{
    MangaPromptRecordError, manga_record_database, require_manga_text, require_positive_manga_id,
};
use a4n_core::models::{
    ai::generated_image,
    character::{character_portrait, protagonist_profile},
    project::novel_project,
};
use sea_orm::{DatabaseConnection, EntityTrait};
use serde_json::{Value, json};

pub async fn manga_reference_bound_asset(
    db: &DatabaseConnection,
    project_id: i64,
    asset_kind: &str,
    asset_id: i64,
) -> Result<Value, MangaPromptRecordError> {
    require_positive_manga_id(project_id, "project_id")?;
    require_positive_manga_id(asset_id, "asset_id")?;
    let asset_kind = require_manga_text(asset_kind, "asset_kind")?;
    let allowed_project_ids = allowed_reference_project_ids(db, project_id).await?;
    match asset_kind {
        "character_portrait" => {
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
            let profile = protagonist_profile::Entity::find_by_id(portrait.profile_id)
                .one(db)
                .await
                .map_err(manga_record_database("读取漫画参考角色档案失败"))?;
            let label = profile
                .as_ref()
                .map(|profile| profile.name.trim())
                .filter(|name| !name.is_empty())
                .unwrap_or("角色立绘");
            Ok(json!({
                "asset_kind": "character_portrait",
                "asset_id": portrait.id,
                "portrait_id": portrait.id,
                "profile_id": portrait.profile_id,
                "label": label,
                "image_path": portrait.image_path,
                "prompt": portrait.prompt,
                "portrait_type": portrait.portrait_type,
                "expression": portrait.expression,
                "pose": portrait.pose,
                "is_primary": portrait.is_primary,
            }))
        }
        "generated_image" => {
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
            Ok(json!({
                "asset_kind": "generated_image",
                "asset_id": image.id,
                "image_id": image.id,
                "label": format!("{} #{}", image.image_type, image.id),
                "image_type": image.image_type,
                "image_path": image.file_path,
                "prompt": image.prompt,
                "chapter_id": image.chapter_id,
                "panel_id": image.panel_id,
                "is_favorite": image.is_favorite,
            }))
        }
        other => Err(MangaPromptRecordError::InvalidArgument(format!(
            "不支持的参考图素材类型: {other}，只能是 character_portrait 或 generated_image"
        ))),
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
