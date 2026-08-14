use std::collections::{HashMap, HashSet};
use std::fs;

use a4n_core::error::AppError;

use super::super::TeraPromptEngine;
use super::support::{make_entry, write_test_registry};

#[test]
fn test_from_directory() {
    let tmp = tempfile::tempdir().unwrap();

    let md1 = "---\nname: greeting\n---\n你好，{{ name }}！";
    let md2 = "---\nname: outline_prompt\n---\n为{{ title }}生成大纲。";

    fs::write(tmp.path().join("greeting.md"), md1).unwrap();
    fs::write(tmp.path().join("outline.md"), md2).unwrap();
    write_test_registry(
        tmp.path(),
        "  greeting:\n    path: \"greeting.md\"\n    title: \"问候\"\n    category: \"writing\"\n    tags: [\"test\", \"writing\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"问候模板\"\n  outline_prompt:\n    path: \"outline.md\"\n    title: \"大纲生成\"\n    category: \"outline\"\n    tags: [\"test\", \"outline\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"大纲生成模板\"\n",
    );
    // 非 md 文件应被忽略
    fs::write(tmp.path().join("readme.txt"), "not a prompt").unwrap();

    let engine = TeraPromptEngine::from_directory(tmp.path()).unwrap();
    let all = engine.list_all();
    assert_eq!(all.len(), 2);

    let name_set: HashSet<String> = all.iter().map(|e| e.name.clone()).collect();
    assert!(name_set.contains("greeting"));
    assert!(name_set.contains("outline_prompt"));

    // 验证可以渲染
    let mut vars = HashMap::new();
    vars.insert("name".into(), serde_json::json!("小明"));
    let result = engine.render("greeting", &vars, None).unwrap();
    assert_eq!(result, "你好，小明！");
}

#[test]
fn registry_category_is_not_limited_to_hard_coded_engine_enum() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("experimental.md"),
        "---\nname: experimental_prompt\n---\n实验模板",
    )
    .unwrap();
    fs::write(
        tmp.path().join("_registry.yaml"),
        "prompts:\n  experimental_prompt:\n    path: \"experimental.md\"\n    title: \"实验模板\"\n    category: \"experimental\"\n    tags: [\"test\", \"experimental\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"实验分类模板\"\ncategories:\n  experimental:\n    label: \"实验\"\n    order: 90\n    description: \"实验模板\"\n",
    )
    .unwrap();

    let engine = TeraPromptEngine::from_directory(tmp.path()).unwrap();
    let entry = engine.get("experimental_prompt").unwrap();

    assert_eq!(entry.category, "experimental");
}

#[test]
fn load_defaults_failure_does_not_mutate_runtime_registry_or_entries() {
    let tmp = tempfile::tempdir().unwrap();

    let md = "---\nname: greeting\n---\n你好，{{ name }}！";
    fs::write(tmp.path().join("greeting.md"), md).unwrap();
    write_test_registry(
        tmp.path(),
        "  greeting:\n    path: \"greeting.md\"\n    title: \"问候\"\n    category: \"writing\"\n    tags: [\"test\", \"writing\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"问候模板\"\n",
    );

    let engine = TeraPromptEngine::from_directory(tmp.path()).unwrap();
    assert_eq!(engine.list_all().len(), 1);
    assert!(engine.registry.read().get("broken").is_none());

    fs::write(
        tmp.path().join("broken.md"),
        "---\nname: broken\n---\n<!-- @include missing.md -->",
    )
    .unwrap();
    write_test_registry(
        tmp.path(),
        "  greeting:\n    path: \"greeting.md\"\n    title: \"问候\"\n    category: \"writing\"\n    tags: [\"test\", \"writing\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"问候模板\"\n  broken:\n    path: \"broken.md\"\n    title: \"损坏模板\"\n    category: \"writing\"\n    tags: [\"test\", \"broken\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"include 缺失的模板\"\n",
    );

    let err = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(engine.load_defaults())
        .expect_err("缺失 include 应让 load_defaults 失败");
    assert!(err.to_string().contains("include 文件不存在"));

    assert_eq!(engine.list_all().len(), 1);
    assert!(engine.get("broken").is_err());
    assert!(engine.registry.read().get("broken").is_none());

    let mut vars = HashMap::new();
    vars.insert("name".into(), serde_json::json!("小明"));
    let result = engine.render("greeting", &vars, None).unwrap();
    assert_eq!(result, "你好，小明！");
}

#[test]
fn load_defaults_refreshes_existing_registry_metadata_without_overwriting_content() {
    let tmp = tempfile::tempdir().unwrap();

    fs::write(
        tmp.path().join("greeting.md"),
        "---\nname: greeting\n---\n原始内容",
    )
    .unwrap();
    write_test_registry(
        tmp.path(),
        "  greeting:\n    path: \"greeting.md\"\n    title: \"旧标题\"\n    category: \"writing\"\n    tags: [\"old\"]\n    version: \"1.0\"\n    used_by: [\"OldCaller\"]\n    dependencies: []\n    description: \"旧描述\"\n",
    );
    let engine = TeraPromptEngine::from_directory(tmp.path()).unwrap();
    let updated = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(engine.update("greeting", "用户修改 {{ name }}".to_string(), "tester"))
        .unwrap();
    assert_eq!(updated.version, "1.0.1");
    assert!(updated.is_modified);

    write_test_registry(
        tmp.path(),
        "  greeting:\n    path: \"greeting.md\"\n    title: \"新标题\"\n    category: \"outline\"\n    tags: [\"new\", \"registry\"]\n    version: \"9.9\"\n    used_by: [\"NewCaller\"]\n    dependencies: []\n    description: \"新描述\"\n",
    );

    let loaded = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(engine.load_defaults())
        .unwrap();
    assert_eq!(loaded, 0);

    let entry = engine.get("greeting").unwrap();
    assert_eq!(entry.title, "新标题");
    assert_eq!(entry.description, "新描述");
    assert_eq!(entry.category, "outline");
    assert_eq!(entry.tags, vec!["new".to_string(), "registry".to_string()]);
    assert_eq!(entry.used_by, vec!["NewCaller".to_string()]);
    assert_eq!(entry.path, "greeting.md");
    assert_eq!(entry.content, "用户修改 {{ name }}");
    assert_eq!(entry.version, "1.0.1");
    assert!(entry.is_modified);
}

