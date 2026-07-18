use a4n_core::error::AppError;
use serde::Serialize;

/// 单条 prompt version commit 摘要。
#[derive(Debug, Clone, Serialize)]
pub struct CommitInfo {
    /// 完整 SHA。
    pub sha: String,
    /// commit message。
    pub message: String,
    /// 作者签名 `name <email>`。
    pub author: String,
    /// Unix 时间戳（秒）。
    pub timestamp: i64,
}

/// Prompt version persistence boundary.
///
/// The core prompt engine depends on this trait instead of a concrete Git
/// implementation so normal prompt rendering does not compile `git2`.
pub trait PromptVersionStore: Send + Sync {
    fn commit_prompt(&self, name: &str, content: &str, message: &str) -> Result<String, AppError>;

    fn commit_prompts(
        &self,
        prompts: &[(String, String)],
        message: &str,
    ) -> Result<String, AppError>;

    fn history(&self, name: &str) -> Result<Vec<CommitInfo>, AppError>;

    fn get_at(&self, name: &str, sha: &str) -> Result<String, AppError>;

    fn latest(&self, name: &str) -> Result<Option<String>, AppError>;
}
