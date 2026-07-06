use super::support::{
    build_image_http_client, detect_format_from_bytes, download_image_url,
    image_provider_http_error, internal_error, normalize_image_format, required_non_empty,
    response_text, send_image_request,
};
use super::types::{ImageGenerationRequest, ImageGenerationResult, ImageProvider};
use a4n_core::error::AppError;
use async_trait::async_trait;
use base64::Engine as _;
use serde_json::{Value, json};

/// Chat-Completions 图像 provider：把 `/v1/chat/completions` 当作图像生成网关。
///
/// **响应解析策略**：按明确支持的 Chat-Completions 图片响应结构查找图片引用：
///
/// 1. `choices[0].message.images[*]` 中的 `image_url.url` / `image_url` / `url` / `b64_json`
/// 2. `choices[0].message.content[*]` 数组中 `type` 为 `image_url` / `image` /
///    `output_image` / `input_image` 的项
/// 3. `choices[0].message.content` 字符串里的 `data:image/*;base64,...` 或 markdown `![alt](url)`
/// 4. 若标准位置都没有命中，则递归扫描整个响应中的图片语义字段和文本图片引用。
///
/// 找到第一个有效引用即返回；URL 形态由 `generate` 后续下载，data URI 当场解码。
/// 失败时报告所扫描位置 + 响应预览，便于用户对照中转站文档定位问题。
pub struct ChatCompletionsImageProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl ChatCompletionsImageProvider {
    pub fn new(base_url: &str, api_key: &str) -> Result<Self, AppError> {
        Ok(Self {
            base_url: required_non_empty(base_url, "base_url")?
                .trim_end_matches('/')
                .to_string(),
            api_key: required_non_empty(api_key, "api_key")?.to_string(),
            client: build_image_http_client()?,
        })
    }

    #[cfg(test)]
    fn parse_response_for_test(body: &str) -> Result<ChatImagePayload, AppError> {
        extract_chat_image_payload(body)
    }

    pub fn request_body_preview(req: &ImageGenerationRequest) -> Value {
        chat_request_body(req)
    }
}

#[async_trait]
impl ImageProvider for ChatCompletionsImageProvider {
    async fn generate(
        &self,
        req: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, AppError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = chat_request_body(&req);

        tracing::debug!(
            "chat-completions image POST {url} model={} {}x{}",
            req.model,
            req.width,
            req.height
        );

        let resp = send_image_request(
            "chat-completions 图像生成请求失败",
            self.client
                .post(&url)
                .bearer_auth(&self.api_key)
                .header(reqwest::header::ACCEPT, "application/json")
                .json(&body),
        )
        .await?;

        let status = resp.status();
        let text = response_text(resp, "chat-completions 图像生成").await?;
        if !status.is_success() {
            return Err(image_provider_http_error(
                "chat-completions 图像生成",
                status,
                &text,
            ));
        }

        let payload = extract_chat_image_payload(&text)?;
        let (bytes, format) = match payload {
            ChatImagePayload::DataUri { format, bytes } => (bytes, format),
            ChatImagePayload::Base64 { format, bytes } => {
                let format = match format {
                    Some(f) => f,
                    None => detect_format_from_bytes(&bytes)?,
                };
                (bytes, format)
            }
            ChatImagePayload::Url(url) => {
                tracing::debug!("chat-completions 返回 URL，下载：{url}");
                download_image_url(&self.client, &url, None).await?
            }
        };

        Ok(ImageGenerationResult {
            bytes,
            format,
            model: req.model,
        })
    }
}

fn chat_request_body(req: &ImageGenerationRequest) -> Value {
    json!({
        "model": req.model,
        "stream": false,
        "modalities": ["image", "text"],
        "n": 1,
        "size": format!("{}x{}", req.width, req.height),
        "messages": [
            { "role": "user", "content": image_user_content(req) }
        ],
    })
}

fn image_user_content(req: &ImageGenerationRequest) -> Value {
    let user_text = image_user_text(req);
    if req.reference_images.is_empty() {
        return Value::String(user_text);
    }

    let mut parts = vec![json!({ "type": "text", "text": user_text })];
    for image in &req.reference_images {
        parts.push(json!({
            "type": "image_url",
            "image_url": {
                "url": image.data_uri,
            },
        }));
    }
    Value::Array(parts)
}

