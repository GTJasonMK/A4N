use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::{Value, json};

use crate::rendering::PromptRenderer;

pub async fn blueprint_refine_manifest_user_message(
    prompt_renderer: &dyn PromptRenderer,
    project_id: Option<i64>,
    focus: Option<&str>,
) -> Result<String, AppError> {
    let mut payload = serde_json::Map::new();
    payload.insert(
        "context_refs".to_string(),
        json!({
            "blueprint_context": "稳定上下文包 existing_blueprint_context",
        }),
    );
    if let Some(focus) = focus.map(str::trim).filter(|value| !value.is_empty()) {
        payload.insert("focus".to_string(), json!(focus));
    }
    payload.insert(
        "output_contract".to_string(),
        json!({
            "root_type": "object",
            "allowed_fields": ["title", "genre", "style", "tone", "target_audience", "one_sentence_summary", "full_synopsis", "world_setting", "characters", "relationships"],
            "forbidden": ["markdown", "explanation", "legacy_alias_fields"],
        }),
    );
    let payload = serde_json::to_string_pretty(&Value::Object(payload))?;
    let variables = HashMap::from([("payload".to_string(), json!(payload))]);
    prompt_renderer
        .render_prompt("blueprint_optimization_user", &variables, project_id)
        .await
}
