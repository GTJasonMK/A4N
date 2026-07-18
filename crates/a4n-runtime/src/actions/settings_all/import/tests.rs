use super::super::types::PromptRow;
use super::*;
use a4n_prompt::engine::{PromptEngine, PromptEntry};
use a4n_prompt::tera_engine::TeraPromptEngine;
#[cfg(feature = "embedded-prompt-git")]
use a4n_prompt_git::GitPromptStore;
use chrono::Utc;
use serde_json::json;
#[cfg(feature = "embedded-prompt-git")]
use std::fs;
#[cfg(feature = "embedded-prompt-git")]
use std::path::PathBuf;
#[cfg(feature = "embedded-prompt-git")]
use std::sync::Arc;
#[cfg(feature = "embedded-prompt-git")]
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

#[cfg(feature = "embedded-prompt-git")]
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

#[test]
fn prompt_import_prevalidation_rejects_missing_prompt_before_db_import() {
    let mut incoming = full_export();
    incoming.prompts_modified.push(PromptRow {
        name: "missing_prompt".to_string(),
        content: "new content".to_string(),
        title: None,
        description: None,
    });

    let err = validate_prompt_import_targets_for_engine(&incoming, &prompt_engine())
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

    let err = validate_prompt_import_targets_for_engine(&incoming, &prompt_engine())
        .expect_err("invalid template should fail before import");

    assert!(err.contains("提示词 existing_prompt 模板编译失败"));
}

#[cfg(feature = "embedded-prompt-git")]
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
        .block_on(import_prompt_rows_for_engine(&engine, rows))
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
        .block_on(import_prompt_rows_for_engine(&engine, rows))
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

    let snapshots = prompt_import_snapshots_for_engine(&engine, &rows)
        .expect("snapshots should read current prompts");
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should initialize");
    runtime
        .block_on(import_prompt_rows_for_engine(&engine, rows))
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

    let err = runtime.block_on(rollback_prompt_import_after_db_error_for_engine(
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