fn image_user_text(req: &ImageGenerationRequest) -> String {
    // 把尺寸提示拼到 user message 里，部分网关靠 prompt 文本决定尺寸；
    // 其他参数 `size` / `modalities` / `n` 走 OpenAI 扩展。
    if req.reference_images.is_empty() {
        return format!(
            "{}\n\n[输出图片，尺寸 {}x{}]",
            req.prompt, req.width, req.height
        );
    }

    let refs = req
        .reference_images
        .iter()
        .enumerate()
        .map(|(index, image)| format!("{}. {}", index + 1, image.label))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{}\n\n[输出图片，尺寸 {}x{}]\n\n参考图绑定：\n{}",
        req.prompt, req.width, req.height, refs
    )
}

/// chat-completions 响应中提取出的图像负载。
#[derive(Debug, Clone, PartialEq, Eq)]
enum ChatImagePayload {
    /// `data:image/<ext>;base64,XXX`，扩展名已知。
    DataUri { format: String, bytes: Vec<u8> },
    /// 纯 base64 字符串，未带 data URI 头，格式需由字节魔数判断。
    Base64 {
        format: Option<String>,
        bytes: Vec<u8>,
    },
    /// HTTP(S) URL，需要后续下载。
    Url(String),
}

fn extract_chat_image_payload(body: &str) -> Result<ChatImagePayload, AppError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| internal_error(format!("chat-completions 响应解析失败：{e}")))?;
    if let Some(message) = first_message(&value) {
        if let Some(payload) = scan_message_images(message)? {
            return Ok(payload);
        }
        if let Some(payload) = scan_message_content(message)? {
            return Ok(payload);
        }
    }
    if let Some(payload) = scan_any_image_payload(&value)? {
        return Ok(payload);
    }

    Err(internal_error(format!(
        "chat-completions 响应未发现图片：扫描了 message.images[]、message.content[].image_url、\
         message.content 字符串，以及整个响应中的图片语义字段。响应预览：{}",
        preview(body)
    )))
}

fn first_message(value: &serde_json::Value) -> Option<&serde_json::Value> {
    value
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("message"))
}

/// 第一层：`message.images[*]` 中的常见字段（OpenRouter Gemini-image 等）。
fn scan_message_images(message: &serde_json::Value) -> Result<Option<ChatImagePayload>, AppError> {
    let Some(arr) = message.get("images").and_then(|v| v.as_array()) else {
        return Ok(None);
    };
    for item in arr {
        if let Some(payload) = read_image_part_url(item)? {
            return Ok(Some(payload));
        }
    }
    Ok(None)
}

/// 第二层：`message.content` —— 数组（找 type=image_url 项）或字符串（扫 data URI / markdown）。
fn scan_message_content(message: &serde_json::Value) -> Result<Option<ChatImagePayload>, AppError> {
    let Some(content) = message.get("content") else {
        return Ok(None);
    };
    if let Some(arr) = content.as_array() {
        for part in arr {
            let kind = part.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(kind, "image_url" | "image" | "output_image" | "input_image")
                && let Some(payload) = read_image_part_url(part)?
            {
                return Ok(Some(payload));
            }
        }
        return Ok(None);
    }
    if let Some(text) = content.as_str() {
        return extract_image_from_text(text);
    }
    Ok(None)
}

fn scan_any_image_payload(value: &serde_json::Value) -> Result<Option<ChatImagePayload>, AppError> {
    scan_any_image_payload_at(value, None)
}

fn scan_any_image_payload_at(
    value: &serde_json::Value,
    key_hint: Option<&str>,
) -> Result<Option<ChatImagePayload>, AppError> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(payload) = read_image_part_url(value)? {
                return Ok(Some(payload));
            }
            for (key, child) in map {
                if let Some(payload) = scan_any_image_payload_at(child, Some(key))? {
                    return Ok(Some(payload));
                }
            }
            Ok(None)
        }
        serde_json::Value::Array(items) => {
            for item in items {
                if let Some(payload) = scan_any_image_payload_at(item, key_hint)? {
                    return Ok(Some(payload));
                }
            }
            Ok(None)
        }
        serde_json::Value::String(text) => {
            if image_payload_key_hint(key_hint) {
                return classify_image_string(text).map(Some);
            }
            extract_image_from_text(text)
        }
        _ => Ok(None),
    }
}

