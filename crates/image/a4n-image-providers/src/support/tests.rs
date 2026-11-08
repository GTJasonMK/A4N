use super::reference::{MAX_REFERENCE_IMAGE_BYTES, decode_data_uri_image};
use super::*;
use a4n_domain::ProviderError as AppError;
use a4n_image_contract::ImageReference;
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
fn rejects_fake_magic_headers_and_decodes_all_supported_formats() {
    for fake in [
        b"\x89PNG\r\n\x1a\nrest".as_slice(),
        b"\xff\xd8\xffrest",
        b"RIFFxxxxWEBPrest",
        b"not-an-image",
        b"",
    ] {
        assert!(detect_format_from_bytes(fake).is_err());
    }
    for (format, extension) in [
        (image::ImageFormat::Png, "png"),
        (image::ImageFormat::Jpeg, "jpg"),
        (image::ImageFormat::WebP, "webp"),
    ] {
        assert_eq!(
            detect_format_from_bytes(&image_bytes(format)).unwrap(),
            extension
        );
    }
}

fn image_bytes(format: image::ImageFormat) -> Vec<u8> {
    let image = image::RgbImage::from_pixel(3, 2, image::Rgb([25, 50, 75]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, format).unwrap();
    bytes.into_inner()
}

#[test]
fn image_format_requires_decodable_bytes_and_matching_declared_format() {
    let bytes = image_bytes(image::ImageFormat::Png);
    assert_eq!(image_format_hint(None, &bytes).unwrap(), "png");
    assert_eq!(image_format_hint(Some("png"), &bytes).unwrap(), "png");
    assert!(image_format_hint(Some("jpg"), &bytes).is_err());
    assert!(image_format_hint(Some("png"), &bytes[..bytes.len() / 2]).is_err());
    assert!(image_format_hint(Some("png"), b"<html>error</html>").is_err());
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

#[tokio::test]
async fn retries_transient_image_download_failures() {
    use tokio::io::AsyncWriteExt;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind image test server");
    let url = format!(
        "http://{}/generated",
        listener.local_addr().expect("image test server address")
    );
    let png = image_bytes(image::ImageFormat::Png);
    let expected = png.clone();
    let server = tokio::spawn(async move {
        for (declared_length, body) in [(12, b"\x89PNG".as_slice()), (png.len(), png.as_slice())] {
            let (mut stream, _) = listener.accept().await.expect("accept image request");
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {declared_length}\r\nConnection: close\r\n\r\n"
            );
            stream
                .write_all(headers.as_bytes())
                .await
                .expect("write image response headers");
            stream
                .write_all(body)
                .await
                .expect("write image response body");
            stream.shutdown().await.expect("close image response");
        }
    });

    let client = build_image_http_client().expect("image HTTP client");
    let (bytes, format) = download_image_url(&client, &url, None)
        .await
        .expect("transient image download should be retried");

    assert_eq!(format, "png");
    assert_eq!(bytes, expected);
    server.await.expect("image test server");
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
