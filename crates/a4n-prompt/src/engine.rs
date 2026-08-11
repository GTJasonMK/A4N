use serde::{Deserialize, Serialize};

/// 提示词条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptEntry {
    pub name: String,
    pub title: String,
    pub content: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub category: String,
    pub dependencies: Vec<String>,
    pub is_modified: bool,
    /// Git commit-ish
    pub version: String,
    pub modified_at: chrono::NaiveDateTime,
    /// 谁调用此提示词（registry 提供，如 "BlueprintPipeline.execute"）
    #[serde(default)]
    pub used_by: Vec<String>,
    /// 相对 prompts/ 根目录的路径（registry 提供，无 registry 时来自扫描时的 rel path）
    #[serde(default)]
    pub path: Option<String>,
}

/// 提示词版本记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptVersion {
    pub version: String,
    pub timestamp: chrono::NaiveDateTime,
    pub author: String,
    pub message: String,
    /// 版本对应的完整模板内容（用于 diff 和恢复）
    pub content_snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptCategory {
    pub key: String,
    pub label: Option<String>,
    pub order: i32,
    pub description: Option<String>,
}

#[cfg(test)]
mod contract_tests {
    use super::*;

    #[test]
    fn prompt_entry_rejects_unknown_fields() {
        let error = serde_json::from_value::<PromptEntry>(serde_json::json!({
            "name": "test",
            "title": "Test",
            "content": "content",
            "description": null,
            "tags": [],
            "category": "test",
            "dependencies": [],
            "is_modified": false,
            "version": "v1",
            "modified_at": "2026-01-01T00:00:00",
            "used_by": [],
            "path": "test.md",
            "legacy_source": "old"
        }))
        .expect_err("prompt entry must reject unknown fields");
        assert!(error.to_string().contains("legacy_source"));
    }
}