fn image_payload_key_hint(key: Option<&str>) -> bool {
    let Some(key) = key else {
        return false;
    };
    let key = key.to_ascii_lowercase();
    key == "url"
        || key == "image_url"
        || key == "b64_json"
        || key == "base64"
        || key == "data_uri"
        || key.ends_with("_url")
        || key.contains("image")
}

fn preview(body: &str) -> String {
    let trimmed = body.trim();
    let cap: String = trimmed.chars().take(300).collect();
    if trimmed.chars().count() > 300 {
        format!("{cap}…（已截断）")
    } else {
        cap
    }
}

fn read_image_part_url(item: &serde_json::Value) -> Result<Option<ChatImagePayload>, AppError> {
    let raw_url = item
        .get("image_url")
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                Some(s)
            } else {
                v.get("url").and_then(|u| u.as_str())
            }
        })
        .or_else(|| item.get("url").and_then(|v| v.as_str()))
        .or_else(|| item.get("b64_json").and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let raw_url = match raw_url {
        Some(v) => v,
        None => return Ok(None),
    };
    classify_image_string(raw_url).map(Some)
}

fn extract_image_from_text(text: &str) -> Result<Option<ChatImagePayload>, AppError> {
    if let Some(start) = text.find("data:image/") {
        let rest = &text[start..];
        let end = rest
            .find(|c: char| c.is_whitespace() || c == ')' || c == '"' || c == '\'')
            .unwrap_or(rest.len());
        let candidate = &rest[..end];
        return classify_image_string(candidate).map(Some);
    }
    if let Some(url) = find_markdown_image(text) {
        return classify_image_string(url).map(Some);
    }
    Ok(None)
}

/// 解析形如 `![alt](https://example.com/x.png)` 的 markdown 图片，返回 URL 部分。
fn find_markdown_image(text: &str) -> Option<&str> {
    let bang = text.find("![")?;
    let after_alt_open = bang + 2;
    let alt_close_rel = text[after_alt_open..].find(']')?;
    let alt_close = after_alt_open + alt_close_rel;
    // 紧跟 `](`
    if text.as_bytes().get(alt_close + 1).copied() != Some(b'(') {
        return None;
    }
    let url_start = alt_close + 2;
    let url_end_rel = text[url_start..].find(')')?;
    let url = text[url_start..url_start + url_end_rel].trim();
    let url = url.split_whitespace().next().unwrap_or(url);
    if url.is_empty() { None } else { Some(url) }
}

