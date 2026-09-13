use a4n_error::AppError;
use serde_json::Value;
use std::collections::HashSet;

pub(super) fn validate_chat_video_source_span_ids(
    chat_video: &Value,
    segment: &crate::video::prompt_json::contracts::VideoSegmentPlan,
) -> Result<(), AppError> {
    let allowed_source_span_ids = &segment.source_span_ids;
    let source_span_ids = chat_video
        .get("source_span_ids")
        .ok_or_else(|| {
            AppError::InvalidArgument("chat_video.source_span_ids 不能为空".to_string())
        })?
        .as_array()
        .ok_or_else(|| {
            AppError::InvalidArgument("chat_video.source_span_ids 必须是数组".to_string())
        })?;
    if source_span_ids.is_empty() && segment.director_constraints.is_none() {
        return Err(AppError::InvalidArgument(
            "chat_video.source_span_ids 不能为空".to_string(),
        ));
    }
    let allowed = allowed_source_span_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    for (index, value) in source_span_ids.iter().enumerate() {
        let id = value
            .as_str()
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "chat_video.source_span_ids[{}] 必须是非空字符串",
                    index + 1
                ))
            })?;
        if !allowed.contains(id) {
            return Err(AppError::InvalidArgument(format!(
                "chat_video.source_span_ids[{}] 不属于当前片段 source_span_ids：{id}",
                index + 1
            )));
        }
        if !seen.insert(id) {
            return Err(AppError::InvalidArgument(format!(
                "chat_video.source_span_ids[{}] 重复：{id}",
                index + 1
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_chat_video_input_images(
    chat_video: &Value,
    reference_image_ids: &[i64],
) -> Result<(), AppError> {
    let Some(input_images_value) = chat_video.get("input_images") else {
        if reference_image_ids.is_empty() {
            return Ok(());
        }
        return Err(AppError::InvalidArgument(
            "chat_video.input_images 必须按 reference_image_ids 记录每张 Chat 上传图片".to_string(),
        ));
    };
    let input_images = input_images_value.as_array().ok_or_else(|| {
        AppError::InvalidArgument("chat_video.input_images 必须是数组".to_string())
    })?;
    if input_images.len() != reference_image_ids.len() {
        return Err(AppError::InvalidArgument(format!(
            "chat_video.input_images 数量必须等于 reference_image_ids 数量：当前 {}，期望 {}",
            input_images.len(),
            reference_image_ids.len()
        )));
    }
    for (index, (input_image, expected_image_id)) in
        input_images.iter().zip(reference_image_ids).enumerate()
    {
        let order = index + 1;
        let object = input_image.as_object().ok_or_else(|| {
            AppError::InvalidArgument(format!("chat_video.input_images[{order}] 必须是对象"))
        })?;
        let image_id = object
            .get("image_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "chat_video.input_images[{order}].image_id 必须是整数"
                ))
            })?;
        if image_id != *expected_image_id {
            return Err(AppError::InvalidArgument(format!(
                "chat_video.input_images[{order}].image_id 必须等于 reference_image_ids[{order}]：期望 {}，实际 {}",
                expected_image_id, image_id
            )));
        }
        let upload_order = object
            .get("upload_order")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "chat_video.input_images[{order}].upload_order 必须是整数"
                ))
            })?;
        if upload_order != order as i64 {
            return Err(AppError::InvalidArgument(format!(
                "chat_video.input_images[{order}].upload_order 必须等于 {order}，实际 {upload_order}"
            )));
        }
        for field in ["role", "usage"] {
            let text = object
                .get(field)
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            if text.is_empty() {
                return Err(AppError::InvalidArgument(format!(
                    "chat_video.input_images[{order}].{field} 必须是非空字符串"
                )));
            }
        }
    }
    Ok(())
}
