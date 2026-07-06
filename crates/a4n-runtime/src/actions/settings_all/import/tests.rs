use super::super::types::{EmbeddingRow, ImageRow, PromptRow, VideoRow};
use super::*;
use a4n_engine::prompt::engine::{PromptEngine, PromptEntry};
use a4n_engine::prompt::git_store::GitPromptStore;
use a4n_engine::prompt::tera_engine::TeraPromptEngine;
use chrono::Utc;
use sea_orm::ActiveValue;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

fn full_export() -> AllConfigsExport {
    serde_json::from_value(json!({
        "version": "1.0",
        "export_time": "2026-05-27T00:00:00Z",
        "export_type": "all",
        "llm_configs": [],
        "embedding_configs": [],
        "image_configs": [],
        "video_configs": [],
        "prompts_modified": [],
        "themes": [],
        "system_settings": []
    }))
    .expect("valid export shape")
}

fn prompt_entry(name: &str, content: &str) -> PromptEntry {
    PromptEntry {
        name: name.to_string(),
        title: name.to_string(),
        content: content.to_string(),
        description: None,
        tags: Vec::new(),
        category: "writing".to_string(),
        dependencies: Vec::new(),
        is_modified: false,
        version: "1.0".to_string(),
        modified_at: Utc::now().naive_utc(),
        used_by: Vec::new(),
        path: None,
    }
}

fn prompt_engine() -> TeraPromptEngine {
    TeraPromptEngine::with_defaults(vec![prompt_entry("existing_prompt", "Hello {{ name }}")])
        .expect("prompt engine should initialize")
}

fn unique_temp_prompt_dir() -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "a4n-runtime-settings-all-prompt-{}-{suffix}",
        std::process::id()
    ))
}

fn existing_default_state(name: &str, is_default: bool) -> DefaultStateRow {
    DefaultStateRow {
        name: name.to_string(),
        is_default,
    }
}

fn embedding_row(name: &str, is_default: bool) -> EmbeddingRow {
    EmbeddingRow {
        name: name.to_string(),
        backend: "local_hash".to_string(),
        base_url: None,
        api_key_encrypted: None,
        model: "local-hash".to_string(),
        dimension: 384,
        batch_size: 16,
        local_model_path: None,
        is_default,
    }
}

