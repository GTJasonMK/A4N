//! 章节场景生成 application。
//!
//! 统一拥有输入合同、上下文与请求构造、workflow/checkpoint、LLM 调用、输出校验和落库。

mod context;
mod contracts;
mod execution;
mod output;
mod request;
mod workflow;

pub use contracts::ChapterSceneRequestPreview;

/// Runtime-facing chapter-scene generation boundary.
#[async_trait::async_trait]
pub trait ChapterSceneGenerationApplicationPort: Send + Sync {
    async fn project_id_for_user(
        &self,
        user_id: i64,
        input: &a4n_api_contract::chapter::GenerateChapterSceneInput,
    ) -> Result<i64, a4n_error::AppError>;

    async fn preview_for_user(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter::GenerateChapterSceneInput,
        prompt_renderer: &dyn a4n_prompt::rendering::PromptRenderer,
    ) -> Result<ChapterSceneRequestPreview, a4n_error::AppError>;

    async fn generate_for_user(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter::GenerateChapterSceneInput,
        prompt_renderer: &dyn a4n_prompt::rendering::PromptRenderer,
        progress: a4n_api_contract::chapter::ChapterSceneProgressReporter<'_>,
        active_task_id: Option<&str>,
    ) -> Result<a4n_api_contract::chapter::ChapterSceneSummary, a4n_error::AppError>;
}

#[cfg(test)]
mod tests;
