use std::collections::HashMap;

use super::{CategoryDef, PromptRegistryEntry, RegistryFile};

/// 已加载的提示词注册表。
#[derive(Debug, Clone, Default)]
pub(crate) struct PromptRegistry {
    pub(super) file: RegistryFile,
}

impl PromptRegistry {
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    pub(crate) fn get(&self, name: &str) -> Option<&PromptRegistryEntry> {
        self.file.prompts.get(name)
    }

    pub(crate) fn categories(&self) -> &HashMap<String, CategoryDef> {
        &self.file.categories
    }
}
