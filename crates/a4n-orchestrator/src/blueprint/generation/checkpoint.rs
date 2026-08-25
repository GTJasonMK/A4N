use a4n_core::error::AppError;
use a4n_prompt::blueprint_generation::parse_blueprint_stage_json_object;
use serde_json::Value;

use crate::structured_llm::ensure_checkpoint_parsed_output_matches;

mod fingerprint;

pub(super) use fingerprint::blueprint_stage_call_fingerprint_json;

pub(super) fn restored_blueprint_stage_output(
    parsed_output: &Value,
    response_content: &str,
    stage_name: &str,
    output_schema: &str,
) -> Result<Value, AppError> {
    let reparsed = parse_blueprint_stage_json_object(response_content, stage_name, output_schema)?;
    ensure_checkpoint_parsed_output_matches(
        parsed_output,
        &reparsed,
        &format!("蓝图生成阶段 {stage_name}"),
    )?;
    Ok(reparsed)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn restores_matching_stage_checkpoint() {
        let restored = restored_blueprint_stage_output(
            &json!({"world_setting": {"premise": "潮汐城"}}),
            r#"{"world_setting":{"premise":"潮汐城"}}"#,
            "阶段 2/4：世界观",
            r#"{"world_setting":{}}"#,
        )
        .expect("matching checkpoint should restore");

        assert_eq!(restored["world_setting"]["premise"], "潮汐城");
    }

    #[test]
    fn rejects_mismatched_stage_checkpoint() {
        let error = restored_blueprint_stage_output(
            &json!({"world_setting": {"premise": "旧城"}}),
            r#"{"world_setting":{"premise":"潮汐城"}}"#,
            "阶段 2/4：世界观",
            r#"{"world_setting":{}}"#,
        )
        .expect_err("mismatched checkpoint must fail");

        assert!(
            error
                .to_string()
                .contains("checkpoint_parsed_output_mismatch")
        );
    }
}
