use std::path::Path;

use super::PromptRegistry;

impl PromptRegistry {
    /// 校验注册表完整性，返回所有错误；空列表表示通过。
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
