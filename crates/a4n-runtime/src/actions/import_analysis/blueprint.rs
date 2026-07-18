use crate::actions::import_reverse::{ReverseEngineeringTarget, build_reverse_request_from_sample};
use crate::state::AppState;
use a4n_core::import_reverse::{ReverseContentRequest, ReverseContextKind};
use a4n_orchestrator::import_reverse::resolve_reverse_content_sample;
use serde_json::Value;

pub(super) async fn apply_reverse_blueprint(
    state: &AppState,
    project_id: i64,
) -> Result<(), String> {
    let rows = state
        .orchestrator
        .reverse_context_rows(project_id, true)
        .await
        .map_err(|e| e.to_string())?;
    let content_sample = resolve_reverse_content_sample(
        ReverseContentRequest {
            project_id,
            manual_sample: "",
            auto_context: true,
            context_limit: Some(50),
        },
        ReverseContextKind::Blueprint,
        Some(&rows),
    )?;
    let variables: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
    let target = ReverseEngineeringTarget::Blueprint;
    let request =
        build_reverse_request_from_sample(state, project_id, target, &content_sample, &variables)
            .await
            .map_err(|e| e.to_string())?;
    let result = state
        .reverse_engineer_request(target.prompt_name(), request, None)
        .await
        .map_err(|e| e.to_string())?;
    let applied = state
        .apply_reverse_blueprint_data(project_id, &result)
        .await
        .map_err(|e| e.to_string())?;
    let character_count = required_applied_count(&applied, "character_count")?;
    let relationship_count = required_applied_count(&applied, "relationship_count")?;
    tracing::info!(
        project_id,
        character_count,
        relationship_count,
        "项目蓝图反推完成"
    );
    Ok(())
}

fn required_applied_count(value: &Value, field: &str) -> Result<u64, String> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("项目蓝图反推结果缺少非负整数字段: {field}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn required_applied_count_rejects_missing_or_invalid_count() {
        assert_eq!(
            required_applied_count(&json!({"character_count": 2}), "character_count").unwrap(),
            2
        );
        assert!(
            required_applied_count(&json!({}), "character_count")
                .expect_err("missing count must fail")
                .contains("缺少非负整数字段")
        );
        assert!(
            required_applied_count(&json!({"character_count": -1}), "character_count")
                .expect_err("negative count must fail")
                .contains("缺少非负整数字段")
        );
    }
}
