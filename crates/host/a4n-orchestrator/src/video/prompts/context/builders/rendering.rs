use std::collections::HashMap;

use a4n_prompt::request::compact_json_with_root_key_order;
use serde_json::{Value, json};

use a4n_error::AppError;

use a4n_prompt::rendering::PromptRenderer;

const VIDEO_USER_REQUEST_TEMPLATE: &str = "chapter_video_user_request";
const VIDEO_PAYLOAD_ROOT_KEY_ORDER: &[&str] = &[
    "task",
    "constraints",
    "output_contract",
    "chapter",
    "previous_steps",
    "current_segment",
    "reference_asset_summaries",
    "input_frames",
];

pub(super) async fn render_strict_json_user_prompt(
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    instruction: &str,
    payload: &Value,
) -> Result<String, AppError> {
    let payload = compact_json_with_root_key_order(payload, VIDEO_PAYLOAD_ROOT_KEY_ORDER)?;
    let mut variables = HashMap::new();
    variables.insert("instruction".to_string(), json!(instruction));
    variables.insert("payload".to_string(), json!(payload));
    prompt_renderer
        .render_prompt(VIDEO_USER_REQUEST_TEMPLATE, &variables, Some(project_id))
        .await
}

pub(super) fn chapter_analysis_context(chapter_analysis: Option<&Value>) -> Option<Value> {
    let analysis = chapter_analysis?;
    let mut object = serde_json::Map::new();
    if let Some(compressed) = analysis
        .get("summaries")
        .and_then(|v| v.get("compressed"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        object.insert(
            "compressed_summary".to_string(),
            Value::String(compressed.to_string()),
        );
    }
    if let Some(key_events) = analysis.get("key_events") {
        object.insert("key_events".to_string(), key_events.clone());
    }
    if let Some(metadata) = analysis.get("metadata") {
        object.insert("metadata".to_string(), metadata.clone());
    }
    if object.is_empty() {
        None
    } else {
        Some(Value::Object(object))
    }
}
