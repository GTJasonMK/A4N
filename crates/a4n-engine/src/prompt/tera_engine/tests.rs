use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;

use super::*;
use crate::prompt::git_store::GitPromptStore;

fn write_test_registry(dir: &Path, entries: &str) {
    fs::write(
        dir.join("_registry.yaml"),
        format!(
            "version: \"1.0\"\nprompts:\n{entries}categories:\n  writing:\n    label: \"写作\"\n    order: 1\n  outline:\n    label: \"大纲\"\n    order: 2"
        ),
    )
    .unwrap();
}

fn make_entry(name: &str, content: &str) -> PromptEntry {
    PromptEntry {
        name: name.to_string(),
        title: name.to_string(),
        content: content.to_string(),
        description: None,
        tags: vec![],
        category: "writing".to_string(),
        dependencies: vec![],
        is_modified: false,
        version: "1.0".to_string(),
        modified_at: Utc::now().naive_utc(),
        used_by: Vec::new(),
        path: None,
    }
}

#[test]
fn test_from_directory() {
    let tmp = tempfile::tempdir().unwrap();

    let md1 = "---\nname: greeting\ntitle: 问候\ncategory: writing\n---\n你好，{{ name }}！";
    let md2 = "---\nname: outline_prompt\ntitle: 大纲生成\ncategory: outline\n---\n为{{ title }}生成大纲。";

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
        "version: \"1.0\"\nprompts:\n  experimental_prompt:\n    path: \"experimental.md\"\n    title: \"实验模板\"\n    category: \"experimental\"\n    tags: [\"test\", \"experimental\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"实验分类模板\"\ncategories:\n  experimental:\n    label: \"实验\"\n    order: 90\n",
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
    assert!(
        engine
            .registry()
            .expect("registry")
            .read()
            .get("broken")
            .is_none()
    );

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
    assert!(
        engine
            .registry()
            .expect("registry")
            .read()
            .get("broken")
            .is_none()
    );

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
    assert_eq!(entry.description.as_deref(), Some("新描述"));
    assert_eq!(entry.category, "outline");
    assert_eq!(entry.tags, vec!["new".to_string(), "registry".to_string()]);
    assert_eq!(entry.used_by, vec!["NewCaller".to_string()]);
    assert_eq!(entry.path.as_deref(), Some("greeting.md"));
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
    rt.block_on(engine.set_project_override(42, "old", "旧覆盖".to_string()))
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
            .get_version("missing", "1.0")
            .expect_err("get_version should fail"),
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

#[test]
fn test_reset_to_original() {
    let tmp = tempfile::tempdir().unwrap();
    let md = "---\nname: test\ntitle: 测试\ncategory: writing\n---\n原始内容";
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
    let tmp = tempfile::tempdir().unwrap();
    let store = GitPromptStore::open_or_init(tmp.path().to_path_buf()).unwrap();
    fs::remove_dir_all(tmp.path()).unwrap();
    let engine = TeraPromptEngine::with_defaults(vec![make_entry("greeting", "旧内容")])
        .unwrap()
        .with_git_store(Arc::new(store))
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
    assert_eq!(history[0].content_snapshot.as_deref(), Some("旧内容"));
}

#[test]
fn with_git_store_rehydrates_latest_committed_prompt_content() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(GitPromptStore::open_or_init(tmp.path().to_path_buf()).unwrap());
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
    assert_eq!(history[0].content_snapshot.as_deref(), Some("旧内容"));
    assert_eq!(history[1].version, "1.0.1");
    assert_eq!(
        history[1].content_snapshot.as_deref(),
        Some("新内容 {{ name }}")
    );
}

#[test]
fn reset_with_git_store_survives_restart() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(GitPromptStore::open_or_init(tmp.path().to_path_buf()).unwrap());
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
    assert_eq!(history[0].content_snapshot.as_deref(), Some("旧内容"));
    assert_eq!(
        history[1].content_snapshot.as_deref(),
        Some("新内容 {{ name }}")
    );
    assert_eq!(history[2].content_snapshot.as_deref(), Some("旧内容"));
}

#[test]
fn reset_with_git_failure_does_not_mutate_runtime_prompt() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(GitPromptStore::open_or_init(tmp.path().to_path_buf()).unwrap());
    let engine = TeraPromptEngine::with_defaults(vec![make_entry("greeting", "旧内容")])
        .unwrap()
        .with_git_store(store)
        .unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime
        .block_on(engine.update("greeting", "新内容".into(), "author"))
        .unwrap();
    fs::remove_dir_all(tmp.path()).unwrap();

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

#[test]
fn test_validate_variables() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("test", "{{ foo }} {{ bar }} {{ baz }}")])
            .unwrap();

    let mut provided = HashSet::new();
    provided.insert("foo".to_string());
    provided.insert("bar".to_string());

    let missing = engine.validate_variables("test", &provided).unwrap();
    assert_eq!(missing, vec!["baz"]);
}

#[test]
fn test_validate_variables_with_filter_expression() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("test", "{{ name | upper }}")]).unwrap();

    let provided = HashSet::new();

    let missing = engine.validate_variables("test", &provided).unwrap();
    assert_eq!(missing, vec!["name"]);
}

#[test]
fn validate_variables_reports_if_condition_variables() {
    let engine =
        TeraPromptEngine::with_defaults(vec![make_entry("test", "{% if user %}你好{% endif %}")])
            .unwrap();

    let provided = HashSet::new();

    let missing = engine.validate_variables("test", &provided).unwrap();
    assert_eq!(missing, vec!["user"]);
}

