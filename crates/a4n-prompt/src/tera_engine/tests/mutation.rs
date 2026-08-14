use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use super::super::TeraPromptEngine;
use super::support::{MemoryPromptVersionStore, make_entry, write_test_registry};

#[test]
fn test_reset_to_original() {
    let tmp = tempfile::tempdir().unwrap();
    let md = "---\nname: test\n---\n原始内容";
    fs::write(tmp.path().join("test.md"), md).unwrap();
    write_test_registry(
        tmp.path(),
        "  test:\n    path: \"test.md\"\n    title: \"测试\"\n    category: \"writing\"\n    tags: [\"test\", \"writing\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"测试模板\"\n",
    );

    let engine = TeraPromptEngine::from_directory(tmp.path()).unwrap();

    // 修改
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(engine.update("test", "修改后的内容".into(), "author"))
        .unwrap();

    let modified = engine.get("test").unwrap();
    assert!(modified.is_modified);
    assert_eq!(modified.content, "修改后的内容");

    // 重置
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(engine.reset("test"))
        .unwrap();

    let reset_entry = engine.get("test").unwrap();
    assert!(!reset_entry.is_modified);
    assert_eq!(reset_entry.content, "原始内容");
}
#[test]
fn update_with_identical_content_is_noop() {
    let engine = TeraPromptEngine::with_defaults(vec![make_entry("greeting", "旧内容")]).unwrap();

    let updated = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(engine.update("greeting", "旧内容".into(), "author"))
        .unwrap();

    assert_eq!(updated.version, "1.0");
    assert!(!updated.is_modified);
    assert_eq!(updated.content, "旧内容");
    let history = engine.version_history("greeting").unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].message, "原始版本");
    assert_eq!(
        engine.render("greeting", &HashMap::new(), None).unwrap(),
        "旧内容"
    );
}
#[test]
fn update_rejects_blank_content_and_blank_author_without_mutating_runtime_prompt() {
    let engine = TeraPromptEngine::with_defaults(vec![make_entry("greeting", "旧内容")]).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let blank_content = runtime
        .block_on(engine.update("greeting", " \n ".into(), "author"))
        .expect_err("blank content should fail");
    let blank_author = runtime
        .block_on(engine.update("greeting", "新内容".into(), " \t "))
        .expect_err("blank author should fail for real updates");

    assert!(blank_content.to_string().contains("提示词内容不能为空"));
    assert!(blank_author.to_string().contains("提示词更新作者不能为空"));
    let entry = engine.get("greeting").unwrap();
    assert_eq!(entry.version, "1.0");
    assert!(!entry.is_modified);
    assert_eq!(entry.content, "旧内容");
    let history = engine.version_history("greeting").unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].message, "原始版本");
    assert_eq!(
        engine.render("greeting", &HashMap::new(), None).unwrap(),
        "旧内容"
    );
}

#[test]
fn update_with_git_failure_does_not_mutate_runtime_prompt() {
    let store = Arc::new(MemoryPromptVersionStore::default());
    store.fail_commits();
    let engine = TeraPromptEngine::with_defaults(vec![make_entry("greeting", "旧内容")])
        .unwrap()
        .with_git_store(store)
        .unwrap();

    let err = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(engine.update("greeting", "新内容".into(), "author"))
        .expect_err("git commit failure should be reported");

    assert!(err.to_string().contains("IO 错误"));
    let entry = engine.get("greeting").unwrap();
    assert!(!entry.is_modified);
    assert_eq!(entry.content, "旧内容");

    let rendered = engine.render("greeting", &HashMap::new(), None).unwrap();
    assert_eq!(rendered, "旧内容");

    let history = engine.version_history("greeting").unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].message, "原始版本");
    assert_eq!(history[0].content_snapshot, "旧内容");
}

#[test]
fn with_git_store_rehydrates_latest_committed_prompt_content() {
    let store = Arc::new(MemoryPromptVersionStore::default());
    let engine = TeraPromptEngine::with_defaults(vec![make_entry("greeting", "旧内容")])
        .unwrap()
        .with_git_store(store.clone())
        .unwrap();

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(engine.update("greeting", "新内容 {{ name }}".into(), "author"))
        .unwrap();

    let restarted = TeraPromptEngine::with_defaults(vec![make_entry("greeting", "旧内容")])
        .unwrap()
        .with_git_store(store)
        .unwrap();
    let entry = restarted.get("greeting").unwrap();

    assert_eq!(entry.content, "新内容 {{ name }}");
    assert!(entry.is_modified);
    assert_eq!(entry.version, "1.0.1");

    let mut vars = HashMap::new();
    vars.insert("name".to_string(), serde_json::json!("A4N"));
    assert_eq!(
        restarted.render("greeting", &vars, None).unwrap(),
        "新内容 A4N"
    );

    let history = restarted.version_history("greeting").unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].version, "1.0");
    assert_eq!(history[0].content_snapshot, "旧内容");
    assert_eq!(history[1].version, "1.0.1");
    assert_eq!(history[1].content_snapshot, "新内容 {{ name }}");
}

#[test]
fn reset_with_git_store_survives_restart() {
    let store = Arc::new(MemoryPromptVersionStore::default());
    let engine = TeraPromptEngine::with_defaults(vec![make_entry("greeting", "旧内容")])
        .unwrap()
        .with_git_store(store.clone())
        .unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime
        .block_on(engine.update("greeting", "新内容 {{ name }}".into(), "author"))
        .unwrap();
    runtime.block_on(engine.reset("greeting")).unwrap();

    let reset_entry = engine.get("greeting").unwrap();
    assert_eq!(reset_entry.content, "旧内容");
    assert!(!reset_entry.is_modified);
    assert_eq!(reset_entry.version, "1.0.2");

    let restarted = TeraPromptEngine::with_defaults(vec![make_entry("greeting", "旧内容")])
        .unwrap()
        .with_git_store(store)
        .unwrap();
    let restarted_entry = restarted.get("greeting").unwrap();

    assert_eq!(restarted_entry.content, "旧内容");
    assert!(!restarted_entry.is_modified);
    assert_eq!(restarted_entry.version, "1.0.2");
    assert_eq!(
        restarted.render("greeting", &HashMap::new(), None).unwrap(),
        "旧内容"
    );

    let history = restarted.version_history("greeting").unwrap();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].content_snapshot, "旧内容");
    assert_eq!(history[1].content_snapshot, "新内容 {{ name }}");
    assert_eq!(history[2].content_snapshot, "旧内容");
}

#[test]
fn reset_with_git_failure_does_not_mutate_runtime_prompt() {
    let store = Arc::new(MemoryPromptVersionStore::default());
    let engine = TeraPromptEngine::with_defaults(vec![make_entry("greeting", "旧内容")])
        .unwrap()
        .with_git_store(store.clone())
        .unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime
        .block_on(engine.update("greeting", "新内容".into(), "author"))
        .unwrap();
    store.fail_commits();

    let err = runtime
        .block_on(engine.reset("greeting"))
        .expect_err("git reset commit failure should be reported");

    assert!(err.to_string().contains("IO 错误"));
    let entry = engine.get("greeting").unwrap();
    assert_eq!(entry.content, "新内容");
    assert!(entry.is_modified);
    assert_eq!(entry.version, "1.0.1");
    assert_eq!(
        engine.render("greeting", &HashMap::new(), None).unwrap(),
        "新内容"
    );
}
