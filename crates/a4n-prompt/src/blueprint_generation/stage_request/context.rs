use super::*;

mod previous;
mod project_seed;

use previous::previous_stage_pack;
use project_seed::project_seed_pack;

pub(super) fn build_blueprint_stage_context_manifest(
    user_payload_json: &str,
    previous_context: Option<&Value>,
    dynamic_user_content: String,
    stage_key: &str,
    stage_name: &str,
    output_schema: &Value,
) -> Result<LlmContextManifest, AppError> {
    let source_input: Value = serde_json::from_str(user_payload_json)?;
    let mut ledger = LlmContextLedger::new();
    ledger.add_pack(project_seed_pack(source_input)?)?;
    if let Some(previous_context) = previous_context {
        ledger.add_pack(previous_stage_pack(previous_context, stage_key)?)?;
    }
    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(format!(
            "当前阶段 {stage_name} 必须只返回严格 JSON 对象；顶层字段必须匹配 output_schema：{}",
            serde_json::to_string(output_schema)?
        )),
    ))
}