#[test]
fn load_defaults_removes_retired_prompt_runtime_state_when_registry_and_file_are_removed() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("old.md"), "---\nname: old\n---\n旧模板").unwrap();
    fs::write(
        tmp.path().join("keep.md"),
        "---\nname: keep\n---\n保留 {{ name }}",
    )
    .unwrap();
    write_test_registry(
        tmp.path(),
        "  old:\n    path: \"old.md\"\n    title: \"旧模板\"\n    category: \"writing\"\n    tags: [\"old\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"旧模板\"\n  keep:\n    path: \"keep.md\"\n    title: \"保留模板\"\n    category: \"writing\"\n    tags: [\"keep\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"保留模板\"\n",
    );
    let engine = TeraPromptEngine::from_directory(tmp.path()).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(engine.update("old", "用户旧模板".to_string(), "tester"))
        .unwrap();
    engine
        .set_project_override(42, "old", "旧覆盖".to_string())
        .unwrap();

    fs::remove_file(tmp.path().join("old.md")).unwrap();
    write_test_registry(
        tmp.path(),
        "  keep:\n    path: \"keep.md\"\n    title: \"保留模板\"\n    category: \"writing\"\n    tags: [\"keep\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"保留模板\"\n",
    );

    let loaded = rt.block_on(engine.load_defaults()).unwrap();
    assert_eq!(loaded, 0);

    assert!(engine.get("old").is_err());
    assert!(engine.version_history("old").is_err());
    assert!(rt.block_on(engine.reset("old")).is_err());
    assert!(engine.render("old", &HashMap::new(), None).is_err());
    assert!(engine.render("old", &HashMap::new(), Some(42)).is_err());
    let retained_names = engine
        .list_all()
        .into_iter()
        .map(|entry| entry.name)
        .collect::<HashSet<_>>();
    assert_eq!(retained_names, HashSet::from(["keep".to_string()]));

    let mut vars = HashMap::new();
    vars.insert("name".to_string(), serde_json::json!("小明"));
    assert_eq!(engine.render("keep", &vars, None).unwrap(), "保留 小明");
}

#[test]
fn load_defaults_rejects_prompt_file_removed_from_registry_without_mutating_runtime() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("old.md"), "---\nname: old\n---\n旧模板").unwrap();
    fs::write(tmp.path().join("keep.md"), "---\nname: keep\n---\n保留").unwrap();
    write_test_registry(
        tmp.path(),
        "  old:\n    path: \"old.md\"\n    title: \"旧模板\"\n    category: \"writing\"\n    tags: [\"old\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"旧模板\"\n  keep:\n    path: \"keep.md\"\n    title: \"保留模板\"\n    category: \"writing\"\n    tags: [\"keep\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"保留模板\"\n",
    );
    let engine = TeraPromptEngine::from_directory(tmp.path()).unwrap();
    write_test_registry(
        tmp.path(),
        "  keep:\n    path: \"keep.md\"\n    title: \"保留模板\"\n    category: \"writing\"\n    tags: [\"keep\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"保留模板\"\n",
    );

    let err = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(engine.load_defaults())
        .expect_err("registry 删除但文件仍在应失败");
    assert!(err.to_string().contains("已不在 registry 中"));

    assert_eq!(engine.get("old").unwrap().content, "旧模板");
    assert_eq!(
        engine.render("old", &HashMap::new(), None).unwrap(),
        "旧模板"
    );
    assert!(engine.get("keep").is_ok());
}

#[test]
fn render_missing_template_returns_error() {
    let engine = TeraPromptEngine::with_defaults(vec![make_entry("greeting", "你好")]).unwrap();
    let vars = HashMap::new();

    assert!(engine.render("missing", &vars, None).is_err());
}

#[test]
fn missing_prompt_is_reported_as_not_found_across_engine_entrypoints() {
    let engine = TeraPromptEngine::with_defaults(vec![make_entry("greeting", "你好")]).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let vars = HashMap::new();
    let provided = HashSet::new();

    let errors = vec![
        engine
            .render("missing", &vars, None)
            .expect_err("render should fail"),
        engine.get("missing").expect_err("get should fail"),
        engine
            .version_history("missing")
            .expect_err("version_history should fail"),
        engine
            .diff("missing", "1.0", "1.0")
            .expect_err("diff should fail"),
        engine
            .validate_variables("missing", &provided)
            .expect_err("validate_variables should fail"),
        runtime
            .block_on(engine.update("missing", "新内容".to_string(), "tester"))
            .expect_err("update should fail"),
        runtime
            .block_on(engine.update_many(
                vec![("missing".to_string(), "新内容".to_string())],
                "tester",
            ))
            .expect_err("update_many should fail"),
        runtime
            .block_on(engine.reset("missing"))
            .expect_err("reset should fail"),
    ];

    for err in errors {
        assert!(matches!(err, AppError::NotFound(_)));
        assert!(err.to_string().contains("提示词不存在：missing"));
    }
}
