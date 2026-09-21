use std::collections::HashSet;
use std::error::Error;

use a4n_db_models::models::ai::system_settings;
use chrono::Utc;
use sea_orm::DbErr;
use serde_json::Value;

use super::cleanup::{retired_project_override_record_ids, validate_project_override_record_keys};
use super::hydration::{
    project_override_hydration_candidates_from_records, project_override_summaries_from_records,
};
use super::*;

#[test]
fn prompt_override_database_error_keeps_context_and_source() {
    let error = PromptOverrideDbError::Database {
        operation: "读取项目提示词覆盖失败",
        source: DbErr::Custom("query failed".to_string()),
    };

    assert_eq!(
        error.to_string(),
        "读取项目提示词覆盖失败: Custom Error: query failed"
    );
    assert!(error.source().is_some());
}

#[test]
fn prompt_override_project_error_keeps_typed_source() {
    let error = PromptOverrideDbError::from(crate::project::ProjectMutationError::NotFound(
        "项目不存在: 7".to_string(),
    ));

    assert_eq!(error.to_string(), "项目不存在: 7");
    assert!(error.source().is_some());
}

fn setting_record(id: i64, key: &str, value: Value) -> system_settings::Model {
    system_settings::Model {
        id,
        key: key.to_string(),
        value,
        category: system_settings::PROMPT_OVERRIDE_CATEGORY.to_string(),
        description: None,
        updated_at: Utc::now().naive_utc(),
    }
}

#[test]
fn parse_project_override_key_accepts_prompt_names_with_underscores() {
    let identity = parse_project_override_key("prompt_override_42_foo_bar").unwrap();

    assert_eq!(identity.project_id, 42);
    assert_eq!(identity.name, "foo_bar");
}

#[test]
fn parse_project_override_key_rejects_unrelated_settings() {
    let error = parse_project_override_key("llm_config").unwrap_err();
    assert!(error.to_string().contains("不是提示词覆盖 key"));
}

#[test]
fn parse_project_override_key_rejects_blank_name() {
    let err = parse_project_override_key("prompt_override_42_")
        .expect_err("blank persisted override name should fail");

    assert!(err.to_string().contains("提示词覆盖 key name 不能为空"));
}

#[test]
fn parse_project_override_key_rejects_name_with_boundary_whitespace() {
    let err = parse_project_override_key("prompt_override_42_ prompt")
        .expect_err("persisted override key names must not be guessed by trimming");

    assert!(
        err.to_string()
            .contains("提示词覆盖 key name 不能包含首尾空白")
    );
}

#[test]
fn parse_project_override_key_rejects_control_character_name() {
    let err = parse_project_override_key("prompt_override_42_bad\nname")
        .expect_err("control characters are not valid prompt override names");

    assert!(
        err.to_string()
            .contains("提示词覆盖 key name 不能包含控制字符")
    );
}

#[test]
fn normalize_project_override_identity_trims_user_input_name() {
    let identity = normalize_project_override_identity(42, " prompt ").unwrap();

    assert_eq!(
        identity,
        ProjectOverrideIdentity {
            project_id: 42,
            name: "prompt".to_string(),
        }
    );
}

#[test]
fn hydrate_project_overrides_plan_accepts_valid_overrides() {
    let records = vec![
        setting_record(
            1,
            "prompt_override_42_writing_prompt",
            serde_json::json!("Override"),
        ),
        setting_record(
            2,
            "prompt_override_7_prompt_with_underscores",
            serde_json::json!("Another override"),
        ),
    ];

    let plan = project_override_hydration_candidates_from_records(&records).unwrap();

    assert_eq!(
        plan,
        vec![
            ProjectOverrideHydration {
                identity: ProjectOverrideIdentity {
                    project_id: 42,
                    name: "writing_prompt".to_string(),
                },
                content: "Override".to_string(),
            },
            ProjectOverrideHydration {
                identity: ProjectOverrideIdentity {
                    project_id: 7,
                    name: "prompt_with_underscores".to_string(),
                },
                content: "Another override".to_string(),
            },
        ]
    );
}

#[test]
fn hydrate_project_overrides_plan_rejects_invalid_override_key() {
    let records = vec![
        setting_record(1, "prompt_override_42_valid", serde_json::json!("Override")),
        setting_record(2, "prompt_override_42_", serde_json::json!("Bad key")),
    ];

    let err = project_override_hydration_candidates_from_records(&records)
        .expect_err("invalid persisted override key should fail before engine apply");

    assert!(err.to_string().contains("提示词覆盖 key name 不能为空"));
}

