use a4n_image::ImageGenerationRequest;
use serde_json::{Value, json};

pub(super) fn chat_request_body(req: &ImageGenerationRequest) -> Value {
    json!({
        "model": req.model,
        "stream": false,
        "modalities": ["image", "text"],
        "n": 1,
        "size": format!("{}x{}", req.width, req.height),
        "messages": [
            { "role": "user", "content": image_user_content(req) }
        ],
    })
}

fn image_user_content(req: &ImageGenerationRequest) -> Value {
    let user_text = image_user_text(req);
    if req.reference_images.is_empty() {
        return Value::String(user_text);
    }

    let mut parts = vec![json!({ "type": "text", "text": user_text })];
    for image in &req.reference_images {
        parts.push(json!({
            "type": "image_url",
            "image_url": {
                "url": image.data_uri,
            },
        }));
    }
    Value::Array(parts)
}

fn image_user_text(req: &ImageGenerationRequest) -> String {
    // 把尺寸提示拼到 user message 里，部分网关靠 prompt 文本决定尺寸；
    // 其他参数 `size` / `modalities` / `n` 走 OpenAI 扩展。
    if req.reference_images.is_empty() {
        return format!(
            "{}\n\n[输出图片，尺寸 {}x{}]",
            req.prompt, req.width, req.height
        );
    }

    let refs = req
        .reference_images
        .iter()
        .enumerate()
        .map(|(index, image)| format!("{}. {}", index + 1, image.label))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{}\n\n[输出图片，尺寸 {}x{}]\n\n参考图绑定：\n{}",
        req.prompt, req.width, req.height, refs
    )
}
