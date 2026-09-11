use crate::AppState;
use crate::actions::image_records;

pub(super) async fn delete_downstream_images_if_requested(
    state: &AppState,
    user_id: i64,
    image_ids: impl IntoIterator<Item = i64>,
) -> Result<(), String> {
    for image_id in image_ids {
        image_records::delete_image(state, user_id, image_records::DeleteImageInput { image_id })
            .await?;
    }
    Ok(())
}

pub(super) fn downstream_cleanup_warnings(result: Result<(), String>) -> Vec<String> {
    result
        .err()
        .map(|error| format!("下游图片清理失败：{error}"))
        .into_iter()
        .collect()
}
