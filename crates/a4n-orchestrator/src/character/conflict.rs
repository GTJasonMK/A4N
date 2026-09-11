//! 角色一致性检查 application。
//!
//! 统一拥有输入合同、上下文与请求构造、workflow/checkpoint、LLM 调用和输出校验。

mod contracts;
mod execution;
mod request;
mod workflow;

pub const CHARACTER_CONFLICT_KIND: &str = "character_conflict_check";
pub const CHARACTER_CONFLICT_PROGRESS_TOTAL: u32 = 4;

pub use contracts::{
    CharacterConflictCheckInput, CharacterConflictProgressReporter, CharacterConflictSummary,
};
pub use execution::validate_character_conflict_input;

/// Runtime-facing character-consistency boundary.
#[async_trait::async_trait]
pub trait CharacterConflictApplicationPort: Send + Sync {
    async fn preview(
        &self,
        input: CharacterConflictCheckInput,
        prompt_renderer: &dyn a4n_prompt::rendering::PromptRenderer,
    ) -> Result<a4n_llm::provider::ChatRequest, a4n_error::AppError>;

    async fn check(
        &self,
        input: CharacterConflictCheckInput,
        prompt_renderer: &dyn a4n_prompt::rendering::PromptRenderer,
        progress: CharacterConflictProgressReporter<'_>,
        active_task_id: Option<&str>,
    ) -> Result<Vec<CharacterConflictSummary>, a4n_error::AppError>;
}

#[cfg(test)]
mod tests;
