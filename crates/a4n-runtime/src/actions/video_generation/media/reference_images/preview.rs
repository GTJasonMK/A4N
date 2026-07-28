use super::records::{
    load_reference_images_by_id, reference_image_project_ids_in_reference_order,
    verify_reference_image_project_editors,
};
use crate::actions::video_generation::media::types::ReferenceImagePreviewPlaceholders;
use crate::state::{CollaborationRuntimeFacade, VideoRuntimeFacade};
use a4n_core::models::ai::generated_image;
use a4n_orchestrator::video::{
    encode_query_component, ensure_reference_image_ids_valid, image_mime_from_path,
    is_project_reusable_video_material_image,
};
use std::path::Path;

pub(crate) async fn reference_image_preview_placeholders_by_ids(
    video: &VideoRuntimeFacade<'_>,
    collaboration: &CollaborationRuntimeFacade<'_>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
) -> Result<ReferenceImagePreviewPlaceholders, String> {
    ensure_reference_image_ids_valid(&ids)?;
    if ids.is_empty() {
        return Ok(ReferenceImagePreviewPlaceholders::default());
    }
    reference_image_preview_placeholders_for_ids(
        video,
        collaboration,
        user_id,
        project_id,
        chapter_id,
        ids,
    )
    .await
}

pub(crate) async fn reference_image_preview_placeholder_by_id(
    video: &VideoRuntimeFacade<'_>,
    collaboration: &CollaborationRuntimeFacade<'_>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    id: Option<i64>,
) -> Result<(Option<String>, Vec<String>), String> {
    let Some(id) = id else {
        return Ok((None, Vec::new()));
    };
    let mut placeholders = reference_image_preview_placeholders_for_ids(
        video,
        collaboration,
        user_id,
        project_id,
        chapter_id,
        vec![id],
    )
    .await?;
    Ok((placeholders.urls.pop(), placeholders.notes))
}

async fn reference_image_preview_placeholders_for_ids(
    video: &VideoRuntimeFacade<'_>,
    collaboration: &CollaborationRuntimeFacade<'_>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    ids: Vec<i64>,
) -> Result<ReferenceImagePreviewPlaceholders, String> {
    let allowed_project_ids = video
        .video_asset_project_ids(project_id)
        .await
        .map_err(|error| error.to_string())?;
    let mut result = ReferenceImagePreviewPlaceholders {
        urls: Vec::with_capacity(ids.len()),
        notes: Vec::new(),
    };

    let images_by_id = load_reference_images_by_id(video, &ids).await?;
    let project_ids = reference_image_project_ids_in_reference_order(&ids, &images_by_id);
    verify_reference_image_project_editors(collaboration, user_id, &project_ids).await?;

    for id in ids {
        if let Some(image) = images_by_id.get(&id) {
            result.notes.extend(reference_image_preview_notes(
                project_id,
                chapter_id,
                &allowed_project_ids,
                image,
            ));
            result.urls.push(reference_image_preview_placeholder(image));
        } else {
            result.notes.push(format!(
                "参考图片 #{id} 不存在；预览使用占位媒体 URL 展示请求结构，真实生成会拒绝该输入"
            ));
            result.urls.push(format!(
                "https://example.invalid/a4n-media/preview/generated-image-{id}?missing_row=1"
            ));
        }
    }

    Ok(result)
}

pub(in crate::actions::video_generation::media) fn reference_image_preview_notes(
    project_id: i64,
    chapter_id: i64,
    allowed_project_ids: &[i64],
    image: &generated_image::Model,
) -> Vec<String> {
    let mut notes = Vec::new();
    if !allowed_project_ids.contains(&image.project_id) {
        notes.push(format!(
            "参考图片 #{} 属于项目 #{}，不属于当前视频项目 #{} 或其根文本项目；真实生成会拒绝该输入",
            image.id, image.project_id, project_id
        ));
    }
    let project_reusable =
        is_project_reusable_video_material_image(&image.image_type, image.metadata_json.as_ref());
    if let Some(image_chapter_id) = image
        .chapter_id
        .filter(|id| *id != chapter_id && !project_reusable)
    {
        notes.push(format!(
            "参考图片 #{} 属于章节 #{}，不属于当前章节 #{}；真实生成会拒绝该输入",
            image.id, image_chapter_id, chapter_id
        ));
    }
    if image.file_path.trim().is_empty() {
        notes.push(format!(
            "参考图片 #{} 文件路径为空；预览使用占位媒体 URL，真实生成会拒绝该输入",
            image.id
        ));
    } else if let Err(err) = image_mime_from_path(Path::new(&image.file_path)) {
        notes.push(format!(
            "参考图片 #{} 文件类型无法作为视频图片输入：{err}；预览使用占位媒体 URL，真实生成会拒绝该输入",
            image.id
        ));
    }
    notes
}

pub(in crate::actions::video_generation::media) fn reference_image_preview_placeholder(
    image: &generated_image::Model,
) -> String {
    let size_hint = image
        .file_size
        .filter(|size| *size > 0)
        .map(|size| format!("{size} bytes"))
        .unwrap_or_else(|| "size unknown".to_string());
    format!(
        "https://example.invalid/a4n-media/preview/generated-image-{}?size_hint={}&validation=deferred",
        image.id,
        encode_query_component(&size_hint)
    )
}
