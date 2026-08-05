use super::super::support::{internal_error, prepare_reference_image_for_upload};
use super::CliProxyImageProvider;
use a4n_core::error::AppError;
use a4n_image::{ImageGenerationRequest, ImageReference};
use reqwest::multipart::{Form, Part};
use serde_json::{Value, json};

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

    pub(super) fn edit_form(&self, req: &ImageGenerationRequest) -> Result<Form, AppError> {
        let reference_images = &req.reference_images;
        if reference_images.is_empty() {
            return Err(internal_error("CLIProxy 图片编辑请求缺少参考图"));
        }
        let prompt = prompt_with_reference_labels(req, reference_images);
        let mut form = Form::new()
            .text("model", req.model.clone())
            .text("prompt", prompt)
            .text("size", format!("{}x{}", req.width, req.height))
            .text("response_format", "url")
            .text("resolution", image_resolution(req.width, req.height));
        if let Some(aspect_ratio) = image_aspect_ratio(req.width, req.height) {
            form = form.text("aspect_ratio", aspect_ratio);
        }
        let field_name = if reference_images.len() == 1 {
            "image"
        } else {
            "image[]"
        };
        for (index, image) in reference_images.iter().enumerate() {
            let decoded = prepare_reference_image_for_upload(&image.data_uri)?;
            let filename = format!("reference_{}.{}", index + 1, decoded.format);
            let part = Part::bytes(decoded.bytes)
                .file_name(filename)
                .mime_str(&decoded.mime_type)
                .map_err(|e| internal_error(format!("参考图 multipart 构造失败：{e}")))?;
            form = form.part(field_name, part);
        }
        Ok(form)
    }

    pub fn request_body_preview(req: &ImageGenerationRequest) -> Result<Value, AppError> {
        if req.reference_images.is_empty() {
            return Ok(Self::generation_body(req));
        }
        let reference_images = &req.reference_images;
        let field_name = if reference_images.len() == 1 {
            "image"
        } else {
            "image[]"
        };
        let files = reference_images
            .iter()
            .enumerate()
            .map(|(index, image)| {
                json!({
                    "field": field_name,
                    "filename": format!("reference_{}", index + 1),
                    "label": image.label,
                    "data_uri": image.data_uri,
                })
            })
            .collect::<Vec<_>>();
        let prompt = prompt_with_reference_labels(req, reference_images);
        Ok(json!({
            "endpoint": "/images/edits",
            "content_type": "multipart/form-data",
            "model": req.model,
            "size": format!("{}x{}", req.width, req.height),
            "fields": {
                "model": req.model,
                "prompt": prompt,
                "size": format!("{}x{}", req.width, req.height),
                "response_format": "url",
                "resolution": image_resolution(req.width, req.height),
                "aspect_ratio": image_aspect_ratio(req.width, req.height),
            },
            "files": files,
        }))
    }
}
fn prompt_with_reference_labels(
    req: &ImageGenerationRequest,
    reference_images: &[ImageReference],
) -> String {
    if reference_images.is_empty() {
        return req.prompt.clone();
    }
    let refs = reference_images
        .iter()
        .enumerate()
        .map(|(index, image)| format!("{}. {}", index + 1, image.label))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{}\n\n参考图绑定：\n{}", req.prompt, refs)
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
