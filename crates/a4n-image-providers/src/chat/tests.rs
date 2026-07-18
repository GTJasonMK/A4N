use super::parser::ChatImagePayload;
use super::provider::ChatCompletionsImageProvider;
use a4n_core::error::AppError;

/// 用最小 1x1 PNG 像素的 base64 作为测试样本，避免重复书写魔数。
const PNG_1X1_B64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

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
