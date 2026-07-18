use a4n_core::error::AppError;
use a4n_prompt::blueprint_generation::parse_blueprint_stage_json_object;
use serde_json::Value;

mod fingerprint;

pub(super) use fingerprint::{blueprint_stage_call_fingerprint_json, blueprint_stage_call_key};

pub(super) fn restored_blueprint_stage_output(
    checkpoint_json: Value,
    stage_name: &str,
    expected_top_level_keys: &[&str],
) -> Result<Value, AppError> {
    checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取蓝图生成阶段 {stage_name} 调用级断点"),
        })?;
    let parsed_output =
        checkpoint_json
            .get("parsed_output")
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_checkpoint_parsed_output".to_string(),
                operation: format!("读取蓝图生成阶段 {stage_name} 调用级断点"),
            })?;
    parse_blueprint_stage_json_object(
        &serde_json::to_string(parsed_output)?,
        stage_name,
        expected_top_level_keys,
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn restores_matching_stage_checkpoint() {
        let checkpoint = json!({
            "response_content": r#"{"world_setting":{"premise":"潮汐城"}}"#,
            "parsed_output": {"world_setting": {"premise": "潮汐城"}},
        });

        let restored =
            restored_blueprint_stage_output(checkpoint, "阶段 2/4：世界观", &["world_setting"])
                .expect("matching checkpoint should restore");

        assert_eq!(restored["world_setting"]["premise"], "潮汐城");
    }

    #[test]
    fn restores_repaired_stage_checkpoint_from_parsed_output() {
        let checkpoint = json!({
            "response_content": r#"{"world_setting":{"premise":"潮汐城"}}"#,
            "parsed_output": {"world_setting": {"premise": "旧城"}},
        });

        let restored =
            restored_blueprint_stage_output(checkpoint, "阶段 2/4：世界观", &["world_setting"])
                .expect("repaired checkpoint should restore parsed output");

        assert_eq!(restored["world_setting"]["premise"], "旧城");
    }

    #[test]
    fn rejects_stage_checkpoint_without_response_content() {
        let error = restored_blueprint_stage_output(
            json!({"parsed_output": {"world_setting": {}}}),
            "阶段 2/4：世界观",
            &["world_setting"],
        )
        .expect_err("missing response content must fail");

        assert!(
            error
                .to_string()
                .contains("missing_checkpoint_response_content")
        );
    }
}
