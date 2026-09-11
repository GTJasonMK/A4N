use a4n_domain::source_text_hash;
use a4n_image::ImageGenerationRequest;
use a4n_provider_contract::ImageProviderConfig;
use serde_json::{Value, json};

pub fn image_generation_request_trace_json(
    config: &ImageProviderConfig,
    request: &ImageGenerationRequest,
) -> Value {
    let reference_images = request
        .reference_images
        .iter()
        .enumerate()
        .map(|(index, reference)| {
            json!({
                "index": index,
                "label": reference.label,
                "data_uri_sha256": source_text_hash(&reference.data_uri),
                "data_uri_length": reference.data_uri.len(),
                "media_type": reference
                    .data_uri
                    .strip_prefix("data:")
                    .and_then(|value| value.split_once(';').map(|(media_type, _)| media_type)),
            })
        })
        .collect::<Vec<_>>();

    json!({
        "provider": {
            "config_id": config.id,
            "config_name": config.name,
            "backend": config.backend,
            "base_url": config.base_url,
            "configured_model": config.model,
        },
        "request": {
            "prompt": request.prompt,
            "width": request.width,
            "height": request.height,
            "aspect_ratio": image_request_aspect_ratio(request.width, request.height),
            "model": request.model,
            "reference_image_count": request.reference_images.len(),
            "reference_images": reference_images,
        },
    })
}

fn image_request_aspect_ratio(width: i32, height: i32) -> String {
    let gcd = gcd_i32(width.abs().max(1), height.abs().max(1));
    format!("{}:{}", width.abs().max(1) / gcd, height.abs().max(1) / gcd)
}

fn gcd_i32(mut left: i32, mut right: i32) -> i32 {
    while right != 0 {
        let next = left % right;
        left = right;
        right = next;
    }
    left.max(1)
}

#[cfg(test)]
pub(super) fn test_image_request_aspect_ratio(width: i32, height: i32) -> String {
    image_request_aspect_ratio(width, height)
}
