use super::super::support::{
    internal_error, prepare_reference_image_for_upload, prompt_with_reference_labels,
};
use super::CliProxyImageProvider;
use a4n_domain::ProviderError as AppError;
use a4n_image_contract::ImageGenerationRequest;
use base64::Engine as _;
use serde_json::{Value, json};

const MAX_XAI_IMAGE_EDIT_REFERENCES: usize = 3;

impl CliProxyImageProvider {
    pub(super) fn generation_body(req: &ImageGenerationRequest) -> Value {
        let size = format!("{}x{}", req.width, req.height);
        let mut body = json!({
            "model": req.model,
            "prompt": req.prompt,
            "n": 1,
            "size": size,
            "response_format": "url",
            "resolution": image_resolution(req.width, req.height),
        });
        if let Some(aspect_ratio) = image_aspect_ratio(req.width, req.height) {
            body["aspect_ratio"] = json!(aspect_ratio);
        }
        body
    }

    pub(super) fn edit_body(req: &ImageGenerationRequest) -> Result<Value, AppError> {
        let reference_images = &req.reference_images;
        if reference_images.is_empty() {
            return Err(internal_error("CLIProxy 图片编辑请求缺少参考图"));
        }
        if reference_images.len() > MAX_XAI_IMAGE_EDIT_REFERENCES {
            return Err(AppError::InvalidArgument(format!(
                "CLIProxy/Grok xAI 图片编辑最多支持 {MAX_XAI_IMAGE_EDIT_REFERENCES} 张参考图"
            )));
        }

        let prompt = prompt_with_reference_labels(&req.prompt, reference_images);
        let mut body = json!({
            "model": req.model,
            "prompt": prompt,
            "size": format!("{}x{}", req.width, req.height),
            "response_format": "url",
            "resolution": image_resolution(req.width, req.height),
        });
        let mut images = Vec::with_capacity(reference_images.len());
        for image in reference_images {
            let decoded = prepare_reference_image_for_upload(&image.data_uri)?;
            let data_url = format!(
                "data:{};base64,{}",
                decoded.mime_type,
                base64::engine::general_purpose::STANDARD.encode(decoded.bytes)
            );
            images.push(json!({"url": data_url}));
        }

        if images.len() == 1 {
            body["image"] = images.remove(0);
        } else {
            body["images"] = json!(images);
            if let Some(aspect_ratio) = image_aspect_ratio(req.width, req.height) {
                body["aspect_ratio"] = json!(aspect_ratio);
            }
        }
        Ok(body)
    }

    pub(crate) fn request_body_preview(req: &ImageGenerationRequest) -> Result<Value, AppError> {
        if req.reference_images.is_empty() {
            return Ok(Self::generation_body(req));
        }
        let body = Self::edit_body(req)?;
        Ok(json!({
            "endpoint": "/images/edits",
            "content_type": "application/json",
            "model": req.model,
            "size": format!("{}x{}", req.width, req.height),
            "fields": body,
        }))
    }
}

fn image_resolution(width: i32, height: i32) -> &'static str {
    if width.max(height) >= 1536 {
        "2k"
    } else {
        "1k"
    }
}

fn image_aspect_ratio(width: i32, height: i32) -> Option<&'static str> {
    if width <= 0 || height <= 0 {
        return None;
    }
    if width == height {
        return Some("1:1");
    }
    if width * 9 == height * 16 {
        return Some("16:9");
    }
    if width * 16 == height * 9 {
        return Some("9:16");
    }
    if width * 2 == height * 3 {
        return Some("3:2");
    }
    if width * 3 == height * 2 {
        return Some("2:3");
    }
    None
}
