use super::types::{SettingsAllConfigImportError, SettingsAllEmbeddingConfigRow};
use super::validation::{DefaultImportPolicy, DefaultStateRow, ensure_single_default_after_import};
use sea_orm::DbErr;
use std::error::Error;

#[test]
fn settings_all_config_import_database_error_keeps_context_and_source() {
    let error = SettingsAllConfigImportError::Database {
        operation: "读取现有 LLM 配置失败",
        source: DbErr::Custom("query failed".to_string()),
    };

    assert_eq!(
        error.to_string(),
        "读取现有 LLM 配置失败: Custom Error: query failed"
    );
    assert!(error.source().is_some());
}

#[test]
fn settings_all_config_import_rule_error_is_invalid_argument() {
    let error = SettingsAllConfigImportError::InvalidArgument("必须保留默认配置".to_string());

    assert_eq!(error.to_string(), "必须保留默认配置");
    assert!(error.source().is_none());
}

fn existing_default_state(name: &str, is_default: bool) -> DefaultStateRow {
    DefaultStateRow {
        name: name.to_string(),
        is_default,
    }
}

fn embedding_row(name: &str, is_default: bool) -> SettingsAllEmbeddingConfigRow {
    SettingsAllEmbeddingConfigRow {
        name: name.to_string(),
        backend: "local_hash".to_string(),
        base_url: None,
        api_key_encrypted: None,
        model: "local-hash".to_string(),
        dimension: 384,
        batch_size: 16,
        local_model_path: None,
        pricing_json: serde_json::json!({}),
        is_default,
    }
}

fn ensure_embedding_defaults(
    existing: &[DefaultStateRow],
    incoming: &[SettingsAllEmbeddingConfigRow],
    overwrite: bool,
) -> Result<(), SettingsAllConfigImportError> {
    ensure_single_default_after_import(
        "默认 Embedding 配置",
        existing,
        incoming,
        overwrite,
        DefaultImportPolicy::Required,
        |row| row.name.as_str(),
        |row| row.is_default,
    )
}

fn ensure_optional_defaults(
    kind: &str,
    existing: &[DefaultStateRow],
    incoming: &[DefaultStateRow],
    overwrite: bool,
) -> Result<(), SettingsAllConfigImportError> {
    ensure_single_default_after_import(
        kind,
        existing,
        incoming,
        overwrite,
        DefaultImportPolicy::PreserveIfConfigured,
        |row| row.name.as_str(),
        |row| row.is_default,
    )
}

#[test]
fn embedding_import_rejects_overwriting_only_default_to_non_default() {
    let existing = vec![existing_default_state("current", true)];
    let incoming = vec![embedding_row("current", false)];

    let err = ensure_embedding_defaults(&existing, &incoming, true)
        .expect_err("overwriting the only default to false should fail");

    assert!(
        err.to_string()
            .contains("全量导入后必须保留一个默认 Embedding 配置")
    );
}

#[test]
fn embedding_import_keeps_existing_default_when_non_default_is_skipped() {
    let existing = vec![existing_default_state("current", true)];
    let incoming = vec![embedding_row("current", false)];

    ensure_embedding_defaults(&existing, &incoming, false)
        .expect("skipped non-default row should not clear existing default");
}

#[test]
fn embedding_import_accepts_new_default() {
    let existing = vec![existing_default_state("current", true)];
    let incoming = vec![embedding_row("replacement", true)];

    ensure_embedding_defaults(&existing, &incoming, true)
        .expect("new default row should leave exactly one default");
}

#[test]
fn embedding_import_rejects_existing_multiple_defaults_unless_import_fixes_it() {
    let existing = vec![
        existing_default_state("first", true),
        existing_default_state("second", true),
    ];

    let err = ensure_embedding_defaults(&existing, &[], true)
        .expect_err("existing duplicated defaults should not be accepted silently");

    assert!(
        err.to_string()
            .contains("全量导入后只能保留一个默认 Embedding 配置，当前为 2 个")
    );

    let incoming = vec![embedding_row("third", true)];
    ensure_embedding_defaults(&existing, &incoming, true)
        .expect("incoming default should normalize existing duplicated defaults");
}

#[test]
fn optional_config_import_rejects_clearing_existing_defaults() {
    for kind in ["默认 LLM 配置", "默认图片配置", "默认视频配置"] {
        let existing = vec![existing_default_state("current", true)];
        let incoming = vec![existing_default_state("current", false)];

        let err = ensure_optional_defaults(kind, &existing, &incoming, true)
            .expect_err("overwriting an existing optional default to false should fail");

        assert!(
            err.to_string()
                .contains(&format!("全量导入后必须保留一个{kind}")),
            "unexpected error for {kind}: {err}"
        );
    }
}

#[test]
fn optional_config_import_allows_still_unconfigured_state() {
    ensure_optional_defaults("默认图片配置", &[], &[], true)
        .expect("optional config type may remain unconfigured");
    ensure_optional_defaults(
        "默认视频配置",
        &[],
        &[existing_default_state("candidate", false)],
        true,
    )
    .expect("importing non-default optional config may remain unconfigured");
}
