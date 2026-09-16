//! Runtime-facing contracts for character AI workers.
//!
//! Prompt rendering, progress, cancellation and trace adapters cross this
//! boundary; character persistence, permissions and workflow state stay in the
//! character owner.

use a4n_api_contract::character::{
    ApplyImplicitUpdateDecisionInput, ApplyImplicitUpdateDecisionResultSummary,
    CharacterConflictCheckInput, CharacterConflictSummary, CheckImplicitUpdateInput,
    DiscoverCharacterProfilesInput, DiscoverCharacterProfilesResult, ImplicitUpdateDecisionSummary,
    SyncAllCharacterProfilesInput, SyncAllCharacterProfilesResult, SyncCharacterProfileInput,
    SyncCharacterProfileResult,
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatMessage, ChatRequest};
use a4n_prompt_contract::rendering::PromptRenderer;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

pub const CHARACTER_DISCOVERY_KIND: &str = "character_discovery";
pub const CHARACTER_DISCOVERY_CALL_KEY: &str = "character_discovery.llm";
pub const CHARACTER_DISCOVERY_PROGRESS_TOTAL: u32 = 100;
pub const CHARACTER_PROFILE_SYNC_STEPS: u32 = 10;
pub const CHARACTER_CONFLICT_KIND: &str = "character_conflict_check";
pub const CHARACTER_CONFLICT_PROGRESS_TOTAL: u32 = 4;
pub const IMPLICIT_UPDATE_PROMPT_NAME: &str = "implicit_update";
pub const IMPLICIT_UPDATE_WORKFLOW_KIND: &str = "implicit_update_review";

pub type CharacterDiscoveryProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Clone, Copy)]
pub struct CharacterDiscoveryProgress<'a> {
    pub reporter: Option<CharacterDiscoveryProgressReporter<'a>>,
    pub stream_reporter: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub active_task_id: Option<&'a str>,
}

impl CharacterDiscoveryProgress<'_> {
    pub fn report(&self, current: u32, message: &str) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter {
            reporter(current, CHARACTER_DISCOVERY_PROGRESS_TOTAL, message)?;
        }
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(AppError::Cancelled);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CharacterDiscoveryRequestPreview {
    pub analyzed_chapter_count: usize,
    pub request: ChatRequest,
}

pub type CharacterProfileSyncProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Clone, Copy, Default)]
pub struct CharacterProfileSyncProgress<'a> {
    pub reporter: Option<CharacterProfileSyncProgressReporter<'a>>,
    pub active_task_id: Option<&'a str>,
}

impl CharacterProfileSyncProgress<'_> {
    pub fn report(&self, current: u32, message: &str) -> Result<(), AppError> {
        self.report_with_total(current, CHARACTER_PROFILE_SYNC_STEPS, message)
    }

    pub fn report_with_total(
        &self,
        current: u32,
        total: u32,
        message: &str,
    ) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter {
            reporter(current, total, message)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CharacterProfileSyncPreviewStep {
    pub id: String,
    pub title: String,
    pub prompt_name: &'static str,
    pub note: String,
    pub request: ChatRequest,
}

#[derive(Debug, Clone)]
pub struct CharacterProfileSyncStageTraceInput {
    pub profile_name: String,
    pub phase: String,
    pub messages: Vec<ChatMessage>,
    pub response_content: String,
    pub temperature: Option<f32>,
    pub parse_error: Option<String>,
}

pub trait CharacterProfileSyncStageTracePort: Send + Sync {
    fn record_stage_trace(&self, input: CharacterProfileSyncStageTraceInput) -> Option<String>;
}

pub type CharacterConflictProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[async_trait]
pub trait CharacterDiscoveryApplicationPort: Send + Sync {
    async fn preview_discovery(
        &self,
        user_id: i64,
        input: DiscoverCharacterProfilesInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<CharacterDiscoveryRequestPreview, AppError>;

    async fn discover(
        &self,
        user_id: i64,
        input: DiscoverCharacterProfilesInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: Option<CharacterDiscoveryProgress<'_>>,
    ) -> Result<DiscoverCharacterProfilesResult, AppError>;
}

#[async_trait]
pub trait CharacterProfileSyncApplicationPort: Send + Sync {
    async fn preview_profile_sync(
        &self,
        user_id: i64,
        input: SyncCharacterProfileInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Option<CharacterProfileSyncPreviewStep>, AppError>;

    async fn sync_profile(
        &self,
        user_id: i64,
        input: SyncCharacterProfileInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: Option<CharacterProfileSyncProgress<'_>>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<SyncCharacterProfileResult, AppError>;

    async fn preview_all_profiles_sync(
        &self,
        user_id: i64,
        input: SyncAllCharacterProfilesInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<CharacterProfileSyncPreviewStep>, AppError>;

    async fn sync_all_profiles(
        &self,
        user_id: i64,
        input: SyncAllCharacterProfilesInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: CharacterProfileSyncProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<SyncAllCharacterProfilesResult, AppError>;
}

#[async_trait]
pub trait CharacterConflictApplicationPort: Send + Sync {
    async fn preview(
        &self,
        input: CharacterConflictCheckInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError>;

    async fn check(
        &self,
        input: CharacterConflictCheckInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: CharacterConflictProgressReporter<'_>,
        active_task_id: Option<&str>,
    ) -> Result<Vec<CharacterConflictSummary>, AppError>;
}

#[async_trait]
pub trait CharacterImplicitReviewCheckApplicationPort: Send + Sync {
    async fn check(
        &self,
        user_id: i64,
        input: CheckImplicitUpdateInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ImplicitUpdateDecisionSummary, AppError>;
}

#[async_trait]
pub trait CharacterImplicitReviewPreviewApplicationPort: Send + Sync {
    async fn preview(
        &self,
        user_id: i64,
        input: CheckImplicitUpdateInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError>;
}

#[async_trait]
pub trait CharacterImplicitReviewApplyApplicationPort: Send + Sync {
    async fn apply(
        &self,
        user_id: i64,
        input: ApplyImplicitUpdateDecisionInput,
    ) -> Result<ApplyImplicitUpdateDecisionResultSummary, AppError>;
}
