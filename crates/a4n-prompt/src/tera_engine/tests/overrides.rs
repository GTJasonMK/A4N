use std::collections::HashMap;

use super::super::TeraPromptEngine;
use super::support::make_entry;

#[test]
fn test_simple_render() {
    let engine = TeraPromptEngine::with_defaults(vec![make_entry(
        "greeting",
        "你好，{{ name }}！欢迎来到 {{ place }}。",
    )])
    .unwrap();

    let mut vars = HashMap::new();
    vars.insert("name".into(), serde_json::json!("小明"));
    vars.insert("place".into(), serde_json::json!("A4N"));

    let result = engine.render("greeting", &vars, None).unwrap();
    assert_eq!(result, "你好，小明！欢迎来到 A4N。");
}

#[test]
fn test_project_override() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("greeting", "你好，{{ name }}！")])
            .unwrap();

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(engine.set_project_override(42, "greeting", "欢迎回来，{{ name }}！".into()))
        .unwrap();

    let mut vars = HashMap::new();
    vars.insert("name".into(), serde_json::json!("小明"));

    let r1 = engine.render("greeting", &vars, None).unwrap();
    assert_eq!(r1, "你好，小明！");

    let r2 = engine.render("greeting", &vars, Some(42)).unwrap();
    assert_eq!(r2, "欢迎回来，小明！");
}

#[test]
fn project_override_trims_name_for_set_and_remove() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("greeting", "你好，{{ name }}！")])
            .unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime
        .block_on(engine.set_project_override(42, " greeting ", "欢迎回来，{{ name }}！".into()))
        .unwrap();

    let mut vars = HashMap::new();
    vars.insert("name".into(), serde_json::json!("小明"));
    assert_eq!(
        engine.render("greeting", &vars, Some(42)).unwrap(),
        "欢迎回来，小明！"
    );

    runtime
        .block_on(engine.remove_project_override(42, " greeting "))
        .unwrap();

    assert_eq!(
        engine.render("greeting", &vars, Some(42)).unwrap(),
        "你好，小明！"
    );
}

#[test]
fn project_override_rejects_invalid_identity_content_and_unknown_prompt() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("greeting", "你好，{{ name }}！")])
            .unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let bad_project = runtime
        .block_on(engine.set_project_override(0, "greeting", "覆盖".into()))
        .expect_err("zero project id should fail");
    let blank_name = runtime
        .block_on(engine.set_project_override(42, " ", "覆盖".into()))
        .expect_err("blank name should fail");
    let blank_content = runtime
        .block_on(engine.set_project_override(42, "greeting", " \n ".into()))
        .expect_err("blank content should fail");
    let unknown = runtime
        .block_on(engine.set_project_override(42, "missing", "覆盖".into()))
        .expect_err("unknown prompt should fail");

    assert!(bad_project.to_string().contains("项目 ID 必须大于 0"));
    assert!(blank_name.to_string().contains("name 不能为空"));
    assert!(blank_content.to_string().contains("内容不能为空"));
    assert!(unknown.to_string().contains("提示词不存在"));
    assert!(engine.render("missing", &HashMap::new(), Some(42)).is_err());
}

#[test]
fn project_override_rejects_invalid_template_without_mutating_runtime_override() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("greeting", "你好，{{ name }}！")])
            .unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut vars = HashMap::new();
    vars.insert("name".into(), serde_json::json!("小明"));

    let err = runtime
        .block_on(engine.set_project_override(42, "greeting", "坏模板 {{ name".into()))
        .expect_err("invalid override template should fail before insert");

    assert!(err.to_string().contains("模板编译失败"));
    assert_eq!(
        engine.render("greeting", &vars, Some(42)).unwrap(),
        "你好，小明！"
    );
}

#[test]
fn hydrate_project_overrides_applies_batch_after_full_validation() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("greeting", "你好，{{ name }}！")])
            .unwrap();

    let loaded = engine
        .hydrate_project_overrides(vec![
            (42, "greeting".to_string(), "项目42 {{ name }}".to_string()),
            (7, "greeting".to_string(), "项目7 {{ name }}".to_string()),
        ])
        .unwrap();

    let mut vars = HashMap::new();
    vars.insert("name".into(), serde_json::json!("小明"));
    assert_eq!(loaded, 2);
    assert_eq!(
        engine.render("greeting", &vars, Some(42)).unwrap(),
        "项目42 小明"
    );
    assert_eq!(
        engine.render("greeting", &vars, Some(7)).unwrap(),
        "项目7 小明"
    );
}

#[test]
fn hydrate_project_overrides_rejects_unknown_prompt_without_partial_apply() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("greeting", "你好，{{ name }}！")])
            .unwrap();

    let err = engine
        .hydrate_project_overrides(vec![
            (42, "greeting".to_string(), "覆盖 {{ name }}".to_string()),
            (42, "missing".to_string(), "缺失提示词覆盖".to_string()),
        ])
        .expect_err("unknown prompt in later record should reject the whole batch");

    let mut vars = HashMap::new();
    vars.insert("name".into(), serde_json::json!("小明"));
    assert!(err.to_string().contains("提示词不存在"));
    assert_eq!(
        engine.render("greeting", &vars, Some(42)).unwrap(),
        "你好，小明！"
    );
}

#[test]
fn hydrate_project_overrides_rejects_bad_template_without_partial_apply() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("greeting", "你好，{{ name }}！")])
            .unwrap();

    let err = engine
        .hydrate_project_overrides(vec![
            (42, "greeting".to_string(), "覆盖 {{ name }}".to_string()),
            (7, "greeting".to_string(), "坏模板 {{ name".to_string()),
        ])
        .expect_err("bad template in later record should reject the whole batch");

    let mut vars = HashMap::new();
    vars.insert("name".into(), serde_json::json!("小明"));
    assert!(err.to_string().contains("模板编译失败"));
    assert_eq!(
        engine.render("greeting", &vars, Some(42)).unwrap(),
        "你好，小明！"
    );
}

#[test]
fn render_ab_compares_current_prompt_with_original_prompt() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("greeting", "原版 {{ name }}")]).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime
        .block_on(engine.update("greeting", "新版 {{ name }}".into(), "author"))
        .unwrap();
    let mut vars = HashMap::new();
    vars.insert("name".into(), serde_json::json!("小明"));

    let (version_a, version_b) = engine.render_ab("greeting", &vars, None).unwrap();

    assert_eq!(version_a, "新版 小明");
    assert_eq!(version_b, "原版 小明");
}

#[test]
fn render_rejects_non_positive_project_id() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("greeting", "你好，{{ name }}！")])
            .unwrap();
    let mut vars = HashMap::new();
    vars.insert("name".into(), serde_json::json!("小明"));

    let zero = engine
        .render("greeting", &vars, Some(0))
        .expect_err("zero project id should fail");
    let negative = engine
        .render_ab("greeting", &vars, Some(-1))
        .expect_err("negative project id should fail");

    assert!(zero.to_string().contains("项目 ID 必须大于 0"));
    assert!(negative.to_string().contains("项目 ID 必须大于 0"));
    assert_eq!(
        engine.render("greeting", &vars, None).unwrap(),
        "你好，小明！"
    );
}
