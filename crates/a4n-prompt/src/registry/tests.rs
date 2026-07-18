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
    let mut file = fs::File::create(tmp.path().join("_registry.yaml")).unwrap();
    writeln!(
        file,
        "version: \"1.0\"\nprompts:\n  blueprint:\n    path: \"novel/02_blueprint/blueprint.md\"\n    title: \"蓝图\"\n    category: \"blueprint\"\n    description: \"蓝图生成\"\n    tags: [\"blueprint\"]\n    version: \"1.0\"\n    used_by:\n      - \"BlueprintPipeline.execute\"\n    dependencies:\n      - \"inspiration\"\ncategories:\n  blueprint:\n    label: \"蓝图\"\n    order: 2"
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
        "version: \"1.0\"\nprompts:\n  inspiration:\n    path: \"novel/01_inspiration/inspiration.md\"\n    title: \"灵感对话\"\n    category: \"inspiration\"\n    description: \"缺少 tags 应失败\"\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\ncategories:\n  inspiration:\n    label: \"构思\""
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
        "prompts:\n  a:\n    path: \"missing.md\"\n    title: \"A\"\n    category: \"x\"\n    tags: [\"test\"]\n    version: \"1.0\"\n    dependencies:\n      - \"nope\"\n    description: \"A prompt\"\ncategories: {{}}"
    )
    .unwrap();

    let registry = PromptRegistry::load_from_dir(tmp.path()).unwrap();
    let errors = registry.validate(tmp.path());
    assert!(errors.iter().any(|error| error.contains("文件不存在")));
    assert!(errors.iter().any(|error| error.contains("依赖不存在")));
    assert!(errors.iter().any(|error| error.contains("分类未定义")));
}

#[test]
fn validate_requires_category_order() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("test.md"), "---\nname: test\n---\nHello").unwrap();
    let mut file = fs::File::create(tmp.path().join("_registry.yaml")).unwrap();
    writeln!(
        file,
        "prompts:\n  test:\n    path: \"test.md\"\n    title: \"测试\"\n    category: \"writing\"\n    tags: [\"test\"]\n    version: \"1.0\"\n    dependencies: []\n    description: \"测试模板\"\ncategories:\n  writing:\n    label: \"写作\""
    )
    .unwrap();

    let registry = PromptRegistry::load_from_dir(tmp.path()).unwrap();
    let errors = registry.validate(tmp.path());
    assert!(
        errors
            .iter()
            .any(|error| error == "分类缺少 order：writing")
    );
}
