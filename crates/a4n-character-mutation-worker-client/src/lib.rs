//! Typed Host adapter for initial character profile mutations.

use a4n_api_contract::character::{
    AttributeEventSummary, AttributeHistoryInput, BehaviorRecordSummary,
    CharacterProfileCreateResult, CharacterProfileMutationApplicationPort,
    CharacterProfileMutationResult, CharacterProfileOverviewReadApplicationPort,
    CharacterProfileSummary, CharacterTimelineReadApplicationPort, CreateProfileInput,
    DeletionMarkSummary, GetProfileStateAtChapterInput, ListBehaviorsInput, ListDeletionMarksInput,
    ListProfilesInput, ListSnapshotsInput, ProfileChapterStateSummary, SnapshotReferenceSummary,
    UpdateCharacterProfileInput,
};
use a4n_character_mutation_worker_contract::{
    CHARACTER_MUTATION_CREATE_OPERATION, CHARACTER_MUTATION_UPDATE_OPERATION,
    CHARACTER_MUTATION_WORKER_ID, CHARACTER_OVERVIEW_ARC_OPERATION,
    CHARACTER_OVERVIEW_LIST_OPERATION, CHARACTER_OVERVIEW_PROJECT_OPERATION,
    CHARACTER_OVERVIEW_VOICE_OPERATION, CHARACTER_TIMELINE_ATTRIBUTE_OPERATION,
    CHARACTER_TIMELINE_BEHAVIOR_OPERATION, CHARACTER_TIMELINE_DELETION_OPERATION,
    CHARACTER_TIMELINE_SNAPSHOTS_OPERATION, CHARACTER_TIMELINE_STATE_OPERATION, CreateInput,
    NamedProfileInput, ProfileIdInput, UpdateInput,
};
use a4n_error::AppError;
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47674";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct CharacterMutationWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for CharacterMutationWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CharacterMutationWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl CharacterMutationWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, CHARACTER_MUTATION_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_CHARACTER_MUTATION_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_CHARACTER_MUTATION_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        user_id: i64,
        project_id: i64,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        if user_id <= 0 || project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "user_id 和 project_id 必须大于 0".into(),
            ));
        }
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: user_id.to_string(),
            },
            operation_id: operation.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: CHARACTER_MUTATION_WORKER_ID.into(),
                epoch: 1,
                fence_token: CONTROL_LEASE_FENCE.into(),
                expires_at: CONTROL_LEASE_EXPIRY.into(),
            },
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(worker_client_error)?;
        match response.status {
            WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
                AppError::ConfigError(format!(
                    "character mutation worker 完成响应缺少 payload: {operation}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation,
            )),
            status => Err(AppError::ConfigError(format!(
                "character mutation worker 返回未完成状态: operation={operation}, status={status:?}"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl CharacterProfileMutationApplicationPort for CharacterMutationWorkerClient {
    async fn create(
        &self,
        user_id: i64,
        input: CreateProfileInput,
    ) -> Result<CharacterProfileCreateResult, AppError> {
        self.command(
            user_id,
            input.project_id,
            CHARACTER_MUTATION_CREATE_OPERATION,
            CreateInput(input),
        )
        .await
    }

    async fn update(
        &self,
        user_id: i64,
        input: UpdateCharacterProfileInput,
    ) -> Result<CharacterProfileMutationResult, AppError> {
        self.command(
            user_id,
            input.project_id,
            CHARACTER_MUTATION_UPDATE_OPERATION,
            UpdateInput(input),
        )
        .await
    }
}

#[async_trait::async_trait]
impl CharacterProfileOverviewReadApplicationPort for CharacterMutationWorkerClient {
    async fn profile_project_id(&self, user_id: i64, profile_id: i64) -> Result<i64, AppError> {
        self.command(
            user_id,
            profile_id,
            CHARACTER_OVERVIEW_PROJECT_OPERATION,
            ProfileIdInput { profile_id },
        )
        .await
    }

    async fn list(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<CharacterProfileSummary>, AppError> {
        self.command(
            user_id,
            project_id,
            CHARACTER_OVERVIEW_LIST_OPERATION,
            ListProfilesInput { project_id },
        )
        .await
    }

    async fn character_arc_by_name(
        &self,
        user_id: i64,
        project_id: i64,
        name: String,
    ) -> Result<Option<serde_json::Value>, AppError> {
        self.command(
            user_id,
            project_id,
            CHARACTER_OVERVIEW_ARC_OPERATION,
            NamedProfileInput { project_id, name },
        )
        .await
    }

    async fn voice_fingerprint_by_name(
        &self,
        user_id: i64,
        project_id: i64,
        name: String,
    ) -> Result<Option<serde_json::Value>, AppError> {
        self.command(
            user_id,
            project_id,
            CHARACTER_OVERVIEW_VOICE_OPERATION,
            NamedProfileInput { project_id, name },
        )
        .await
    }
}

#[async_trait::async_trait]
impl CharacterTimelineReadApplicationPort for CharacterMutationWorkerClient {
    async fn attribute_history(
        &self,
        user_id: i64,
        input: AttributeHistoryInput,
    ) -> Result<Vec<AttributeEventSummary>, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_TIMELINE_ATTRIBUTE_OPERATION,
            input,
        )
        .await
    }

    async fn behavior_history(
        &self,
        user_id: i64,
        input: ListBehaviorsInput,
    ) -> Result<Vec<BehaviorRecordSummary>, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_TIMELINE_BEHAVIOR_OPERATION,
            input,
        )
        .await
    }

    async fn deletion_marks(
        &self,
        user_id: i64,
        input: ListDeletionMarksInput,
    ) -> Result<Vec<DeletionMarkSummary>, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_TIMELINE_DELETION_OPERATION,
            input,
        )
        .await
    }

    async fn snapshots(
        &self,
        user_id: i64,
        input: ListSnapshotsInput,
    ) -> Result<Vec<SnapshotReferenceSummary>, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_TIMELINE_SNAPSHOTS_OPERATION,
            input,
        )
        .await
    }

    async fn state_at_chapter(
        &self,
        user_id: i64,
        input: GetProfileStateAtChapterInput,
    ) -> Result<ProfileChapterStateSummary, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_TIMELINE_STATE_OPERATION,
            input,
        )
        .await
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("character mutation worker 请求失败: {error}"))
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict => AppError::InvalidState {
            current: error.message.clone(),
            operation: operation.into(),
        },
        _ => AppError::ConfigError(format!(
            "character mutation worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = CharacterMutationWorkerClient::new(
            "https://character-mutation-worker.example.test",
            Some("character-mutation-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("character-mutation-worker.example.test"));
        assert!(!debug.contains("character-mutation-token"));
    }
}
