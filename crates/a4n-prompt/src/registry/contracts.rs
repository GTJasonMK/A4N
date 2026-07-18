use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 单条提示词在注册表中的元数据。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptRegistryEntry {
    /// 相对 prompts/ 根目录的路径。
    pub path: String,
    pub title: String,
    pub category: String,
    pub description: String,
    pub tags: Vec<String>,
    pub version: String,
    /// 谁调用此提示词（如 `BlueprintPipeline.execute`）。
    #[serde(default)]
    pub used_by: Vec<String>,
    /// 此提示词依赖哪些其他提示词。
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// 分类定义。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CategoryDef {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub order: Option<i32>,
    #[serde(default)]
    pub description: Option<String>,
}

/// `_registry.yaml` 顶层结构。
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
