//! Runtime-facing contracts for the remaining novel-side workers.

use a4n_api_contract::{
    chapter::{ChapterSceneProgressReporter, ChapterSceneSummary, GenerateChapterSceneInput},
    import_reverse::{ReverseEngineerInput, ReverseEngineeringCandidate},
    import_review::ImportReviewSummary,
    shooting_script::{
        ChapterShootingScriptUnitProgressView, ChapterShootingScriptView,
        GenerateChapterShootingScriptInput,
    },
};
use a4n_domain::import_review::ImportChapterDraft;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt_contract::rendering::PromptRenderer;
use async_trait::async_trait;
use std::{future::Future, pin::Pin};
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct ChapterSceneRequestPreview {
    pub chapter_number: i32,
    pub request: ChatRequest,
}

#[async_trait]
pub trait ChapterSceneGenerationApplicationPort: Send + Sync {
    async fn project_id_for_user(
        &self,
        user_id: i64,
        input: &GenerateChapterSceneInput,
    ) -> Result<i64, AppError>;

    async fn preview_for_user(
        &self,
        user_id: i64,
        input: GenerateChapterSceneInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChapterSceneRequestPreview, AppError>;

    async fn generate_for_user(
        &self,
        user_id: i64,
        input: GenerateChapterSceneInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: ChapterSceneProgressReporter<'_>,
        active_task_id: Option<&str>,
    ) -> Result<ChapterSceneSummary, AppError>;
}

pub const CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL: u32 = 100;
pub type ShootingScriptProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);
pub type ShootingScriptStreamReporter<'a> = &'a (dyn Fn(&str) + Send + Sync);

#[async_trait]
pub trait ShootingScriptFullGenerationApplicationPort: Send + Sync {
    async fn generate(
        &self,
        user_id: i64,
        input: GenerateChapterShootingScriptInput,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
        active_task_id: Option<&str>,
    ) -> Result<ChapterShootingScriptView, AppError>;
}

#[async_trait]
pub trait ShootingScriptNextUnitGenerationApplicationPort: Send + Sync {
    async fn generate_next(
        &self,
        user_id: i64,
        input: GenerateChapterShootingScriptInput,
        progress: ShootingScriptProgressReporter<'_>,
        stream_reporter: Option<ShootingScriptStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
        active_task_id: Option<&str>,
    ) -> Result<ChapterShootingScriptUnitProgressView, AppError>;
}

#[async_trait]
pub trait ImportReversePreviewApplicationPort: Send + Sync {
    async fn prepare_request(
        &self,
        input: ReverseEngineerInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError>;
}

pub trait ImportReverseGenerationApplicationPort: Send + Sync {
    fn run<'a>(
        &'a self,
        input: ReverseEngineerInput,
        prompt_renderer: &'a dyn PromptRenderer,
        progress: &'a (dyn for<'b> Fn(u32, u32, &'b str) -> Result<(), AppError> + Send + Sync),
        active_task_id: Option<&'a str>,
    ) -> Pin<Box<dyn Future<Output = Result<ReverseEngineeringCandidate, AppError>> + Send + 'a>>;
}

#[async_trait]
pub trait ImportReviewIngestionApplicationPort: Send + Sync {
    async fn create_from_parsed(
        &self,
        user_id: i64,
        project_id: i64,
        filename: &str,
        chapters: &[ImportChapterDraft],
    ) -> Result<Vec<ImportReviewSummary>, AppError>;
}
