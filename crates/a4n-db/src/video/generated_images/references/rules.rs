use crate::video::{VideoDbError, validate_positive_video_i64};
use a4n_core::models::ai::generated_image;
use serde_json::Value;

pub(in crate::video::generated_images) fn validate_positive_image_i64(
    value: i64,
    field: &str,
) -> Result<i64, VideoDbError> {
    validate_positive_video_i64(value, field)
}

pub(in crate::video::generated_images) fn validate_positive_image_i32(
    value: i32,
    field: &str,
) -> Result<i32, VideoDbError> {
    if value <= 0 {
        return Err(VideoDbError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value)
}

pub(in crate::video::generated_images) fn required_image_record_text(
    value: &str,
    field: &str,
) -> Result<String, VideoDbError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(VideoDbError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(value.to_string())
}

pub(in crate::video::generated_images) fn normalize_optional_image_record_text(
    value: Option<&str>,
) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(in crate::video::generated_images) fn validate_generated_image_ai_score(
    value: f32,
) -> Result<f32, VideoDbError> {
    if !value.is_finite() {
        return Err(VideoDbError::InvalidArgument(
            "ai_score 必须是有限数".to_string(),
        ));
    }
    Ok(value)
}

fn generated_image_page_number(metadata: Option<&Value>) -> Option<i32> {
    metadata
        .and_then(|value| value.get("page_number"))
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

pub(in crate::video::generated_images) fn should_delete_manga_page_image_record(
    image: &generated_image::Model,
    panel_id: &str,
    page_number: i32,
    preserve_image_id: Option<i64>,
) -> bool {
    if Some(image.id) == preserve_image_id || image.image_type != "manga_page" {
        return false;
    }
    image.panel_id.as_deref() == Some(panel_id)
        || generated_image_page_number(image.metadata_json.as_ref()) == Some(page_number)
}
