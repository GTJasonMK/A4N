use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::{Value, json};

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_prompt::rendering::PromptRenderer;

use super::super::contracts::MANGA_PIPELINE_PROMPT_NAMES;

pub(in crate::manga) async fn manga_pipeline_prompt_fingerprint(
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    variables: &HashMap<String, Value>,
) -> Result<Value, AppError> {
    let mut prompts = Vec::with_capacity(MANGA_PIPELINE_PROMPT_NAMES.len());
    for name in MANGA_PIPELINE_PROMPT_NAMES {
        let rendered = prompt_renderer
            .render_prompt(name, variables, Some(project_id))
            .await?;
        prompts.push(json!({
            "name": name,
            "sha256": source_content_sha256(&rendered),
        }));
    }
    Ok(json!({
        "schema_version": 2,
        "prompts": prompts,
    }))
}
