use std::collections::HashMap;

use super::{CategoryDef, PromptRegistryEntry, RegistryFile};

/// 已加载的提示词注册表。
#[derive(Debug, Clone, Default)]
pub struct PromptRegistry {
    pub(super) file: RegistryFile,
    /// 是否成功加载过 `_registry.yaml`，用于区分空注册表与未加载状态。
    pub(super) loaded: bool,
}

impl PromptRegistry {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Option<&PromptRegistryEntry> {
        self.file.prompts.get(name)
    }

    pub fn categories(&self) -> &HashMap<String, CategoryDef> {
        &self.file.categories
    }
}
