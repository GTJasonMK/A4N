use serde_json::{Map, Value, json};

use super::super::types::VideoGenerationRequest;

pub(super) fn chat_request_body(req: &VideoGenerationRequest, user_content: String) -> Value {
    let has_boundary_frames =
        req.first_frame_image_url.is_some() || req.last_frame_image_url.is_some();
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
        json!(if has_boundary_frames {
            Vec::<String>::new()
        } else {
            req.reference_image_urls.clone()
        }),
    );
    if let Some(image) = req.first_frame_image_url.as_ref() {
        body.insert("image".into(), json!({ "url": image }));
        body.insert("first_frame_image_url".into(), json!(image));
    }
    if let Some(image) = req.last_frame_image_url.as_ref() {
        body.insert("last_frame".into(), json!({ "url": image }));
        body.insert("last_frame_image_url".into(), json!(image));
    }
    body.insert(
        "messages".into(),
        json!([
            { "role": "user", "content": user_content }
        ]),
    );
    Value::Object(body)
}