#[test]
fn validate_variables_uses_for_loop_container_not_loop_local() {
    let engine = TeraPromptEngine::with_defaults(vec![make_entry(
        "test",
        "{% for item in items %}{{ loop.index }} {{ item.name }}{% endfor %}",
    )])
    .unwrap();

    let provided = HashSet::new();

    let missing = engine.validate_variables("test", &provided).unwrap();
    assert_eq!(missing, vec!["items"]);

    let mut provided = HashSet::new();
    provided.insert("items".to_string());
    let missing = engine.validate_variables("test", &provided).unwrap();
    assert!(missing.is_empty());
}

/// 集成测试：从真实仓库 prompts/ 目录加载当前提示词集合，
/// 验证递归扫描、frontmatter 解析、@include 展开、registry 严格注入全链路。
#[test]
fn test_load_real_prompts_dir() {
    // 仓库根目录的 prompts/，相对于 crates/a4n-engine
    let prompts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts");
    if !prompts_dir.is_dir() {
        // 在某些构建环境（孤立沙箱）下可能没有 prompts/，跳过而非失败
        eprintln!("跳过：prompts 目录不存在 ({prompts_dir:?})");
        return;
    }

    let engine = TeraPromptEngine::from_directory(&prompts_dir)
        .expect("from_directory 应能加载真实 prompts/");
    let all = engine.list_all();

    // 关键 prompt 必须存在（service 直接调用的）
    let names: std::collections::HashSet<String> = all.iter().map(|e| e.name.clone()).collect();
    for required in [
        "inspiration",
        "blueprint",
        "outline",
        "part_outline_single",
        "writing",
        "chapter_fact_extraction",
        "chapter_summary_synthesis",
        "chapter_scene_generation",
        "inspiration_project_context_guard",
        "manga_event_beats",
        "manga_character_binding",
        "manga_speech_selection",
        "manga_visual_anchor_extraction",
        "manga_item_extraction",
        "manga_chapter_comic_synthesis",
        "manga_page_planning",
        "manga_page_drawing_prompt",
        "character_portrait_generation",
        "character_discovery",
        "character_conflict_check",
        "blueprint_optimization",
    ] {
        assert!(
            names.contains(required),
            "缺少必需的 prompt: {required}（实际：{:?}）",
            names
        );
    }

    // registry 应已加载并通过完整性校验
    let reg = engine.registry().expect("TeraPromptEngine 应暴露 registry");
    {
        let reg = reg.read();
        let errors = reg.validate(&prompts_dir);
        assert!(errors.is_empty(), "registry 校验失败：{errors:?}");
    }

    // 至少一个 prompt 应有 used_by（registry 元数据）
    let with_used_by = all.iter().filter(|e| !e.used_by.is_empty()).count();
    assert!(
        with_used_by >= 6,
        "应有 ≥6 个 prompt 通过 registry 注入 used_by，实际：{with_used_by}"
    );

    // _partials/ 内的文件不应作为顶层 entry 暴露
    assert!(
        !names.contains("json_only_rule") && !names.contains("_json_only_rule"),
        "_partials 内的文件不应作为顶层 prompt 暴露"
    );

    let portrait_prompt = engine.get("character_portrait_generation").unwrap().content;
    assert!(portrait_prompt.contains("中文 prompt"));
    assert!(!portrait_prompt.contains("英文 prompt"));

    let mut context_guard_vars = HashMap::new();
    context_guard_vars.insert(
        "project_context_json".to_string(),
        serde_json::json!("{\"title\":\"测试项目\"}"),
    );
    let context_guard = engine
        .render(
            "inspiration_project_context_guard",
            &context_guard_vars,
            None,
        )
        .unwrap();
    assert!(context_guard.contains("测试项目"));
    assert!(context_guard.contains("优先尊重"));

    let mut scene_vars = HashMap::new();
    scene_vars.insert("chapter_number".to_string(), serde_json::json!(3));
    scene_vars.insert("chapter_title".to_string(), serde_json::json!("雨夜"));
    scene_vars.insert("scene_number".to_string(), serde_json::json!(2));
    let scene_prompt = engine
        .render("chapter_scene_generation", &scene_vars, None)
        .unwrap();
    assert!(scene_prompt.contains("第 3 章《雨夜》"));
    assert!(scene_prompt.contains("第 2 个场景"));

    let conflict_prompt = engine.get("character_conflict_check").unwrap().content;
    assert!(conflict_prompt.contains("故事一致性检查专家"));
    assert!(conflict_prompt.contains("source_id"));

    let manga_scene_prompt = engine
        .get("manga_visual_anchor_extraction")
        .unwrap()
        .content;
    assert!(manga_scene_prompt.contains("漫画视觉锚点抽取"));
    assert!(manga_scene_prompt.contains("`location_en` 是简短英文标签"));

    let manga_item_prompt = engine.get("manga_item_extraction").unwrap().content;
    assert!(manga_item_prompt.contains("漫画物品锚点抽取"));
    assert!(manga_item_prompt.contains("不要把地点、角色、组织、能力、事件当作物品"));

    for name in ["content_optimization", "content_optimization_chat"] {
        let content = engine.get(name).unwrap().content;
        assert!(content.contains("参数结构"));
        assert!(!content.contains("符合对应 schema"));
    }

    for name in [
        "chapter_video_frame_prompts",
        "chapter_video_prompt",
        "chapter_video_prompt_review",
    ] {
        let content = engine.get(name).unwrap().content;
        assert!(content.contains("中文"), "{name} 应明确要求输出中文提示词");
        for forbidden in [
            "low quality",
            "no reference images",
            "no revision needed",
            "英文或中英混合",
            "camera 6 o'clock",
            "cinematic lighting",
            "sad、angry、tense、emotional",
        ] {
            assert!(
                !content.contains(forbidden),
                "{name} 仍包含旧英文提示词片段: {forbidden}"
            );
        }
    }
}