#[test]
fn hydrate_project_overrides_plan_rejects_bad_override_value() {
    let records = vec![
        setting_record(
            1,
            "prompt_override_42_non_string",
            serde_json::json!({"content": "not a string"}),
        ),
        setting_record(2, "prompt_override_42_blank", serde_json::json!("   ")),
    ];

    let err = project_override_hydration_candidates_from_records(&records)
        .expect_err("non-string persisted override value should fail before engine apply");

    assert!(
        err.to_string()
            .contains("提示词覆盖记录 value 必须是字符串")
    );

    let err = project_override_hydration_candidates_from_records(&records[1..])
        .expect_err("blank persisted override value should fail before engine apply");

    assert!(err.to_string().contains("提示词覆盖内容不能为空"));
}

#[test]
fn project_override_list_filters_records_for_retired_prompt_names() {
    let mut active_prompt_names = HashSet::new();
    active_prompt_names.insert("active_prompt".to_string());
    let records = vec![
        setting_record(
            1,
            &system_settings::prompt_override_key(42, "active_prompt"),
            serde_json::json!("active override"),
        ),
        setting_record(
            2,
            &system_settings::prompt_override_key(42, "retired_prompt"),
            serde_json::json!("retired override"),
        ),
        setting_record(
            3,
            &system_settings::prompt_override_key(7, "active_prompt"),
            serde_json::json!("other project"),
        ),
    ];

    let overrides =
        project_override_summaries_from_records(42, records, &active_prompt_names).unwrap();

    assert_eq!(overrides.len(), 1);
    assert_eq!(overrides[0].prompt_name, "active_prompt");
    assert_eq!(overrides[0].content, "active override");
}

#[test]
fn project_override_list_rejects_invalid_override_key() {
    let mut active_prompt_names = HashSet::new();
    active_prompt_names.insert("active_prompt".to_string());
    let records = vec![setting_record(
        1,
        system_settings::PROMPT_OVERRIDE_KEY_PREFIX,
        serde_json::json!("bad"),
    )];

    let err = project_override_summaries_from_records(42, records, &active_prompt_names)
        .expect_err("invalid prompt override key should not be hidden by list filtering");

    assert!(err.to_string().contains("提示词覆盖 key 格式无效"));
}

#[test]
fn project_override_list_rejects_non_string_content_before_active_filter() {
    let mut active_prompt_names = HashSet::new();
    active_prompt_names.insert("active_prompt".to_string());
    let records = vec![setting_record(
        1,
        &system_settings::prompt_override_key(42, "retired_prompt"),
        serde_json::json!({"content": "bad"}),
    )];

    let err = project_override_summaries_from_records(42, records, &active_prompt_names)
        .expect_err("invalid persisted override content should be explicit");

    assert!(
        err.to_string()
            .contains("提示词覆盖记录 value 必须是字符串")
    );
}

#[test]
fn retired_project_override_cleanup_matches_parsed_prompt_name() {
    let mut retired_prompt_names = HashSet::new();
    retired_prompt_names.insert("bar".to_string());
    let records = vec![
        setting_record(
            1,
            &system_settings::prompt_override_key(42, "bar"),
            serde_json::json!("retired"),
        ),
        setting_record(
            2,
            &system_settings::prompt_override_key(42, "foo_bar"),
            serde_json::json!("still active"),
        ),
    ];

    let ids = retired_project_override_record_ids(&records, &retired_prompt_names).unwrap();

    assert_eq!(ids, vec![1]);
}

#[test]
fn retired_project_override_cleanup_rejects_invalid_override_key() {
    let mut retired_prompt_names = HashSet::new();
    retired_prompt_names.insert("bar".to_string());
    let records = vec![setting_record(
        1,
        system_settings::PROMPT_OVERRIDE_KEY_PREFIX,
        serde_json::json!("bad"),
    )];

    let err = retired_project_override_record_ids(&records, &retired_prompt_names)
        .expect_err("invalid prompt override key should be explicit");

    assert!(err.to_string().contains("提示词覆盖 key 格式无效"));
}

#[test]
fn project_override_key_preflight_accepts_valid_override() {
    let records = vec![setting_record(
        1,
        &system_settings::prompt_override_key(42, "active_prompt"),
        serde_json::json!("override"),
    )];

    validate_project_override_record_keys(&records).unwrap();
}

#[test]
fn project_override_key_preflight_rejects_invalid_override_key_before_reload() {
    let records = vec![setting_record(
        1,
        system_settings::PROMPT_OVERRIDE_KEY_PREFIX,
        serde_json::json!("bad"),
    )];

    let err = validate_project_override_record_keys(&records)
        .expect_err("load_defaults preflight should catch invalid persisted override keys");

    assert!(err.to_string().contains("提示词覆盖 key 格式无效"));
}
