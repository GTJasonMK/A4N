use super::response::aspect_ratio_from_size;
use a4n_domain::ProviderError as AppError;
use a4n_video::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, VideoGenerationRequest, validate_video_provider_duration,
};
use serde_json::{Map, Value, json};

const MAX_REFERENCE_IMAGES: usize = 7;

pub(super) fn request_body_for_route(
    req: &VideoGenerationRequest,
    openai_route: bool,
) -> Result<Value, AppError> {
    validate_video_provider_duration(CLIPROXY_OPENAI_VIDEO_BACKEND, req.duration_secs)
        .map_err(AppError::InvalidArgument)?;
    let mut body = Map::new();
    body.insert("model".into(), json!(req.model));
    body.insert("prompt".into(), json!(req.prompt));
    if openai_route {
        body.insert("seconds".into(), json!(req.duration_secs.to_string()));
    } else {
        body.insert("duration".into(), json!(req.duration_secs));
    }
    if openai_route {
        body.insert(
            "size".into(),
            json!(format!("{}x{}", req.width, req.height)),
        );
    }
    if let Some(aspect_ratio) = aspect_ratio_from_size(req.width, req.height) {
        body.insert("aspect_ratio".into(), json!(aspect_ratio));
    }
    body.insert(
        "resolution".into(),
        json!(resolution_from_size(req.width, req.height)),
    );

    let first_frame_image_url = req
        .media
        .first_frame()
        .map(str::trim)
        .filter(|url| !url.is_empty());
    let reference_image_urls = req
        .media
        .reference_images()
        .iter()
        .map(|url| url.trim())
        .filter(|url| !url.is_empty())
        .collect::<Vec<_>>();

    if req
        .media
        .last_frame()
        .map(str::trim)
        .is_some_and(|url| !url.is_empty())
    {
        return Err(AppError::InvalidArgument(
            "CLIProxy / Grok 视频端点没有尾帧输入字段，不能执行首尾帧模式；请切换到支持尾帧输入的视频 provider"
                .to_string(),
        ));
    }
    if first_frame_image_url.is_some() && !reference_image_urls.is_empty() {
        return Err(AppError::InvalidArgument(
            "CLIProxy / Grok 视频端点不允许同时发送首帧 image 和 reference_images；请使用首帧模式或参考图模式中的一种"
                .to_string(),
        ));
    }

    if reference_image_urls.len() > MAX_REFERENCE_IMAGES {
        return Err(AppError::InvalidArgument(format!(
            "CLIProxy Grok 视频最多支持 {MAX_REFERENCE_IMAGES} 张素材参考图，当前 {} 张",
            reference_image_urls.len()
        )));
    }
    if !reference_image_urls.is_empty() && req.duration_secs > 10 {
        return Err(AppError::InvalidArgument(
            "CLIProxy Grok 视频使用素材参考图时最大时长为 10 秒，请降低时长或移除素材参考图".into(),
        ));
    }
    if let Some(image_url) = first_frame_image_url {
        let (field, value) = if openai_route {
            ("input_reference", json!({ "image_url": image_url }))
        } else {
            ("image", json!({ "url": image_url }))
        };
        body.insert(field.into(), value);
    } else if !reference_image_urls.is_empty() {
        body.insert(
            "reference_images".into(),
            Value::Array(
                reference_image_urls
                    .into_iter()
                    .map(|url| json!({ "url": url }))
                    .collect(),
            ),
        );
    }

    Ok(Value::Object(body))
}

fn resolution_from_size(width: i32, height: i32) -> &'static str {
    if width.min(height) >= 1080 {
        "1080p"
    } else {
        "720p"
    }
}
