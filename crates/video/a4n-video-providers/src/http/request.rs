use serde_json::{Map, Value, json};

use a4n_video_sidecar_contract::VideoGenerationRequest;

use super::HttpVideoProvider;

impl HttpVideoProvider {
    pub(crate) fn request_body(req: &VideoGenerationRequest) -> Value {
        let mut body = Map::new();
        body.insert("model".into(), json!(req.model));
        body.insert("prompt".into(), json!(req.prompt));
        body.insert("negative_prompt".into(), json!(req.negative_prompt));
        body.insert("width".into(), json!(req.width));
        body.insert("height".into(), json!(req.height));
        body.insert("duration_secs".into(), json!(req.duration_secs));
        body.insert("fps".into(), json!(req.fps));
        body.insert(
            "reference_image_urls".into(),
            json!(req.media.reference_images()),
        );
        if let Some(image) = req.media.first_frame() {
            body.insert("image".into(), json!({ "url": image }));
        }
        if let Some(image) = req.media.last_frame() {
            body.insert("last_frame".into(), json!({ "url": image }));
        }
        body.insert("response_format".into(), json!("b64_json"));
        Value::Object(body)
    }
}
