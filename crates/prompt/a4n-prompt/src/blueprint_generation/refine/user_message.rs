use std::collections::HashMap;

use a4n_domain::project::{
    BLUEPRINT_REFINEMENT_FIELDS, BLUEPRINT_REFINEMENT_USER_PROMPT,
    normalized_blueprint_refinement_focus,
};
use a4n_error::AppError;
use serde_json::{Value, json};

use crate::rendering::PromptRenderer;

pub(super) async fn blueprint_refine_manifest_user_message(
    prompt_renderer: &dyn PromptRenderer,
    project_id: Option<i64>,
    focus: Option<&str>,
) -> Result<String, AppError> {
    let mut payload = serde_json::Map::new();
    payload.insert(
        "context_refs".to_string(),
        json!({
            "blueprint_context": "稳定上下文包 existing_blueprint_context",
            "inspiration_conversation": "稳定上下文包 inspiration_conversation_context",
        }),
    );
    if let Some(focus) = normalized_blueprint_refinement_focus(focus) {
        payload.insert("focus".to_string(), json!(focus));
    }
    payload.insert(
        "output_contract".to_string(),
        json!({
            "root_type": "object",
            "allowed_fields": BLUEPRINT_REFINEMENT_FIELDS,
            "schema": a4n_domain::project::blueprint_refinement_output_schema(),
        }),
    );
    let payload = serde_json::to_string(&Value::Object(payload))?;
    let variables = HashMap::from([("payload".to_string(), json!(payload))]);
    prompt_renderer
        .render_prompt_for_call(
            "a4n_prompt::blueprint_generation::build_blueprint_refine_request",
            BLUEPRINT_REFINEMENT_USER_PROMPT,
            &variables,
            project_id,
        )
        .await
}
