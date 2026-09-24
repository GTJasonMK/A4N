//! Runtime-facing contracts for scene-profile synchronization.
//!
//! Scene persistence, permissions, workflow state and RAG ownership remain in
//! the scene owner; this crate only describes Host callbacks and execution
//! progress crossing the Runtime boundary.

use a4n_api_contract::profile::SyncSceneProfilesInput;
use a4n_domain::scene_profile::SceneProfileSyncRecordsInput;
use a4n_error::AppError;
use a4n_prompt_contract::rendering::PromptRenderer;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

pub const SCENE_PROFILE_SYNC_PROGRESS_TOTAL: u32 = 100;

pub type SceneProfileSyncProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);
pub type SceneProfileSyncStreamReporter<'a> = &'a (dyn Fn(&str) + Send + Sync);

pub struct SceneProfileSyncProgress<'a> {
    reporter: Option<SceneProfileSyncProgressReporter<'a>>,
    stream_reporter: Option<SceneProfileSyncStreamReporter<'a>>,
    cancel_token: Option<&'a CancellationToken>,
    active_task_id: Option<&'a str>,
}

impl<'a> SceneProfileSyncProgress<'a> {
    pub fn new(
        reporter: Option<SceneProfileSyncProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
        active_task_id: Option<&'a str>,
    ) -> Self {
        Self {
            reporter,
            stream_reporter: None,
            cancel_token,
            active_task_id,
        }
    }

    pub fn with_stream_reporter(
        mut self,
        stream_reporter: Option<SceneProfileSyncStreamReporter<'a>>,
    ) -> Self {
        self.stream_reporter = stream_reporter;
        self
    }

    pub fn active_task_id(&self) -> Option<&str> {
        self.active_task_id
    }

    pub fn cancel_token(&self) -> Option<&CancellationToken> {
        self.cancel_token
    }

    pub fn stream_text(&self, text: &str) {
        if let Some(reporter) = self.stream_reporter {
            reporter(text);
        }
    }

    pub fn has_stream_reporter(&self) -> bool {
        self.stream_reporter.is_some()
    }

    pub fn report(&self, current: u32, total: u32, message: &str) {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
    }

    pub fn check_cancelled(&self) -> Result<(), AppError> {
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            Err(AppError::Cancelled)
        } else {
            Ok(())
        }
    }
}

#[async_trait]
pub trait SceneProfileSyncHostPort: Send + Sync {
    async fn replace_chapter_scene_sync_records(
        &self,
        user_id: i64,
        input: SceneProfileSyncRecordsInput,
    ) -> Result<(), AppError>;

    async fn sync_scene_rag_records(
        &self,
        project_id: i64,
        workflow_run_id: i64,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait SceneProfileSyncApplicationPort: Send + Sync {
    async fn sync(
        &self,
        user_id: i64,
        input: SyncSceneProfilesInput,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn SceneProfileSyncHostPort,
        progress: Option<&SceneProfileSyncProgress<'_>>,
    ) -> Result<(), AppError>;

    async fn resume_workflow_run(
        &self,
        user_id: i64,
        workflow_run_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn SceneProfileSyncHostPort,
    ) -> Result<(), AppError>;
}
