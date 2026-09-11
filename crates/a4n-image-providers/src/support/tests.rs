use super::reference::{MAX_REFERENCE_IMAGE_BYTES, decode_data_uri_image};
use super::*;
use a4n_domain::ProviderError as AppError;
use a4n_image::ImageReference;
use base64::Engine as _;
use reqwest::StatusCode;
#[test]
fn normalizes_supported_declared_formats() {
    assert_eq!(normalize_image_format("png").unwrap(), "png");
    assert_eq!(normalize_image_format(".JPEG").unwrap(), "jpg");
    assert_eq!(normalize_image_format("webp").unwrap(), "webp");
    assert!(normalize_image_format("").is_err());
    assert!(normalize_image_format("gif").is_err());
}

#[test]
fn detects_known_image_magic_without_default_format() {
    assert_eq!(
        detect_format_from_bytes(b"\x89PNG\r\n\x1a\nrest").unwrap(),
        "png"
    );
    assert_eq!(
        detect_format_from_bytes(b"\xff\xd8\xffrest").unwrap(),
        "jpg"
    );
    assert_eq!(
        detect_format_from_bytes(b"RIFFxxxxWEBPrest").unwrap(),
        "webp"
    );
    assert!(detect_format_from_bytes(b"not-an-image").is_err());
}

#[test]
fn infers_image_format_from_url_content_type_or_bytes() {
    assert_eq!(
        image_format_hint(None, Some("https://example.com/a.jpeg?x=1"), None, b"").unwrap(),
        "jpg"
    );
    assert_eq!(
        image_format_hint(None, None, Some("image/webp; charset=utf-8"), b"").unwrap(),
        "webp"
    );
    assert_eq!(
        image_format_hint(None, None, None, b"\x89PNG\r\n\x1a\nrest").unwrap(),
        "png"
    );
}

#[test]
fn reads_optional_format_or_mime_type_from_response_item() {
    assert_eq!(
        optional_format_from_response_item(&serde_json::json!({"format": "jpeg"})).unwrap(),
        Some("jpg".into())
    );
    assert_eq!(
        optional_format_from_response_item(&serde_json::json!({"mime_type": "image/jpeg"}))
            .unwrap(),
        Some("jpg".into())
    );
    assert_eq!(
        optional_format_from_response_item(&serde_json::json!({})).unwrap(),
        None
    );
}

#[test]
fn content_policy_http_error_is_invalid_argument() {
    let body = r#"{"error":{"message":"抱歉，我不能根据这个描述生成图像，因为其中包含明确的自我伤害/流血与即将接触尖锐物体的情节描写。","type":"invalid_request_error","param":"","code":"content_policy_violation"}}"#;

    let err = image_provider_http_error("图像生成", StatusCode::BAD_REQUEST, body);

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("内容安全策略拒绝")
                && message.contains("自我伤害")
                && message.contains("请修改提示词")
    ));
}

#[test]
fn invalid_request_http_error_is_invalid_argument() {
    let body = r#"{"error":{"message":"size is invalid","type":"invalid_request_error","code":"bad_request"}}"#;

    let err = image_provider_http_error("图像生成", StatusCode::BAD_REQUEST, body);

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("请求被供应商拒绝") && message.contains("size is invalid")
    ));
}

#[test]
fn non_client_provider_http_error_stays_internal() {
    let body = r#"{"error":{"message":"upstream unavailable"}}"#;

    let err = image_provider_http_error("图像生成", StatusCode::BAD_GATEWAY, body);

    assert!(matches!(
        err,
        AppError::Internal(error) if error.contains("502 Bad Gateway")
    ));
}

#[test]
fn decodes_data_uri_input_image() {
    let decoded = decode_data_uri_image(
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
    )
    .unwrap();

    assert_eq!(decoded.mime_type, "image/png");
    assert_eq!(decoded.format, "png");
    assert!(decoded.bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn rejects_data_uri_with_mismatched_mime() {
    let err = decode_data_uri_image(
        "data:image/jpeg;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
    )
    .unwrap_err();

    assert!(err.to_string().contains("不一致"));
}

#[test]
fn prepares_large_reference_image_as_compact_jpeg() {
    let image = image::RgbImage::from_pixel(1024, 1024, image::Rgb([220, 40, 30]));
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let data_uri = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    );

    let prepared = prepare_reference_image_for_upload(&data_uri).unwrap();

    assert_eq!(prepared.mime_type, "image/jpeg");
    assert_eq!(prepared.format, "jpg");
    assert!(prepared.bytes.starts_with(b"\xff\xd8\xff"));
    assert!(prepared.bytes.len() < MAX_REFERENCE_IMAGE_BYTES);
}

#[test]
fn rejects_system_time_before_unix_epoch_for_provider_seed() {
    let before_epoch = std::time::UNIX_EPOCH - std::time::Duration::from_secs(1);
    let error = super::time_seed_at(before_epoch).expect_err("pre-epoch time must fail");
    assert!(error.to_string().contains("Unix epoch"));
}

#[test]
fn shares_reference_prompt_and_multipart_field_rules() {
    let references = vec![
        ImageReference {
            label: "参考图 1".into(),
            data_uri: "data:image/png;base64,one".into(),
        },
        ImageReference {
            label: "参考图 2".into(),
            data_uri: "data:image/png;base64,two".into(),
        },
    ];

    assert_eq!(reference_image_field_name(1), "image");
    assert_eq!(reference_image_field_name(2), "image[]");
    assert_eq!(
        prompt_with_reference_labels("draw", &references),
        "draw\n\n参考图绑定：\n1. 参考图 1\n2. 参考图 2"
    );
}