fn ensure_embedding_defaults(
    existing: &[DefaultStateRow],
    incoming: &[EmbeddingRow],
    overwrite: bool,
) -> Result<(), String> {
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
) -> Result<(), String> {
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
fn prompt_import_prevalidation_rejects_missing_prompt_before_db_import() {
    let mut incoming = full_export();
    incoming.prompts_modified.push(PromptRow {
        name: "missing_prompt".to_string(),
        content: "new content".to_string(),
        title: None,
        description: None,
    });

    let err = validate_prompt_import_targets(&incoming, &prompt_engine())
        .expect_err("missing prompt should fail before import");

    assert!(err.contains("提示词 missing_prompt 导入失败：注册表中不存在"));
}

#[test]
fn prompt_import_prevalidation_rejects_invalid_template_before_db_import() {
    let mut incoming = full_export();
    incoming.prompts_modified.push(PromptRow {
        name: "existing_prompt".to_string(),
        content: "broken {{ name ".to_string(),
        title: None,
        description: None,
    });

    let err = validate_prompt_import_targets(&incoming, &prompt_engine())
        .expect_err("invalid template should fail before import");

    assert!(err.contains("提示词 existing_prompt 模板编译失败"));
}

#[test]
fn prompt_import_update_failure_returns_error_without_mutating_runtime_prompt() {
    let git_dir = unique_temp_prompt_dir();
    let store = GitPromptStore::open_or_init(git_dir.clone()).expect("git store should initialize");
    fs::remove_dir_all(&git_dir).expect("removing git store should make commits fail");
    let engine =
        TeraPromptEngine::with_defaults(vec![prompt_entry("existing_prompt", "old content")])
            .expect("prompt engine should initialize")
            .with_git_store(Arc::new(store))
            .expect("git-backed prompt engine should initialize");

    let rows = vec![PromptRow {
        name: "existing_prompt".to_string(),
        content: "new content".to_string(),
        title: None,
        description: None,
    }];

    let err = tokio::runtime::Runtime::new()
        .expect("tokio runtime should initialize")
        .block_on(import_prompt_rows(&engine, rows))
        .expect_err("prompt update failure should abort import");

    assert!(err.contains("提示词批量导入失败"));
    assert!(err.contains("IO 错误"));
    let entry = engine
        .get("existing_prompt")
        .expect("prompt should remain readable");
    assert_eq!(entry.content, "old content");
    assert!(!entry.is_modified);
}

#[test]
fn prompt_import_batch_validation_failure_does_not_mutate_previous_prompt() {
    let engine = TeraPromptEngine::with_defaults(vec![
        prompt_entry("first_prompt", "first old"),
        prompt_entry("second_prompt", "second old"),
    ])
    .expect("prompt engine should initialize");

    let rows = vec![
        PromptRow {
            name: "first_prompt".to_string(),
            content: "first new".to_string(),
            title: None,
            description: None,
        },
        PromptRow {
            name: "second_prompt".to_string(),
            content: "broken {{ name ".to_string(),
            title: None,
            description: None,
        },
    ];

    let err = tokio::runtime::Runtime::new()
        .expect("tokio runtime should initialize")
        .block_on(import_prompt_rows(&engine, rows))
        .expect_err("batch validation failure should abort import");

    assert!(err.contains("提示词批量导入失败"));
    assert!(err.contains("模板编译失败"));
    let first = engine
        .get("first_prompt")
        .expect("first prompt should remain readable");
    let second = engine
        .get("second_prompt")
        .expect("second prompt should remain readable");
    assert_eq!(first.content, "first old");
    assert!(!first.is_modified);
    assert_eq!(second.content, "second old");
    assert!(!second.is_modified);
}

#[test]
fn prompt_import_rollback_restores_snapshots_after_database_failure() {
    let engine = TeraPromptEngine::with_defaults(vec![
        prompt_entry("first_prompt", "first old"),
        prompt_entry("second_prompt", "second old"),
    ])
    .expect("prompt engine should initialize");

    let rows = vec![
        PromptRow {
            name: "first_prompt".to_string(),
            content: "first new".to_string(),
            title: None,
            description: None,
        },
        PromptRow {
            name: "second_prompt".to_string(),
            content: "second new".to_string(),
            title: None,
            description: None,
        },
    ];

    let snapshots =
        prompt_import_snapshots(&engine, &rows).expect("snapshots should read current prompts");
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
    runtime
        .block_on(import_prompt_rows(&engine, rows))
        .expect("prompt import should succeed before database phase");

    assert_eq!(
        engine
            .get("first_prompt")
            .expect("first prompt should be readable")
            .content,
        "first new"
    );
    assert_eq!(
        engine
            .get("second_prompt")
            .expect("second prompt should be readable")
            .content,
        "second new"
    );

    let err = runtime.block_on(rollback_prompt_import_after_db_error(
        &engine,
        snapshots,
        "数据库写入失败".to_string(),
    ));

    assert!(err.contains("数据库写入失败"));
    assert!(err.contains("已回滚本次提示词导入"));
    let first = engine
        .get("first_prompt")
        .expect("first prompt should remain readable");
    let second = engine
        .get("second_prompt")
        .expect("second prompt should remain readable");
    assert_eq!(first.content, "first old");
    assert_eq!(second.content, "second old");
}

#[test]
fn embedding_import_rejects_overwriting_only_default_to_non_default() {
    let existing = vec![existing_default_state("current", true)];
    let incoming = vec![embedding_row("current", false)];

    let err = ensure_embedding_defaults(&existing, &incoming, true)
        .expect_err("overwriting the only default to false should fail");

    assert!(err.contains("全量导入后必须保留一个默认 Embedding 配置"));
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

    assert!(err.contains("全量导入后只能保留一个默认 Embedding 配置，当前为 2 个"));

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
            err.contains(&format!("全量导入后必须保留一个{kind}")),
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

#[test]
fn image_import_active_model_preserves_provider_options() {
    let now = Utc::now().naive_utc();
    let row = ImageRow {
        name: "cliproxy image".to_string(),
        backend: "cliproxy-image".to_string(),
        base_url: Some("https://image.example/v1".to_string()),
        api_key_encrypted: Some("secret".to_string()),
        model: "grok-image".to_string(),
        max_concurrent: 1,
        default_width: 1024,
        default_height: 1024,
        provider_options_json: Some(json!({
            "grok_reference_mode": "first_frame",
        })),
        is_default: true,
    };

    let expected = ActiveValue::Set(Some(json!({
        "grok_reference_mode": "first_frame",
    })));
    assert_eq!(
        image_active_model_for_insert(row.clone(), now).provider_options_json,
        expected
    );
    assert_eq!(
        image_active_model_for_update(Default::default(), row, now).provider_options_json,
        expected
    );
}

#[test]
fn video_import_active_model_preserves_provider_options() {
    let now = Utc::now().naive_utc();
    let row = VideoRow {
        name: "cliproxy video".to_string(),
        backend: "cliproxy-openai-video".to_string(),
        base_url: Some("https://video.example/v1".to_string()),
        api_key_encrypted: Some("secret".to_string()),
        model: "grok-video".to_string(),
        max_concurrent: 1,
        default_width: 1280,
        default_height: 720,
        default_duration_secs: 5,
        min_duration_secs: 1,
        max_duration_secs: 15,
        duration_mode: "range".to_string(),
        supported_durations_json: None,
        default_fps: 24,
        provider_options_json: Some(json!({
            "grok_reference_mode": "reference",
        })),
        is_default: true,
    };

    let expected = ActiveValue::Set(Some(json!({
        "grok_reference_mode": "reference",
    })));
    assert_eq!(
        video_active_model_for_insert(row.clone(), now).provider_options_json,
        expected
    );
    assert_eq!(
        video_active_model_for_update(Default::default(), row, now).provider_options_json,
        expected
    );
}
