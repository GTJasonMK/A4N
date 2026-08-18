use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::{Value, json};

use crate::rendering::PromptRenderer;

pub(super) async fn build_blueprint_stage_manifest_user_message(
    prompt_renderer: &dyn PromptRenderer,
    project_id: Option<i64>,
    stage_key: &str,
    stage_name: &str,
    output_schema: &Value,
) -> Result<String, AppError> {
    let payload = json!({
        "context_refs": {
            "source_input": "稳定上下文包 blueprint_project_seed_context.source_input",
            "previous_context": "稳定上下文包 blueprint_previous_stage_outputs，仅在当前阶段需要上游阶段时存在",
        },
        "current_stage": {
            "key": stage_key,
            "name": stage_name,
            "output_schema": output_schema,
        },
    });
    let payload = serde_json::to_string_pretty(&payload)?;
    let variables = HashMap::from([
        ("stage_key".to_string(), json!(stage_key)),
        ("payload".to_string(), json!(payload)),
    ]);
    prompt_renderer
        .render_prompt(
            a4n_core::blueprint::BLUEPRINT_STAGE_USER_PROMPT,
            &variables,
            project_id,
        )
        .await
}
