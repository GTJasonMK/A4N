use super::{
    client::{prompt_client_for_profile, prompt_service_request},
    overrides::rollback_project_override_runtime_after_persist_error,
    queries::reverse_dependencies_for_entries,
};
use crate::actions::template_variables;
use a4n_capabilities::A4nCapabilities;
use a4n_orchestrator::prompt_override::ProjectOverrideIdentity;
use a4n_prompt::{
    engine::{PromptEngine, PromptEntry},
    tera_engine::TeraPromptEngine,
};
use a4n_service_protocol::SecretForwardingPolicy;
use chrono::Utc;
use std::collections::HashMap;

#[test]
fn prompt_sidecar_profile_rejects_missing_capability_without_embedded_fallback() {
    let error = prompt_client_for_profile(
        crate::state::RuntimeProfile::Sidecar,
        &A4nCapabilities::default(),
    )
    .err()
    .expect("sidecar profile without prompt capability must fail");

    assert!(error.contains("prompt sidecar capability 未配置"));
    assert!(error.contains("不能回退到 embedded prompt engine"));
}

#[test]
fn prompt_non_sidecar_profile_explicitly_uses_embedded_engine() {
    assert!(
        prompt_client_for_profile(
            crate::state::RuntimeProfile::Minimal,
            &A4nCapabilities::default(),
        )
        .expect("non-sidecar profile should be accepted")
        .is_none()
    );
}

fn prompt_entry(name: &str, content: &str, dependencies: Vec<&str>) -> PromptEntry {
    PromptEntry {
        name: name.to_string(),
        title: name.to_string(),
        content: content.to_string(),
        description: Some(format!("{name} description")),
        tags: vec!["test".to_string()],
        category: "writing".to_string(),
        dependencies: dependencies.into_iter().map(ToString::to_string).collect(),
        is_modified: false,
        version: "1.0".to_string(),
        modified_at: Utc::now().naive_utc(),
        used_by: Vec::new(),
        path: Some(format!("{name}.md")),
    }
}

#[test]
fn parse_template_variables_allows_missing_null_or_object_values() {
    assert!(
        template_variables::parse_template_variables(None)
            .unwrap()
            .is_empty()
    );
    assert!(
        template_variables::parse_template_variables(Some(&serde_json::Value::Null))
            .unwrap()
            .is_empty()
    );
    let variables =
        template_variables::parse_template_variables(Some(&serde_json::json!({"tone": "warm"})))
            .unwrap();
    assert_eq!(variables["tone"], serde_json::json!("warm"));
}

#[test]
fn parse_template_variables_rejects_explicit_non_object_values() {
    let err = template_variables::parse_template_variables(Some(&serde_json::json!(["tone"])))
        .unwrap_err();
    assert!(err.contains("variables 必须是对象或 null"));
}

#[test]
fn reverse_dependencies_requires_existing_prompt_name() {
    let entries = vec![prompt_entry("outline", "大纲", vec!["blueprint"])];
    let err = reverse_dependencies_for_entries(&entries, "missing").unwrap_err();
    assert!(err.contains("提示词不存在：missing"));
}

#[test]
fn reverse_dependencies_returns_dependents_for_existing_prompt_name() {
    let entries = vec![
        prompt_entry("outline", "大纲", vec!["blueprint"]),
        prompt_entry("writing", "正文", vec!["outline"]),
        prompt_entry("blueprint", "蓝图", vec![]),
    ];
    assert_eq!(
        reverse_dependencies_for_entries(&entries, "outline").unwrap(),
        vec!["writing".to_string()]
    );
}

#[test]
fn prompt_sidecar_request_requires_complete_project_scope() {
    let request = prompt_service_request(Some(42), Some(3), serde_json::json!({"project_id": 42}))
        .expect("project-scoped prompt request");
    assert_eq!(request.project_id, Some(42));
    assert_eq!(request.root_project_id, Some(3));
    assert_eq!(request.secret_policy, SecretForwardingPolicy::deny_all());

    let error = prompt_service_request(Some(42), None, serde_json::json!({"project_id": 42}))
        .expect_err("project scope without root must fail");
    assert!(error.contains("缺少 root_project_id"));

    let error = prompt_service_request(None, Some(3), serde_json::json!({}))
        .expect_err("root scope without project must fail");
    assert!(error.contains("不能单独携带 root_project_id"));

    let request =
        prompt_service_request(None, None, serde_json::json!({})).expect("global prompt request");
    assert_eq!(request.project_id, None);
    assert_eq!(request.root_project_id, None);
}

#[tokio::test]
async fn rollback_project_override_runtime_restores_previous_content() {
    let engine =
        TeraPromptEngine::with_defaults(vec![prompt_entry("greeting", "默认 {{ name }}", vec![])])
            .unwrap();
    let identity = ProjectOverrideIdentity {
        project_id: 42,
        name: "greeting".to_string(),
    };
    engine
        .set_project_override(42, "greeting", "旧覆盖 {{ name }}".to_string())
        .await
        .unwrap();
    engine
        .set_project_override(42, "greeting", "新覆盖 {{ name }}".to_string())
        .await
        .unwrap();
    let message = rollback_project_override_runtime_after_persist_error(
        &engine,
        &identity,
        Some("旧覆盖 {{ name }}".to_string()),
        "提示词覆盖持久化失败: db down".to_string(),
        "已回滚运行时覆盖",
        "运行时覆盖已更新但回滚失败：",
    )
    .await;
    let mut variables = HashMap::new();
    variables.insert("name".to_string(), serde_json::json!("小明"));
    assert!(message.contains("已回滚运行时覆盖"));
    assert_eq!(
        engine.render("greeting", &variables, Some(42)).unwrap(),
        "旧覆盖 小明"
    );
}

#[tokio::test]
async fn rollback_project_override_runtime_removes_new_override_without_snapshot() {
    let engine =
        TeraPromptEngine::with_defaults(vec![prompt_entry("greeting", "默认 {{ name }}", vec![])])
            .unwrap();
    let identity = ProjectOverrideIdentity {
        project_id: 42,
        name: "greeting".to_string(),
    };
    engine
        .set_project_override(42, "greeting", "新覆盖 {{ name }}".to_string())
        .await
        .unwrap();
    let message = rollback_project_override_runtime_after_persist_error(
        &engine,
        &identity,
        None,
        "提示词覆盖持久化失败: db down".to_string(),
        "已回滚运行时覆盖",
        "运行时覆盖已更新但回滚失败：",
    )
    .await;
    let mut variables = HashMap::new();
    variables.insert("name".to_string(), serde_json::json!("小明"));
    assert!(message.contains("已回滚运行时覆盖"));
    assert_eq!(
        engine.render("greeting", &variables, Some(42)).unwrap(),
        "默认 小明"
    );
}
