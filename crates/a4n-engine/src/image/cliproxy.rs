use super::support::{
    build_image_http_client, image_provider_http_error, image_result_from_openai_payload,
    internal_error, prepare_reference_image_for_upload, required_non_empty, response_text,
    send_image_request,
};
use super::types::{ImageGenerationRequest, ImageGenerationResult, ImageProvider, ImageReference};
use a4n_core::error::AppError;
use async_trait::async_trait;
use reqwest::multipart::{Form, Part};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrokReferenceMode {
    Reference,
    FirstFrame,
}

/// CLIProxyAPI image provider for xAI Grok Imagine Image.
pub struct CliProxyImageProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
    reference_mode: GrokReferenceMode,
}

impl CliProxyImageProvider {
    pub fn new(base_url: &str, api_key: &str) -> Result<Self, AppError> {
        Self::new_with_options(base_url, api_key, None)
    }

    pub fn new_with_options(
        base_url: &str,
        api_key: &str,
        provider_options_json: Option<&Value>,
    ) -> Result<Self, AppError> {
        Ok(Self {
            base_url: required_non_empty(base_url, "base_url")?
                .trim_end_matches('/')
                .to_string(),
            api_key: required_non_empty(api_key, "api_key")?.to_string(),
            client: build_image_http_client()?,
            reference_mode: grok_reference_mode(provider_options_json)?,
        })
    }

    fn generation_body(req: &ImageGenerationRequest) -> Value {
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

    fn edit_form(&self, req: &ImageGenerationRequest) -> Result<Form, AppError> {
        let reference_images =
            selected_reference_images(&req.reference_images, self.reference_mode);
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
        Self::request_body_preview_with_options(req, None)
    }

    pub fn request_body_preview_with_options(
        req: &ImageGenerationRequest,
        provider_options_json: Option<&Value>,
    ) -> Result<Value, AppError> {
        if req.reference_images.is_empty() {
            return Ok(Self::generation_body(req));
        }
        let reference_mode = grok_reference_mode(provider_options_json)?;
        let reference_images = selected_reference_images(&req.reference_images, reference_mode);
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
            "grok_reference_mode": grok_reference_mode_name(reference_mode),
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

#[async_trait]
impl ImageProvider for CliProxyImageProvider {
    async fn generate(
        &self,
        req: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, AppError> {
        if req.reference_images.is_empty() {
            let url = format!("{}/images/generations", self.base_url);
            let body = Self::generation_body(&req);
            tracing::debug!(
                "cliproxy image generate POST {url} model={} {}x{}",
                req.model,
                req.width,
                req.height
            );
            let resp = send_image_request(
                "CLIProxy 图片生成请求失败",
                self.client
                    .post(&url)
                    .bearer_auth(&self.api_key)
                    .json(&body),
            )
            .await?;
            let status = resp.status();
            let text = response_text(resp, "CLIProxy 图片生成").await?;
            if !status.is_success() {
                return Err(image_provider_http_error(
                    "CLIProxy 图片生成",
                    status,
                    &text,
                ));
            }
            return image_result_from_openai_payload(
                &self.client,
                &text,
                req.model,
                "CLIProxy 图片生成",
            )
            .await;
        }

        let url = format!("{}/images/edits", self.base_url);
        tracing::debug!(
            "cliproxy image edit POST {url} model={} {}x{} refs={}",
            req.model,
            req.width,
            req.height,
            req.reference_images.len()
        );
        let form = self.edit_form(&req)?;
        let resp = send_image_request(
            "CLIProxy 图片编辑请求失败",
            self.client
                .post(&url)
                .bearer_auth(&self.api_key)
                .multipart(form),
        )
        .await?;
        let status = resp.status();
        let text = response_text(resp, "CLIProxy 图片编辑").await?;
        if !status.is_success() {
            return Err(image_provider_http_error(
                "CLIProxy 图片编辑",
                status,
                &text,
            ));
        }
        image_result_from_openai_payload(&self.client, &text, req.model, "CLIProxy 图片编辑").await
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

fn selected_reference_images(
    images: &[ImageReference],
    mode: GrokReferenceMode,
) -> &[ImageReference] {
    match mode {
        GrokReferenceMode::Reference => images,
        GrokReferenceMode::FirstFrame => images.get(..1).unwrap_or(images),
    }
}

fn grok_reference_mode(value: Option<&Value>) -> Result<GrokReferenceMode, AppError> {
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
            "CLIProxy 图片 grok_reference_mode 无效：{other}"
        ))),
    }
}

fn grok_reference_mode_name(mode: GrokReferenceMode) -> &'static str {
    match mode {
        GrokReferenceMode::Reference => "reference",
        GrokReferenceMode::FirstFrame => "first_frame",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ImageGenerationRequest {
        ImageGenerationRequest {
            prompt: "draw".into(),
            width: 1280,
            height: 720,
            model: "grok-imagine-image-quality".into(),
            reference_images: Vec::new(),
        }
    }

    #[test]
    fn generation_body_requests_url_response() {
        let body = CliProxyImageProvider::generation_body(&request());
        assert_eq!(body["response_format"], "url");
        assert_eq!(body["aspect_ratio"], "16:9");
        assert_eq!(body["resolution"], "1k");
    }

    #[test]
    fn generation_body_uses_2k_for_large_images() {
        let mut req = request();
        req.width = 2048;
        req.height = 2048;
        let body = CliProxyImageProvider::generation_body(&req);
        assert_eq!(body["resolution"], "2k");
        assert_eq!(body["aspect_ratio"], "1:1");
    }

    #[test]
    fn first_frame_mode_preview_uses_only_first_reference_image() {
        let mut req = request();
        req.reference_images = vec![
            ImageReference {
                label: "参考图 1".into(),
                data_uri: "data:image/png;base64,aaa".into(),
            },
            ImageReference {
                label: "参考图 2".into(),
                data_uri: "data:image/png;base64,bbb".into(),
            },
        ];

        let body = CliProxyImageProvider::request_body_preview_with_options(
            &req,
            Some(&json!({ "grok_reference_mode": "first_frame" })),
        )
        .unwrap();

        assert_eq!(body["grok_reference_mode"], "first_frame");
        let files = body["files"].as_array().expect("files should be an array");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0]["label"], "参考图 1");
    }
}
