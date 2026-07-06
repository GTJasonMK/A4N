//! 提示词注册表（registry）
//!
//! `prompts/_registry.yaml` 是提示词元数据的唯一来源，便于跨提示词查询使用关系
//!（used_by）、依赖。
//!
//! 每个 .md 文件的 frontmatter 只负责声明 `name` 和模板本地字段，展示、分类、
//! 依赖、调用方等跨文件元数据全部来自注册表。
//!
//! 文件格式（节选）：
//! ```yaml
//! version: "1.0"
//! prompts:
//!   blueprint:
//!     path: "novel/02_blueprint/blueprint.md"
//!     title: "蓝图生成"
//!     category: "blueprint"
//!     tags: ["blueprint", "worldbuilding"]
//!     version: "1.0"
//!     used_by:
//!       - "BlueprintPipeline.execute"
//!     dependencies:
//!       - "inspiration"
//! categories:
//!   blueprint:
//!     label: "蓝图"
//!     order: 2
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use a4n_core::error::AppError;

/// 单条提示词在注册表中的元数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PromptRegistryEntry {
    /// 相对 prompts/ 根目录的路径
    pub path: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub tags: Vec<String>,
    pub version: String,
    /// 谁调用此提示词（如 "BlueprintPipeline.execute"）
    #[serde(default)]
    pub used_by: Vec<String>,
    /// 此提示词依赖哪些其他提示词
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// 分类定义
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryDef {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub order: Option<i32>,
    #[serde(default)]
    pub description: Option<String>,
}

/// `_registry.yaml` 顶层结构
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegistryFile {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub last_updated: Option<String>,
    #[serde(default)]
    pub prompts: HashMap<String, PromptRegistryEntry>,
    #[serde(default)]
    pub categories: HashMap<String, CategoryDef>,
}

/// 已加载的提示词注册表
///
/// 通过 `load_from_dir(prompts_dir)` 从 `_registry.yaml` 加载。
#[derive(Debug, Clone, Default)]
pub struct PromptRegistry {
    file: RegistryFile,
    /// 是否成功加载过 _registry.yaml（区分 "空注册表" 与 "未加载"）
    loaded: bool,
}

impl PromptRegistry {
    pub fn empty() -> Self {
        Self::default()
    }

