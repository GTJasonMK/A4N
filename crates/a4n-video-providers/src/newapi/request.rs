use a4n_video::VideoGenerationRequest;
use serde_json::{Map, Value, json};

pub(super) fn is_veo_model(model: &str) -> bool {
    model.trim().to_ascii_lowercase().contains("veo")
}

pub(super) fn size_from_request(req: &VideoGenerationRequest) -> String {
    format!("{}x{}", req.width.max(1), req.height.max(1))
}

pub(super) fn veo_metadata(
    req: &VideoGenerationRequest,
    aspect_ratio: Option<&str>,
    negative_prompt: Option<&str>,
) -> Value {
    let mut metadata = Map::new();
    metadata.insert("durationSeconds".into(), json!(req.duration_secs));
    metadata.insert(
        "resolution".into(),
        json!(veo_resolution_from_size(req.width, req.height)),
    );
    if let Some(aspect_ratio) = aspect_ratio {
        metadata.insert("aspectRatio".into(), json!(aspect_ratio));
    }
    if let Some(negative_prompt) = negative_prompt {
        metadata.insert("negativePrompt".into(), json!(negative_prompt));
    }
    Value::Object(metadata)
}

pub(super) fn veo_resolution_from_size(width: i32, height: i32) -> &'static str {
    let max_dim = width.max(height);
    if max_dim >= 3840 {
        "4k"
    } else if max_dim >= 1920 {
        "1080p"
    } else {
        "720p"
    }
}

pub(super) fn aspect_ratio_from_size(width: i32, height: i32) -> Option<&'static str> {
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
    None
}
