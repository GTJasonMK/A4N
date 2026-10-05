use super::{CliProxyImageProvider, cli_proxy_image_edit_error};
use a4n_domain::ProviderError;
use a4n_image_contract::{ImageGenerationRequest, ImageReference};
use reqwest::StatusCode;

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
fn unsupported_edit_endpoint_error_reports_cpa_instance_capability() {
    let error = cli_proxy_image_edit_error(
        "grok-imagine-image",
        StatusCode::FORBIDDEN,
        "{\"error\":{\"message\":\"Model does not support this API endpoint\"}}",
    );

    assert!(matches!(&error, ProviderError::InvalidArgument(_)));
    let message = error.to_string();
    assert!(message.contains("grok-imagine-image"));
    assert!(message.contains("当前 CPA 实例、账号或模型配置"));
    assert!(message.contains("/images/edits"));
    assert!(message.contains("/images/generations"));
    assert!(!message.contains("quality"));
}

#[test]
fn edit_preview_uses_all_reference_images() {
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

    let body = CliProxyImageProvider::request_body_preview(&req).unwrap();

    let files = body["files"].as_array().expect("files should be an array");
    assert_eq!(files.len(), 2);
    assert_eq!(files[0]["label"], "参考图 1");
    assert_eq!(files[1]["label"], "参考图 2");
}