    /// 从 prompts 目录加载 `_registry.yaml`
    pub fn load_from_dir(prompts_dir: &Path) -> Result<Self, AppError> {
        let path = prompts_dir.join("_registry.yaml");
        if !path.is_file() {
            return Err(AppError::ConfigError(format!(
                "缺少提示词注册表：{}",
                path.display()
            )));
        }

        let text = fs::read_to_string(&path)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("读取注册表 {path:?} 失败：{e}")))?;

        let file: RegistryFile = serde_yaml::from_str(&text)
            .map_err(|e| AppError::Internal(anyhow::anyhow!("解析注册表 YAML 失败：{e}")))?;

        tracing::info!(
            "已加载提示词注册表：{} 个 prompts / {} 个 categories",
            file.prompts.len(),
            file.categories.len(),
        );

        Ok(Self { file, loaded: true })
    }

    pub fn get(&self, name: &str) -> Option<&PromptRegistryEntry> {
        self.file.prompts.get(name)
    }

    pub fn categories(&self) -> &HashMap<String, CategoryDef> {
        &self.file.categories
    }

    /// 校验完整性：返回错误列表（空表示通过）
    ///
    /// 检查：
    /// - prompts 引用的文件是否存在
    /// - dependencies 中的提示词名是否都在 prompts 表里
    /// - category 是否在 categories 中定义
    pub fn validate(&self, prompts_dir: &Path) -> Vec<String> {
        if !self.loaded {
            return vec!["注册表未加载".into()];
        }
        let mut errors = Vec::new();

        for (key, category) in &self.file.categories {
            if category.order.is_none() {
                errors.push(format!("分类缺少 order：{key}"));
            }
        }

        for (name, entry) in &self.file.prompts {
            if entry.path.trim().is_empty() {
                errors.push(format!("path 不能为空（prompt: {name})"));
            } else {
                let full = prompts_dir.join(&entry.path);
                if !full.is_file() {
                    errors.push(format!("文件不存在：{}（prompt: {name})", entry.path));
                }
            }
            if entry.title.trim().is_empty() {
                errors.push(format!("title 不能为空（prompt: {name})"));
            }
            if entry.description.trim().is_empty() {
                errors.push(format!("description 不能为空（prompt: {name})"));
            }
            if entry.version.trim().is_empty() {
                errors.push(format!("version 不能为空（prompt: {name})"));
            }
            if entry.tags.is_empty() {
                errors.push(format!("tags 不能为空（prompt: {name})"));
            }
            for tag in &entry.tags {
                if tag.trim().is_empty() {
                    errors.push(format!("tags 不能包含空值（prompt: {name})"));
                }
            }
            for dep in &entry.dependencies {
                if !self.file.prompts.contains_key(dep) {
                    errors.push(format!("依赖不存在：{name} → {dep}"));
                }
            }
            if entry.category.trim().is_empty() {
                errors.push(format!("category 不能为空（prompt: {name})"));
            } else if !self.file.categories.contains_key(&entry.category) {
                errors.push(format!("分类未定义：{}（prompt: {name})", entry.category));
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn load_missing_file_returns_error() {
        let tmp = TempDir::new().unwrap();
        let err = PromptRegistry::load_from_dir(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("缺少提示词注册表"));
    }

    #[test]
    fn load_parse_and_lookup() {
        let tmp = TempDir::new().unwrap();
        let mut f = fs::File::create(tmp.path().join("_registry.yaml")).unwrap();
        writeln!(
            f,
            "version: \"1.0\"\nprompts:\n  blueprint:\n    path: \"novel/02_blueprint/blueprint.md\"\n    title: \"蓝图\"\n    category: \"blueprint\"\n    description: \"蓝图生成\"\n    tags: [\"blueprint\"]\n    version: \"1.0\"\n    used_by:\n      - \"BlueprintPipeline.execute\"\n    dependencies:\n      - \"inspiration\"\ncategories:\n  blueprint:\n    label: \"蓝图\"\n    order: 2"
        )
        .unwrap();

        let reg = PromptRegistry::load_from_dir(tmp.path()).unwrap();
        let bp = reg.get("blueprint").unwrap();
        assert_eq!(bp.path, "novel/02_blueprint/blueprint.md");
        assert_eq!(bp.used_by, vec!["BlueprintPipeline.execute".to_string()]);
    }

    #[test]
    fn load_registry_requires_tags() {
        let tmp = TempDir::new().unwrap();
        let mut f = fs::File::create(tmp.path().join("_registry.yaml")).unwrap();
        writeln!(
            f,
            "version: \"1.0\"\nprompts:\n  inspiration:\n    path: \"novel/01_inspiration/inspiration.md\"\n    title: \"灵感对话\"\n    category: \"inspiration\"\n    description: \"缺少 tags 应失败\"\n    version: \"1.0\"\n    used_by: []\n    dependencies: []\ncategories:\n  inspiration:\n    label: \"构思\""
        )
        .unwrap();

        let err = PromptRegistry::load_from_dir(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("missing field `tags`"));
    }

    #[test]
    fn validate_catches_missing_dep_and_file() {
        let tmp = TempDir::new().unwrap();
        let mut f = fs::File::create(tmp.path().join("_registry.yaml")).unwrap();
        writeln!(
            f,
            "prompts:\n  a:\n    path: \"missing.md\"\n    title: \"A\"\n    category: \"x\"\n    tags: [\"test\"]\n    version: \"1.0\"\n    dependencies:\n      - \"nope\"\n    description: \"A prompt\"\ncategories: {{}}"
        )
        .unwrap();

        let reg = PromptRegistry::load_from_dir(tmp.path()).unwrap();
        let errs = reg.validate(tmp.path());
        assert!(errs.iter().any(|e| e.contains("文件不存在")));
        assert!(errs.iter().any(|e| e.contains("依赖不存在")));
        assert!(errs.iter().any(|e| e.contains("分类未定义")));
    }

    #[test]
    fn validate_requires_category_order() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("test.md"), "---\nname: test\n---\nHello").unwrap();
        let mut f = fs::File::create(tmp.path().join("_registry.yaml")).unwrap();
        writeln!(
            f,
            "prompts:\n  test:\n    path: \"test.md\"\n    title: \"测试\"\n    category: \"writing\"\n    tags: [\"test\"]\n    version: \"1.0\"\n    dependencies: []\n    description: \"测试模板\"\ncategories:\n  writing:\n    label: \"写作\""
        )
        .unwrap();

        let reg = PromptRegistry::load_from_dir(tmp.path()).unwrap();
        let errs = reg.validate(tmp.path());
        assert!(errs.iter().any(|e| e == "分类缺少 order：writing"));
    }
}
