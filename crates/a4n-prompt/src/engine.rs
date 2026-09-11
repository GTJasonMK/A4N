pub use a4n_prompt_contract::{PromptCategory, PromptEntry};

/// Prompt version record kept by the implementation and version store.
/// 提示词版本记录
#[derive(Debug, Clone)]
pub struct PromptVersion {
    pub version: String,
    pub timestamp: chrono::NaiveDateTime,
    pub author: String,
    pub message: String,
    /// 版本对应的完整模板内容（用于 diff 和恢复）
    pub content_snapshot: String,
}

impl From<PromptVersion> for a4n_prompt_contract::PromptVersionSummary {
    fn from(version: PromptVersion) -> Self {
        Self {
            version: version.version,
            timestamp: version.timestamp,
            author: version.author,
            message: version.message,
        }
    }
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
            "description": "description",
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
