//! PromptEngine trait：提示词模板引擎
//!
//! 基于 Tera 模板引擎，支持 Git 版本管理、项目级覆盖、A/B 测试。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;

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

/// PromptEngine trait
///
/// 提示词渲染、版本管理、项目覆盖的完整抽象。
#[async_trait]
pub trait PromptEngine: Send + Sync {
    /// 渲染提示词模板（优先项目覆盖 → 全局 → 默认）
    fn render(
        &self,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError>;

    /// 获取提示词内容
    fn get(&self, name: &str) -> Result<PromptEntry, AppError>;

    /// 列出所有提示词
    fn list_all(&self) -> Vec<PromptEntry>;

    /// 加载默认提示词（从 Markdown 文件初始化到数据库）
    async fn load_defaults(&self) -> Result<usize, AppError>;

    /// 更新提示词（自动 Git commit）
    async fn update(
        &self,
        name: &str,
        content: String,
        author: &str,
    ) -> Result<PromptEntry, AppError>;

    /// 批量更新提示词。
    ///
    /// 实现必须先验证整批更新，再提交持久化和运行时状态；任一条失败时不能留下
    /// 本批次的部分运行时更新。
    async fn update_many(
        &self,
        updates: Vec<(String, String)>,
        author: &str,
    ) -> Result<Vec<PromptEntry>, AppError>;

    /// 重置为原始内容（Git revert）
    async fn reset(&self, name: &str) -> Result<(), AppError>;

    /// 获取提示词版本历史
    fn version_history(&self, name: &str) -> Result<Vec<PromptVersion>, AppError>;

    /// 设置项目级提示词覆盖
    async fn set_project_override(
        &self,
        project_id: i64,
        name: &str,
        content: String,
    ) -> Result<(), AppError>;

    /// 移除项目级提示词覆盖
    async fn remove_project_override(&self, project_id: i64, name: &str) -> Result<(), AppError>;

    /// 对比两个版本差异
    fn diff(&self, name: &str, version_a: &str, version_b: &str) -> Result<String, AppError>;

    /// 验证模板变量完整性
    fn validate_variables(
        &self,
        template_name: &str,
        variables: &HashSet<String>,
    ) -> Result<Vec<String>, AppError>;

    /// 获取中央注册表中的提示词分类。
    fn categories(&self) -> Result<Vec<PromptCategory>, AppError>;

    /// A/B 测试：同时渲染两个版本
    fn render_ab(
        &self,
        template_name: &str,
        variables: &HashMap<String, serde_json::Value>,
        project_id: Option<i64>,
    ) -> Result<(String, String), AppError>;
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