fn classify_image_string(raw: &str) -> Result<ChatImagePayload, AppError> {
    let raw = raw.trim();
    if let Some(rest) = raw.strip_prefix("data:") {
        let (header, payload) = rest
            .split_once(',')
            .ok_or_else(|| internal_error("chat-completions data URI 缺少 ','"))?;
        let payload = payload.trim();
        let mime = header.split(';').next().unwrap_or("").trim();
        let format = mime
            .strip_prefix("image/")
            .map(normalize_image_format)
            .transpose()?
            .ok_or_else(|| {
                internal_error(format!("chat-completions 不支持的 data URI mime：{mime}"))
            })?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .map_err(|e| internal_error(format!("base64 解码失败：{e}")))?;
        return Ok(ChatImagePayload::DataUri { format, bytes });
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return Ok(ChatImagePayload::Url(raw.to_string()));
    }
    // 纯 base64 串：先解码，再用魔数嗅探。
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|e| internal_error(format!("base64 解码失败：{e}")))?;
    Ok(ChatImagePayload::Base64 {
        format: None,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 用最小 1x1 PNG 像素的 base64 作为测试样本，避免重复书写魔数。
    const PNG_1X1_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

    fn parse(body: serde_json::Value) -> Result<ChatImagePayload, AppError> {
        ChatCompletionsImageProvider::parse_response_for_test(&body.to_string())
    }

    #[test]
    fn chat_auto_parses_message_images_data_uri_url() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "images": [
                        { "image_url": { "url": format!("data:image/png;base64,{PNG_1X1_B64}") } }
                    ]
                }
            }]
        });
        let payload = parse(body).unwrap();
        match payload {
            ChatImagePayload::DataUri { format, bytes } => {
                assert_eq!(format, "png");
                assert!(bytes.starts_with(b"\x89PNG"));
            }
            other => panic!("expected DataUri, got {other:?}"),
        }
    }

    #[test]
    fn chat_auto_parses_message_images_b64_json() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "images": [ { "b64_json": PNG_1X1_B64 } ]
                }
            }]
        });
        let payload = parse(body).unwrap();
        assert!(
            matches!(payload, ChatImagePayload::Base64 { ref bytes, .. } if bytes.starts_with(b"\x89PNG"))
        );
    }

    #[test]
    fn chat_auto_parses_content_array_image_url_string() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "content": [
                        { "type": "text", "text": "preface" },
                        { "type": "image_url", "image_url": format!("data:image/png;base64,{PNG_1X1_B64}") }
                    ]
                }
            }]
        });
        let payload = parse(body).unwrap();
        assert!(matches!(payload, ChatImagePayload::DataUri { ref format, .. } if format == "png"));
    }

    #[test]
    fn chat_auto_parses_content_array_image_url_object() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "content": [
                        { "type": "image_url", "image_url": { "url": "https://example.com/x.png" } }
                    ]
                }
            }]
        });
        let payload = parse(body).unwrap();
        assert_eq!(
            payload,
            ChatImagePayload::Url("https://example.com/x.png".into())
        );
    }

    #[test]
    fn chat_auto_parses_content_string_data_uri() {
        let body = serde_json::json!({
            "choices": [{
                "message": { "content": format!("Here:\n\ndata:image/png;base64,{PNG_1X1_B64}\n") }
            }]
        });
        let payload = parse(body).unwrap();
        assert!(matches!(payload, ChatImagePayload::DataUri { ref format, .. } if format == "png"));
    }

    #[test]
    fn chat_auto_parses_content_string_markdown_url() {
        let body = serde_json::json!({
            "choices": [{
                "message": { "content": "前置 ![alt](https://example.com/x.png) 后置" }
            }]
        });
        let payload = parse(body).unwrap();
        assert_eq!(
            payload,
            ChatImagePayload::Url("https://example.com/x.png".into())
        );
    }

    #[test]
    fn chat_auto_parses_nested_b64_json_outside_standard_fields() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "extras": {
                        "generation": { "b64_json": PNG_1X1_B64 }
                    }
                }
            }]
        });
        let payload = parse(body).unwrap();
        assert!(
            matches!(payload, ChatImagePayload::Base64 { ref bytes, .. } if bytes.starts_with(b"\x89PNG"))
        );
    }

    #[test]
    fn chat_auto_parses_image_field_without_choices_message() {
        let body = serde_json::json!({
            "result": {
                "output_image": {
                    "url": "https://example.com/nonstandard.png"
                }
            }
        });
        let payload = parse(body).unwrap();
        assert_eq!(
            payload,
            ChatImagePayload::Url("https://example.com/nonstandard.png".into())
        );
    }

    #[test]
    fn chat_auto_prefers_message_images_over_content_string() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "images": [
                        { "url": "https://primary.example/a.png" }
                    ],
                    "content": "https://secondary.example/should-not-pick.png"
                }
            }]
        });
        let payload = parse(body).unwrap();
        assert_eq!(
            payload,
            ChatImagePayload::Url("https://primary.example/a.png".into())
        );
    }

    #[test]
    fn chat_auto_reports_diagnostic_when_no_image_found() {
        let body = serde_json::json!({
            "choices": [{ "message": { "content": "just text, nothing useful" } }]
        });
        let err = parse(body).unwrap_err();
        let msg = err.to_string();
        // 错误信息必须列出扫描过的位置 + 包含响应预览
        assert!(msg.contains("message.images[]"));
        assert!(msg.contains("message.content[].image_url"));
        assert!(msg.contains("图片语义字段"));
        assert!(msg.contains("响应预览"));
        assert!(msg.contains("just text"));
    }

    #[test]
    fn chat_auto_rejects_missing_choices_without_any_image_payload() {
        let body = serde_json::json!({});
        let err = parse(body).unwrap_err();
        assert!(err.to_string().contains("响应未发现图片"));
        assert!(err.to_string().contains("响应预览"));
    }

    #[test]
    fn chat_auto_rejects_unsupported_data_uri_mime() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "images": [
                        { "image_url": { "url": format!("data:image/gif;base64,{PNG_1X1_B64}") } }
                    ]
                }
            }]
        });
        assert!(parse(body).is_err());
    }
}
