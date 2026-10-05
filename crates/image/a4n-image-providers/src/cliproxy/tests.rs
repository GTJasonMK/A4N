use super::{CliProxyImageProvider, cli_proxy_image_edit_error};
use a4n_domain::ProviderError;
use a4n_image_contract::{ImageGenerationRequest, ImageReference};
use reqwest::StatusCode;

const PNG_1X1_B64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

fn request() -> ImageGenerationRequest {
    ImageGenerationRequest {
        prompt: "draw".into(),
        width: 1280,
        height: 720,
        model: "grok-imagine-image-quality".into(),
        reference_images: Vec::new(),
    }
}

fn reference(index: usize) -> ImageReference {
    ImageReference {
        label: format!("参考图 {index}"),
        data_uri: format!("data:image/png;base64,{PNG_1X1_B64}"),
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
fn unsupported_edit_endpoint_error_reports_gateway_contract() {
    let error = cli_proxy_image_edit_error(
        "grok-imagine-image",
        StatusCode::FORBIDDEN,
        "{\"error\":{\"message\":\"Model does not support this API endpoint\"}}",
    );

    assert!(matches!(&error, ProviderError::InvalidArgument(_)));
    let message = error.to_string();
    assert!(message.contains("grok-imagine-image"));
    assert!(message.contains("CPAproxy/Gateway"));
    assert!(message.contains("images_xai"));
    assert!(message.contains("image.url/images[].url"));
    assert!(message.contains("/images/edits"));
    assert!(message.contains("/images/generations"));
    assert!(!message.contains("quality"));
}

#[test]
fn edit_body_uses_xai_single_image_contract_without_aspect_ratio() {
    let mut req = request();
    req.reference_images = vec![reference(1)];

    let body = CliProxyImageProvider::edit_body(&req).unwrap();

    assert_eq!(body["image"]["url"], req.reference_images[0].data_uri);
    assert!(body.get("images").is_none());
    assert!(body.get("aspect_ratio").is_none());
}

#[test]
fn edit_body_uses_xai_multiple_image_contract_and_aspect_ratio() {
    let mut req = request();
    req.reference_images = vec![reference(1), reference(2)];

    let body = CliProxyImageProvider::edit_body(&req).unwrap();
    let images = body["images"].as_array().expect("images array");

    assert_eq!(images.len(), 2);
    assert_eq!(images[0]["url"], req.reference_images[0].data_uri);
    assert_eq!(images[1]["url"], req.reference_images[1].data_uri);
    assert!(body.get("image").is_none());
    assert_eq!(body["aspect_ratio"], "16:9");
}

#[test]
fn edit_body_rejects_more_than_three_xai_references() {
    let mut req = request();
    req.reference_images = (1..=4).map(reference).collect();

    let error = CliProxyImageProvider::edit_body(&req).unwrap_err();

    assert!(matches!(error, ProviderError::InvalidArgument(_)));
    assert!(error.to_string().contains("最多支持 3 张参考图"));
}

#[test]
fn spending_limit_error_reports_api_billing_boundary() {
    let error = cli_proxy_image_edit_error(
        "grok-imagine-image-2.0",
        StatusCode::FORBIDDEN,
        r#"{"code":"personal-team-blocked:spending-limit"}"#,
    );

    assert!(matches!(&error, ProviderError::InvalidArgument(_)));
    let message = error.to_string();
    assert!(message.contains("XAI API/team 消费策略"));
    assert!(message.contains("API billing"));
    assert!(message.contains("grok-imagine-image-2.0"));
}

#[test]
fn edit_preview_uses_xai_json_contract_and_all_reference_images() {
    let mut req = request();
    req.reference_images = vec![reference(1), reference(2)];

    let body = CliProxyImageProvider::request_body_preview(&req).unwrap();
    let images = body["fields"]["images"]
        .as_array()
        .expect("xAI images array");

    assert_eq!(body["content_type"], "application/json");
    assert_eq!(images.len(), 2);
    assert!(
        images[0]["url"]
            .as_str()
            .unwrap()
            .starts_with("data:image/png;base64,")
    );
}
