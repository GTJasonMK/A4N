use std::collections::HashMap;

use serde::Deserialize;

/// 单条提示词在注册表中的元数据。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PromptRegistryEntry {
    /// 相对 prompts/ 根目录的路径。
    pub path: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub tags: Vec<String>,
    pub version: String,
    /// 谁调用此提示词（如 `BlueprintPipeline.execute`）。
    pub used_by: Vec<String>,
    /// 此提示词依赖哪些其他提示词。
    pub dependencies: Vec<String>,
}

/// 分类定义。
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CategoryDef {
    pub label: String,
    pub order: i32,
    pub description: String,
}

/// `_registry.yaml` 顶层结构。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistryFile {
    pub prompts: HashMap<String, PromptRegistryEntry>,
    pub categories: HashMap<String, CategoryDef>,
}
