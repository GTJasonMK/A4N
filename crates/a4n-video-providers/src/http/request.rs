use serde_json::{Map, Value, json};

use a4n_video::VideoGenerationRequest;

use super::HttpVideoProvider;

impl HttpVideoProvider {
    pub(super) fn request_body(req: &VideoGenerationRequest) -> Value {
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
            json!(req.reference_image_urls.clone()),
        );
        if let Some(image) = req.first_frame_image_url.as_ref() {
            body.insert("image".into(), json!({ "url": image }));
            body.insert("first_frame_image_url".into(), json!(image));
        }
        if let Some(image) = req.last_frame_image_url.as_ref() {
            body.insert("last_frame".into(), json!({ "url": image }));
            body.insert("last_frame_image_url".into(), json!(image));
        }
        body.insert("response_format".into(), json!("b64_json"));
        Value::Object(body)
    }

    pub fn request_body_preview(req: &VideoGenerationRequest) -> Value {
        Self::request_body(req)
    }
}
