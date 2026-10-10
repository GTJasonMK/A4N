//! Typed Host adapter for initial character profile mutations.

use a4n_api_contract::character::{
    AddProfileAttributeInput, AttributeEventSummary, AttributeHistoryInput, BehaviorRecordSummary,
    CharacterAttributeRagSyncHostPort, CharacterProfileAttributeDeletionApplicationPort,
    CharacterProfileAttributeMutationApplicationPort, CharacterProfileCreateResult,
    CharacterProfileDeleteResult, CharacterProfileDeletionApplicationPort,
    CharacterProfileDeletionHostPort, CharacterProfileMutationApplicationPort,
    CharacterProfileMutationResult, CharacterProfileOverviewReadApplicationPort,
    CharacterProfileSummary, CharacterProfileTimelineMutationApplicationPort,
    CharacterProfileTimelineMutationHostPort, CharacterTimelineReadApplicationPort,
    CheckProfileSyncConflictInput, CreateProfileInput, DeletionMarkSummary,
    DiffProfileChaptersInput, ExecuteAttributeDeletionInput, GetProfileStateAtChapterInput,
    GetSnapshotInput, ListBehaviorsInput, ListDeletionMarksInput, ListProfilesInput,
    ListSnapshotsInput, ModifyProfileAttributeInput, ProfileChapterStateSummary,
    ProfileSyncConflictSummary, RequestAttributeDeletionInput, ResetDeletionMarksInput,
    RollbackResultSummary, RollbackToChapterInput, SnapshotDeltaSummary, SnapshotReferenceSummary,
    SnapshotSummary, UpdateCharacterProfileInput,
};
use a4n_character_mutation_worker_contract::{
    AttributeEventMutationResult, CHARACTER_ATTRIBUTE_ADD_OPERATION,
    CHARACTER_ATTRIBUTE_EXECUTE_DELETION_OPERATION, CHARACTER_ATTRIBUTE_MODIFY_OPERATION,
    CHARACTER_ATTRIBUTE_REQUEST_DELETION_OPERATION, CHARACTER_ATTRIBUTE_RESET_DELETION_OPERATION,
    CHARACTER_MUTATION_CREATE_OPERATION, CHARACTER_MUTATION_DELETE_OPERATION,
    CHARACTER_MUTATION_UPDATE_OPERATION, CHARACTER_MUTATION_WORKER_ID,
    CHARACTER_OVERVIEW_ARC_OPERATION, CHARACTER_OVERVIEW_LIST_OPERATION,
    CHARACTER_OVERVIEW_PROJECT_OPERATION, CHARACTER_OVERVIEW_VOICE_OPERATION,
    CHARACTER_TIMELINE_ATTRIBUTE_OPERATION, CHARACTER_TIMELINE_BEHAVIOR_OPERATION,
    CHARACTER_TIMELINE_DELETION_OPERATION, CHARACTER_TIMELINE_DIFF_OPERATION,
    CHARACTER_TIMELINE_ROLLBACK_OPERATION, CHARACTER_TIMELINE_SNAPSHOT_BY_HASH_OPERATION,
    CHARACTER_TIMELINE_SNAPSHOTS_OPERATION, CHARACTER_TIMELINE_STATE_OPERATION,
    CHARACTER_TIMELINE_SYNC_CONFLICT_OPERATION, CharacterTimelineRollbackMutationResult,
    CreateInput, NamedProfileInput, ProfileIdInput, UpdateInput,
};
use a4n_error::AppError;
use a4n_worker_client::{
    HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error, map_worker_response_error,
};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerLease, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47674";

#[derive(Clone, Debug)]
pub struct CharacterMutationWorkerClient {
    transport: HttpWorkerClient,
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
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_CHARACTER_MUTATION_WORKER_URL",
            "A4N_CHARACTER_MUTATION_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
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
        let project_id = a4n_worker_client::positive_id("project_id", project_id)?;
        self.command_with_project(user_id, Some(project_id), operation, payload)
            .await
    }

    async fn command_global<I, O>(
        &self,
        user_id: i64,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        self.command_with_project(user_id, None, operation, payload)
            .await
    }

    async fn command_with_project<I, O>(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let principal = a4n_worker_client::user_principal(user_id)?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal,
            operation_id: operation.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(CHARACTER_MUTATION_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "character mutation worker",
                    WorkerClientErrorPolicy::Configuration,
                )
            })?;
        a4n_worker_client::worker_response_payload(
            response,
            "character mutation worker",
            Some(operation),
            |error| worker_response_error(error, operation),
        )
    }
}

