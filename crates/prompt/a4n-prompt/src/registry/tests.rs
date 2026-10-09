use std::fs;
use std::io::Write;

use tempfile::TempDir;

use super::PromptRegistry;

#[test]
fn load_missing_file_returns_error() {
    let tmp = TempDir::new().unwrap();
    let err = PromptRegistry::load_from_dir(tmp.path()).unwrap_err();
    assert!(err.to_string().contains("缺少提示词注册表"));
}

#[test]
fn load_parse_and_lookup() {
    let tmp = TempDir::new().unwrap();
    fs::create_dir_all(tmp.path().join("novel/02_blueprint")).unwrap();
    fs::write(
        tmp.path().join("novel/02_blueprint/blueprint.md"),
        "---\nname: blueprint\n---\nBlueprint",
    )
    .unwrap();
    let mut file = fs::File::create(tmp.path().join("_registry.yaml")).unwrap();
    writeln!(
        file,
        "prompts:\n  blueprint:\n    path: \"novel/02_blueprint/blueprint.md\"\n    title: \"蓝图\"\n    category: \"blueprint\"\n    description: \"蓝图生成\"\n    tags: [\"blueprint\"]\n    version: \"1.0\"\n    used_by:\n      - \"BlueprintPipeline.execute\"\n    dependencies: []\ncategories:\n  blueprint:\n    label: \"蓝图\"\n    order: 2\n    description: \"蓝图模板\""
    )
    .unwrap();

    let registry = PromptRegistry::load_from_dir(tmp.path()).unwrap();
    let blueprint = registry.get("blueprint").unwrap();
    assert_eq!(blueprint.path, "novel/02_blueprint/blueprint.md");
    assert_eq!(blueprint.used_by, vec!["BlueprintPipeline.execute"]);
}

#[test]
fn load_registry_requires_tags() {
    let tmp = TempDir::new().unwrap();
    let mut file = fs::File::create(tmp.path().join("_registry.yaml")).unwrap();
    writeln!(
        file,
        "prompts:\n  inspiration:\n    path: \"novel/01_inspiration/inspiration.md\"\n    title: \"灵感对话\"\n    category: \"inspiration\"\n    description: \"缺少 tags 应失败\"\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\ncategories:\n  inspiration:\n    label: \"构思\"\n    order: 1\n    description: \"构思模板\""
    )
    .unwrap();

    let err = PromptRegistry::load_from_dir(tmp.path()).unwrap_err();
    assert!(err.to_string().contains("missing field `tags`"));
}

#[test]
fn validate_catches_missing_dep_and_file() {
    let tmp = TempDir::new().unwrap();
    let mut file = fs::File::create(tmp.path().join("_registry.yaml")).unwrap();
    writeln!(
        file,
        "prompts:\n  a:\n    path: \"missing.md\"\n    title: \"A\"\n    category: \"x\"\n    tags: [\"test\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies:\n      - \"nope\"\n    description: \"A prompt\"\ncategories: {{}}"
    )
    .unwrap();

    let error = PromptRegistry::load_from_dir(tmp.path()).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("文件不存在"));
    assert!(message.contains("依赖不存在"));
    assert!(message.contains("分类未定义"));
}

#[test]
fn load_requires_complete_category_metadata() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("test.md"), "---\nname: test\n---\nHello").unwrap();
    let mut file = fs::File::create(tmp.path().join("_registry.yaml")).unwrap();
    writeln!(
        file,
        "prompts:\n  test:\n    path: \"test.md\"\n    title: \"测试\"\n    category: \"writing\"\n    tags: [\"test\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"测试模板\"\ncategories:\n  writing:\n    label: \"写作\"\n    description: \"写作模板\""
    )
    .unwrap();

    let error = PromptRegistry::load_from_dir(tmp.path()).unwrap_err();
    assert!(error.to_string().contains("missing field `order`"));
}

#[test]
fn registry_rejects_assembly_without_an_enabled_block() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("test.md"), "---\nname: test\n---\nHello").unwrap();
    fs::write(
        tmp.path().join("_registry.yaml"),
        "prompts:\n  test:\n    path: \"test.md\"\n    title: \"测试\"\n    category: \"writing\"\n    tags: [\"test\"]\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\n    description: \"测试模板\"\n    assembly_blocks:\n      - id: primary\n        kind: template\n        value: test\n        enabled: false\ncategories:\n  writing:\n    label: \"写作\"\n    order: 1\n    description: \"写作模板\"",
    )
    .unwrap();

    let error = PromptRegistry::load_from_dir(tmp.path()).unwrap_err();

    assert!(error.to_string().contains("至少需要启用一个模块"));
}
