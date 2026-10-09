use std::path::Path;

use super::PromptRegistry;

impl PromptRegistry {
    pub(super) fn validation_errors(&self, prompts_dir: &Path) -> Vec<String> {
        let mut errors = Vec::new();

        for (name, entry) in &self.file.prompts {
            let mut block_ids = std::collections::HashSet::new();
            let mut assembly_call_keys = std::collections::HashSet::new();
            for call_key in &entry.assembly_call_keys {
                if call_key.trim().is_empty() || !assembly_call_keys.insert(call_key.as_str()) {
                    errors.push(format!(
                        "装配调用 key 为空或重复：{call_key}（prompt: {name})"
                    ));
                }
            }
            for block in &entry.assembly_blocks {
                if block.id.trim().is_empty() || !block_ids.insert(block.id.as_str()) {
                    errors.push(format!(
                        "装配模块 ID 为空或重复：{}（prompt: {name})",
                        block.id
                    ));
                }
                if block.label.trim().is_empty() || block.value.trim().is_empty() {
                    errors.push(format!(
                        "装配模块 label/value 不能为空：{}（prompt: {name})",
                        block.id
                    ));
                }
            }
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
            for dependency in &entry.dependencies {
                if !self.file.prompts.contains_key(dependency) {
                    errors.push(format!("依赖不存在：{name} → {dependency}"));
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
