use serde_json::{Map, Value, json};

use a4n_video_sidecar_contract::VideoGenerationRequest;

pub(super) fn chat_request_body(req: &VideoGenerationRequest, user_content: String) -> Value {
    let mut body = Map::new();
    body.insert("model".into(), json!(req.model));
    body.insert("modalities".into(), json!(["video", "text"]));
    body.insert("n".into(), json!(1));
    body.insert(
        "size".into(),
        json!(format!("{}x{}", req.width, req.height)),
    );
    body.insert("duration_secs".into(), json!(req.duration_secs));
    body.insert("fps".into(), json!(req.fps));
    body.insert("negative_prompt".into(), json!(req.negative_prompt));
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
    body.insert(
        "messages".into(),
        json!([
            { "role": "user", "content": user_content }
        ]),
    );
    Value::Object(body)
}
