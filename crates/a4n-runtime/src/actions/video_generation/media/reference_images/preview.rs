use super::records::validated_reference_images_for_ids;
use a4n_api_contract::image::GeneratedImageSummary;
use a4n_domain::image::validate_reference_image_id_list;
use a4n_domain::media::image_mime_from_path;
use a4n_orchestrator::video::VideoRecordApplicationPort;
use std::path::Path;

pub(crate) async fn reference_image_request_preview_urls_by_ids(
    video: &dyn VideoRecordApplicationPort,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
) -> Result<Vec<String>, String> {
    validate_reference_image_id_list(&ids)?;
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    reference_image_request_preview_urls_for_ids(video, user_id, project_id, chapter_id, ids).await
}

pub(crate) async fn reference_image_request_preview_url_by_id(
    video: &dyn VideoRecordApplicationPort,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    id: Option<i64>,
) -> Result<Option<String>, String> {
    let Some(id) = id else {
        return Ok(None);
    };
    let mut urls = reference_image_request_preview_urls_for_ids(
        video,
        user_id,
        project_id,
        chapter_id,
        vec![id],
    )
    .await?;
    Ok(urls.pop())
}

async fn reference_image_request_preview_urls_for_ids(
    video: &dyn VideoRecordApplicationPort,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
) -> Result<Vec<String>, String> {
    let images =
        validated_reference_images_for_ids(video, user_id, project_id, chapter_id, ids).await?;
    images
        .iter()
        .map(reference_image_request_preview_url)
        .collect()
}

fn reference_image_request_preview_url(image: &GeneratedImageSummary) -> Result<String, String> {
    let file_path = image.file_path.trim();
    if file_path.is_empty() {
        return Err(format!("参考图片 #{} 文件路径为空", image.id));
    }
    image_mime_from_path(Path::new(file_path)).map_err(|error| {
        format!(
            "参考图片 #{} 文件类型无法作为视频图片输入：{error}",
            image.id
        )
    })?;
    Ok(format!(
        "https://example.invalid/a4n-media/preview/generated-image-{}",
        image.id
    ))
}
