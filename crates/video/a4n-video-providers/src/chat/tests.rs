use super::{ChatCompletionsVideoProvider, ChatVideoPayload};
use a4n_domain::ProviderError as AppError;
use a4n_video_provider_contract::VideoProvider;
use base64::Engine as _;
use serde_json::json;

/// 最小可解码 MP4 ftyp 头：4 字节大小 + "ftyp" + brand + minor + compatible（共 16 字节）
fn minimal_mp4_bytes() -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&[0, 0, 0, 16]);
    buf.extend_from_slice(b"ftypmp42");
    buf.extend_from_slice(&[0, 0, 0, 0]);
    buf.extend_from_slice(b"mp42");
    buf
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn parse(body: serde_json::Value) -> Result<ChatVideoPayload, AppError> {
    ChatCompletionsVideoProvider::parse_response_for_test(&body.to_string())
}

#[test]
fn chat_auto_video_parses_message_video_object_url() {
    let body = json!({
        "choices": [{
            "message": { "video": { "url": "https://example.com/x.mp4" } }
        }]
    });
    let payload = parse(body).unwrap();
    match payload {
        ChatVideoPayload::Url { url, hint_format } => {
            assert_eq!(url, "https://example.com/x.mp4");
            assert_eq!(hint_format.as_deref(), Some("mp4"));
        }
        other => panic!("expected Url, got {other:?}"),
    }
}

#[test]
fn chat_auto_video_parses_message_video_b64_with_format() {
    let raw = b64(b"raw-video-bytes");
    let body = json!({
        "choices": [{
            "message": { "video": { "b64_json": raw, "format": "webm" } }
        }]
    });
    let payload = parse(body).unwrap();
    match payload {
        ChatVideoPayload::Base64 { format, bytes } => {
            assert_eq!(format.as_deref(), Some("webm"));
            assert_eq!(bytes, b"raw-video-bytes");
        }
        other => panic!("expected Base64, got {other:?}"),
    }
}

#[test]
fn chat_auto_video_parses_videos_array_first() {
    let body = json!({
        "choices": [{
            "message": {
                "videos": [
                    { "url": "https://example.com/x.webm", "content_type": "video/webm" }
                ]
            }
        }]
    });
    let payload = parse(body).unwrap();
    match payload {
        ChatVideoPayload::Url { url, hint_format } => {
            assert_eq!(url, "https://example.com/x.webm");
            assert_eq!(hint_format.as_deref(), Some("webm"));
        }
        other => panic!("expected Url, got {other:?}"),
    }
}

#[test]
fn chat_auto_video_parses_content_array_video_url_string() {
    let body = json!({
        "choices": [{
            "message": {
                "content": [
                    { "type": "text", "text": "preface" },
                    { "type": "video_url", "video_url": "https://example.com/x.mp4" }
                ]
            }
        }]
    });
    let payload = parse(body).unwrap();
    assert!(matches!(
        payload,
        ChatVideoPayload::Url { ref url, .. } if url == "https://example.com/x.mp4"
    ));
}

#[test]
fn chat_auto_video_parses_content_array_video_url_object() {
    let body = json!({
        "choices": [{
            "message": {
                "content": [
                    { "type": "video_url",
                      "video_url": { "url": "https://example.com/x.webm", "format": "webm" } }
                ]
            }
        }]
    });
    let payload = parse(body).unwrap();
    match payload {
        ChatVideoPayload::Url { url, hint_format } => {
            assert_eq!(url, "https://example.com/x.webm");
            assert_eq!(hint_format.as_deref(), Some("webm"));
        }
        other => panic!("expected Url, got {other:?}"),
    }
}

#[test]
fn chat_auto_video_parses_content_string_inline_data_uri() {
    let raw = b64(&minimal_mp4_bytes());
    let body = json!({
        "choices": [{
            "message": { "content": format!("Here:\n\ndata:video/mp4;base64,{raw}\n") }
        }]
    });
    let payload = parse(body).unwrap();
    assert!(matches!(payload, ChatVideoPayload::DataUri { ref format, .. } if format == "mp4"));
}

#[test]
fn chat_auto_video_parses_content_string_markdown_url() {
    let body = json!({
        "choices": [{
            "message": { "content": "前置 ![demo](https://example.com/x.mp4) 后置" }
        }]
    });
    let payload = parse(body).unwrap();
    match payload {
        ChatVideoPayload::Url { url, hint_format } => {
            assert_eq!(url, "https://example.com/x.mp4");
            assert_eq!(hint_format.as_deref(), Some("mp4"));
        }
        other => panic!("expected Url, got {other:?}"),
    }
}

#[test]
fn chat_auto_video_parses_content_string_bare_url() {
    let body = json!({
        "choices": [{
            "message": { "content": "你的视频已经生成：https://example.com/path/clip.webm" }
        }]
    });
    let payload = parse(body).unwrap();
    match payload {
        ChatVideoPayload::Url { url, hint_format } => {
            assert_eq!(url, "https://example.com/path/clip.webm");
            assert_eq!(hint_format.as_deref(), Some("webm"));
        }
        other => panic!("expected Url, got {other:?}"),
    }
}

#[test]
fn chat_auto_video_rejects_nested_download_url_outside_supported_fields() {
    let body = json!({
        "choices": [{
            "message": {
                "extras": {
                    "result": { "download_url": "https://cdn.example/out.mp4" }
                }
            }
        }]
    });
    let err = parse(body).unwrap_err();
    assert!(err.to_string().contains("视频响应未发现视频"));
}

#[test]
fn chat_auto_video_reports_diagnostic_when_nothing_found() {
    let body = json!({
        "choices": [{ "message": { "content": "just text, no video link" } }]
    });
    let err = parse(body).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("message.video"));
    assert!(msg.contains("message.content[].video_url"));
    assert!(msg.contains("响应预览"));
    assert!(msg.contains("just text"));
}

#[test]
fn chat_auto_video_rejects_missing_choices() {
    let body = json!({});
    let err = parse(body).unwrap_err();
    assert!(err.to_string().contains("choices[0].message"));
    assert!(err.to_string().contains("响应预览"));
}

#[test]
fn chat_auto_video_rejects_unsupported_data_uri_mime() {
    let raw = b64(&minimal_mp4_bytes());
    let body = json!({
        "choices": [{
            "message": { "content": format!("data:video/avi;base64,{raw}") }
        }]
    });
    assert!(parse(body).is_err());
}

#[tokio::test]
async fn chat_video_provider_poll_always_errors() {
    let provider = ChatCompletionsVideoProvider::new("http://127.0.0.1:9", "test-key").unwrap();
    let err = provider.poll("any-job-id").await.unwrap_err();
    assert!(err.to_string().contains("不支持轮询"));
}
