use super::provider::GrokReferenceMode;
use super::response::{aspect_ratio_from_size, ensure_http_media_url};
use a4n_core::error::AppError;
use a4n_video::VideoGenerationRequest;
use serde_json::{Map, Value, json};

const MAX_REFERENCE_IMAGES: usize = 7;

pub(super) fn request_body_with_mode(
    req: &VideoGenerationRequest,
    reference_mode: GrokReferenceMode,
    openai_route: bool,
) -> Result<Value, AppError> {
    let mut body = Map::new();
    body.insert("model".into(), json!(req.model));
    body.insert("prompt".into(), json!(req.prompt));
    if openai_route {
        body.insert(
            "seconds".into(),
            json!(req.duration_secs.max(1).to_string()),
        );
    } else {
        body.insert("duration".into(), json!(req.duration_secs.max(1)));
    }
    body.insert(
        "size".into(),
        json!(format!("{}x{}", req.width, req.height)),
    );
    if let Some(aspect_ratio) = aspect_ratio_from_size(req.width, req.height) {
        body.insert("aspect_ratio".into(), json!(aspect_ratio));
    }
    body.insert("resolution".into(), json!("720p"));

    let first_frame_image_url = req
        .first_frame_image_url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty());
    let reference_image_urls = req
        .reference_image_urls
        .iter()
        .map(|url| url.trim())
        .filter(|url| !url.is_empty())
        .collect::<Vec<_>>();

    if reference_mode == GrokReferenceMode::FirstFrame {
        if let Some(last_frame_url) = req.last_frame_image_url.as_deref().map(str::trim)
            && !last_frame_url.is_empty()
        {
            return Err(AppError::InvalidArgument(
                "CLIProxy Grok 视频首帧模式只支持首帧 input_reference，不支持尾帧输入；请改用首帧串联模式或切换视频 provider"
                    .into(),
            ));
        }
        if let Some(image_url) =
            first_frame_image_url.or_else(|| reference_image_urls.first().copied())
        {
            ensure_http_media_url(image_url, "首帧图片 URL")?;
            body.insert(
                "input_reference".into(),
                json!({
                    "image_url": image_url,
                }),
            );
        }
        return Ok(Value::Object(body));
    }

    if first_frame_image_url.is_some()
        || req
            .last_frame_image_url
            .as_deref()
            .map(str::trim)
            .is_some_and(|url| !url.is_empty())
    {
        return Err(AppError::InvalidArgument(
            "CLIProxy Grok 视频参考模式只读取 reference_image_urls；请将所有输入图片按实际参考顺序放入 reference_image_urls，不要设置 first_frame_image_url 或 last_frame_image_url"
                .into(),
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
    let mut refs = Vec::with_capacity(reference_image_urls.len());
    for (index, url) in reference_image_urls.iter().enumerate() {
        ensure_http_media_url(url, &format!("素材参考图 #{}", index + 1))?;
        refs.push(json!({ "url": url }));
    }
    if !refs.is_empty() {
        body.insert("reference_images".into(), Value::Array(refs));
    }

    Ok(Value::Object(body))
}

pub(super) fn grok_reference_mode(value: Option<&Value>) -> Result<GrokReferenceMode, AppError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(GrokReferenceMode::Reference);
    };
    let Some(mode) = value
        .get("grok_reference_mode")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|mode| !mode.is_empty())
    else {
        return Ok(GrokReferenceMode::Reference);
    };
    match mode {
        "reference" => Ok(GrokReferenceMode::Reference),
        "first_frame" => Ok(GrokReferenceMode::FirstFrame),
        other => Err(AppError::InvalidArgument(format!(
            "CLIProxy Grok 视频 grok_reference_mode 无效：{other}"
        ))),
    }
}
