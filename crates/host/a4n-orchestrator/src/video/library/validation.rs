use a4n_domain::media::GeneratedImageRecord;
use std::collections::HashSet;

use a4n_db::profile_material::is_project_reusable_video_material_image;
use a4n_error::AppError;
use serde_json::Value;

pub(in crate::video) fn available_video_image_ids(
    available_assets: &Value,
    extra_ids: &[i64],
) -> Result<HashSet<i64>, AppError> {
    let mut ids = HashSet::new();
    let mut seen_extra_ids = HashSet::new();
    for (index, id) in extra_ids.iter().copied().enumerate() {
        if id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "reference_image_ids[{index}] 必须为正整数"
            )));
        }
        if !seen_extra_ids.insert(id) {
            return Err(AppError::InvalidArgument(format!(
                "reference_image_ids 包含重复 ID：{id}"
            )));
        }
        ids.insert(id);
    }

    let items = available_assets.as_array().ok_or_else(|| {
        AppError::InvalidArgument("available_assets 必须是视频素材数组".to_string())
    })?;
    let mut seen_asset_ids = HashSet::new();
    for (index, item) in items.iter().enumerate() {
        let id = item
            .get("image_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "available_assets[{index}].image_id 必须为正整数"
                ))
            })?;
        if id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "available_assets[{index}].image_id 必须为正整数"
            )));
        }
        if !seen_asset_ids.insert(id) {
            return Err(AppError::InvalidArgument(format!(
                "available_assets 包含重复图片 ID：{id}"
            )));
        }
        ids.insert(id);
    }
    Ok(ids)
}

pub(in crate::video) fn ensure_video_reference_ids_known(
    ids: &[i64],
    allowed_ids: &HashSet<i64>,
    context: &str,
) -> Result<(), AppError> {
    if let Some(id) = ids.iter().find(|id| !allowed_ids.contains(id)) {
        return Err(AppError::InvalidArgument(format!(
            "{context} 引用了不在当前视频素材库中的图片 ID：{id}"
        )));
    }
    Ok(())
}

pub(super) fn ensure_video_asset_image_usable(
    project_id: i64,
    chapter_id: i64,
    allowed_project_ids: &[i64],
    image: &GeneratedImageRecord,
) -> Result<(), AppError> {
    if !allowed_project_ids.contains(&image.project_id) {
        return Err(AppError::InvalidArgument(format!(
            "视频素材库不能引用不属于视频项目 {project_id} 或其根文本项目的图片 #{}",
            image.id
        )));
    }
    if image.chapter_id.is_some_and(|id| id != chapter_id)
        && !is_project_reusable_video_material_image(
            &image.image_type,
            image.metadata_json.as_ref(),
        )
    {
        return Err(AppError::InvalidArgument(format!(
            "视频素材库不能引用其他章节的图片 #{}",
            image.id
        )));
    }
    Ok(())
}
