use a4n_error::AppError;
use a4n_llm_context::{LlmContextLedger, LlmContextManifest};
use serde_json::Value;

mod previous;
mod project_seed;

use previous::previous_stage_pack;
use project_seed::project_seed_pack;

pub(super) fn build_blueprint_stage_context_manifest(
    user_payload_json: &str,
    previous_context: Option<&Value>,
    dynamic_user_content: String,
    stage_key: &str,
) -> Result<LlmContextManifest, AppError> {
    let source_input: Value = serde_json::from_str(user_payload_json)?;
    let mut ledger = LlmContextLedger::new();
    ledger.add_pack(project_seed_pack(source_input)?)?;
    if let Some(previous_context) = previous_context {
        ledger.add_pack(previous_stage_pack(previous_context, stage_key)?)?;
    }
    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(
            "只返回 current_stage.output_schema 要求的严格 JSON 对象，不要输出其他阶段字段。"
                .to_string(),
        ),
    ))
}