include!("reads.rs");

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
impl CharacterProfileTimelineMutationApplicationPort for CharacterMutationWorkerClient {
    async fn rollback(
        &self,
        user_id: i64,
        host_port: &dyn CharacterProfileTimelineMutationHostPort,
        input: RollbackToChapterInput,
    ) -> Result<RollbackResultSummary, AppError> {
        let result: CharacterTimelineRollbackMutationResult = self
            .command(
                user_id,
                input.profile_id,
                CHARACTER_TIMELINE_ROLLBACK_OPERATION,
                input,
            )
            .await?;
        let (deleted_state_rag, deleted_attribute_event_rag) = host_port
            .delete_timeline_rag(
                result.project_id,
                result.state_index_ids,
                result.attribute_event_ids,
            )
            .await?;
        Ok(RollbackResultSummary {
            deleted_events: result.deleted_events,
            deleted_snapshots: result.deleted_snapshots,
            deleted_behaviors: result.deleted_behaviors,
            deleted_state_indexes: result.deleted_state_indexes,
            deleted_state_rag,
            deleted_attribute_event_rag,
            deleted_deletion_marks: result.deleted_deletion_marks,
            rewound_deletion_marks: result.rewound_deletion_marks,
        })
    }
}

#[async_trait::async_trait]
impl CharacterProfileDeletionApplicationPort for CharacterMutationWorkerClient {
    async fn delete(
        &self,
        user_id: i64,
        host_port: &dyn CharacterProfileDeletionHostPort,
        profile_id: i64,
    ) -> Result<CharacterProfileDeleteResult, AppError> {
        let project_id = self.profile_project_id(user_id, profile_id).await?;
        let mut result: CharacterProfileDeleteResult = self
            .command(
                user_id,
                profile_id,
                CHARACTER_MUTATION_DELETE_OPERATION,
                ProfileIdInput { profile_id },
            )
            .await?;
        if let Err(error) = host_port.sync_profile_rag(project_id).await {
            result.warnings.push(format!(
                "角色资料检索索引未完全更新：{error}。正式资料未受影响，可稍后在项目操作中重新同步"
            ));
        }
        Ok(result)
    }
}

#[async_trait::async_trait]
impl CharacterProfileAttributeMutationApplicationPort for CharacterMutationWorkerClient {
    async fn add(
        &self,
        user_id: i64,
        host_port: &dyn CharacterAttributeRagSyncHostPort,
        input: AddProfileAttributeInput,
    ) -> Result<(), AppError> {
        let profile_id = input.profile_id;
        let project: AttributeEventMutationResult = self
            .command(
                user_id,
                profile_id,
                CHARACTER_ATTRIBUTE_ADD_OPERATION,
                input,
            )
            .await?;
        host_port
            .sync_attribute_event(project.project_id, project.event_id)
            .await
    }

    async fn modify(
        &self,
        user_id: i64,
        host_port: &dyn CharacterAttributeRagSyncHostPort,
        input: ModifyProfileAttributeInput,
    ) -> Result<(), AppError> {
        let profile_id = input.profile_id;
        let project: AttributeEventMutationResult = self
            .command(
                user_id,
                profile_id,
                CHARACTER_ATTRIBUTE_MODIFY_OPERATION,
                input,
            )
            .await?;
        host_port
            .sync_attribute_event(project.project_id, project.event_id)
            .await
    }
}

#[async_trait::async_trait]
impl CharacterProfileAttributeDeletionApplicationPort for CharacterMutationWorkerClient {
    async fn request(
        &self,
        user_id: i64,
        input: RequestAttributeDeletionInput,
    ) -> Result<DeletionMarkSummary, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_ATTRIBUTE_REQUEST_DELETION_OPERATION,
            input,
        )
        .await
    }

    async fn execute(
        &self,
        user_id: i64,
        host_port: &dyn CharacterAttributeRagSyncHostPort,
        input: ExecuteAttributeDeletionInput,
    ) -> Result<(), AppError> {
        let profile_id = input.profile_id;
        let project: AttributeEventMutationResult = self
            .command(
                user_id,
                profile_id,
                CHARACTER_ATTRIBUTE_EXECUTE_DELETION_OPERATION,
                input,
            )
            .await?;
        host_port
            .sync_attribute_event(project.project_id, project.event_id)
            .await
    }

    async fn reset(&self, user_id: i64, input: ResetDeletionMarksInput) -> Result<u64, AppError> {
        self.command(
            user_id,
            input.profile_id,
            CHARACTER_ATTRIBUTE_RESET_DELETION_OPERATION,
            input,
        )
        .await
    }
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    map_worker_response_error(error, "character mutation worker", operation)
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
