use super::super::{UpdateGeneratedImageRecordInput, VideoDbError};
use a4n_core::models::ai::generated_image;
use a4n_domain::image::is_system_managed_image_type;

pub(super) fn validate_generated_image_update_clear_flags(
    input: &UpdateGeneratedImageRecordInput,
) -> Result<(), VideoDbError> {
    if input.clear_chapter_id && input.chapter_id.is_some() {
        return Err(VideoDbError::InvalidArgument(
            "clear_chapter_id 不能与 chapter_id 同时使用".to_string(),
        ));
    }
    if input.clear_panel_id && input.panel_id.is_some() {
        return Err(VideoDbError::InvalidArgument(
            "clear_panel_id 不能与 panel_id 同时使用".to_string(),
        ));
    }
    if input.clear_ai_score && input.ai_score.is_some() {
        return Err(VideoDbError::InvalidArgument(
            "clear_ai_score 不能与 ai_score 同时使用".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_generated_image_update_image_type(
    current: &str,
    next: &str,
) -> Result<(), VideoDbError> {
    let current = current.trim();
    let next = next.trim();
    if current == next {
        return Ok(());
    }
    if is_system_managed_image_type(current) || is_system_managed_image_type(next) {
        return Err(VideoDbError::InvalidArgument(format!(
            "系统管理图片类型不能通过通用 update_image 修改：{current} -> {next}"
        )));
    }
    Ok(())
}

pub(super) fn validate_generated_image_update_chapter_binding(
    image: &generated_image::Model,
    next: Option<i64>,
) -> Result<(), VideoDbError> {
    if image.chapter_id == next {
        return Ok(());
    }
    if is_system_managed_image_type(&image.image_type) {
        return Err(VideoDbError::InvalidArgument(format!(
            "系统管理图片的 chapter_id 不能通过通用 update_image 修改：image_id={} {:?} -> {:?}",
            image.id, image.chapter_id, next
        )));
    }
    Ok(())
}

pub(super) fn validate_generated_image_update_panel_binding(
    image: &generated_image::Model,
    next: Option<&str>,
) -> Result<(), VideoDbError> {
    if image.panel_id.as_deref() == next {
        return Ok(());
    }
    if is_system_managed_image_type(&image.image_type) {
        return Err(VideoDbError::InvalidArgument(format!(
            "系统管理图片的 panel_id 不能通过通用 update_image 修改：image_id={} {:?} -> {:?}",
            image.id,
            image.panel_id.as_deref(),
            next
        )));
    }
    Ok(())
}
