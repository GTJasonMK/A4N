use super::context::video_asset_project_ids_record;
use crate::video::{
    IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL, VideoDbError,
    generated_image_records_by_ids,
};
use a4n_core::models::ai::generated_image;
use sea_orm::ConnectionTrait;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub async fn ensure_video_reference_images_available_record<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    reference_image_ids_json: Option<&Value>,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    validated_video_reference_image_records(db, project_id, chapter_id, reference_image_ids_json)
        .await
        .map(|_| ())
}

pub async fn validated_video_reference_image_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    reference_image_ids_json: Option<&Value>,
) -> Result<Vec<generated_image::Model>, VideoDbError>
where
    C: ConnectionTrait,
{
    let ids = prompt_reference_image_ids(reference_image_ids_json)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let allowed_project_ids = video_asset_project_ids_record(db, project_id)
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let rows = generated_image_records_by_ids(db, &ids).await?;
    let mut images_by_id = rows
        .into_iter()
        .map(|image| (image.id, image))
        .collect::<BTreeMap<_, _>>();

    let mut images = Vec::with_capacity(ids.len());
    for id in ids {
        let image = images_by_id
            .remove(&id)
            .ok_or_else(|| VideoDbError::NotFound(format!("参考图片不存在: {id}")))?;
        if !allowed_project_ids.contains(&image.project_id) {
            return Err(VideoDbError::InvalidArgument(format!(
                "参考图片 #{id} 不属于视频项目 {project_id} 或其根文本项目"
            )));
        }
        if is_project_reusable_video_material_image_record(
            &image.image_type,
            image.metadata_json.as_ref(),
        ) {
            images.push(image);
            continue;
        }
        if image.chapter_id.is_some() && image.chapter_id != Some(chapter_id) {
            return Err(VideoDbError::InvalidArgument(format!(
                "参考图片 #{id} 不属于提示词章节 {chapter_id}"
            )));
        }
        images.push(image);
    }
    Ok(images)
}

fn prompt_reference_image_ids(value: Option<&Value>) -> Result<Vec<i64>, VideoDbError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = value.as_array().ok_or_else(|| {
        VideoDbError::InvalidArgument("reference_image_ids 必须是数组".to_string())
    })?;
    let mut ids = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let id = value.as_i64().filter(|id| *id > 0).ok_or_else(|| {
            VideoDbError::InvalidArgument(format!("reference_image_ids[{index}] 必须是正整数"))
        })?;
        if ids.contains(&id) {
            return Err(VideoDbError::InvalidArgument(format!(
                "reference_image_ids 包含重复 ID：{id}"
            )));
        }
        ids.push(id);
    }
    Ok(ids)
}

fn is_project_reusable_video_material_image_record(
    image_type: &str,
    metadata: Option<&Value>,
) -> bool {
    !is_video_frame_input_metadata_record(metadata)
        && profile_material_identity_record(image_type, metadata).is_some()
}

fn profile_material_identity_record(image_type: &str, metadata: Option<&Value>) -> Option<i64> {
    match image_type {
        IMAGE_TYPE_SCENE_MATERIAL if metadata_profile_kind(metadata) == Some("scene") => {
            metadata_number(metadata, "profile_id")
        }
        IMAGE_TYPE_OBJECT_MATERIAL if metadata_profile_kind(metadata) == Some("object") => {
            metadata_number(metadata, "profile_id")
        }
        _ => None,
    }
}

fn is_video_frame_input_metadata_record(metadata: Option<&Value>) -> bool {
    let Some(metadata) = metadata.and_then(Value::as_object) else {
        return false;
    };
    metadata
        .get("generation_context")
        .and_then(Value::as_object)
        .and_then(|context| context.get("source"))
        .and_then(Value::as_str)
        .is_some_and(|source| {
            matches!(
                source.trim(),
                "video_boundary_frame_prompt"
                    | "video_first_frame_prompt"
                    | "video_tail_frame_capture"
            )
        })
        || metadata
            .get("video_frame_role")
            .and_then(Value::as_str)
            .is_some_and(|role| matches!(role.trim(), "key_frame" | "start_frame" | "end_frame"))
}

fn metadata_number(metadata: Option<&Value>, key: &str) -> Option<i64> {
    metadata
        .and_then(|metadata| metadata.get(key))
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
}

fn metadata_profile_kind(metadata: Option<&Value>) -> Option<&str> {
    metadata
        .and_then(|metadata| metadata.get("profile_kind"))
        .and_then(Value::as_str)
}
