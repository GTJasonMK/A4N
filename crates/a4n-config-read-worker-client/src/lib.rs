//! Typed Host adapter for provider configuration reads.

use a4n_api_contract::config::{
    ConfigMutationApplicationPort, ConfigProviderKind, ConfigReadApplicationPort,
    CreateEmbeddingInput, CreateLlmInput, EmbeddingConfigSummary, ImageConfigForConnectivityTest,
    ImageConfigInput, ImageConfigSummary, LlmConfigSummary, UpdateEmbeddingInput,
    UpdateImageConfigInput, UpdateLlmInput, UpdateVideoConfigInput, VideoConfigInput,
    VideoConfigSummary,
};
use a4n_config_read_worker_contract::{
    CONFIG_MUTATION_BASELINE_EMBEDDING_OPERATION, CONFIG_MUTATION_BASELINE_IMAGE_OPERATION,
    CONFIG_MUTATION_BASELINE_LLM_OPERATION, CONFIG_MUTATION_BASELINE_VIDEO_OPERATION,
    CONFIG_MUTATION_CREATE_EMBEDDING_OPERATION, CONFIG_MUTATION_CREATE_IMAGE_OPERATION,
    CONFIG_MUTATION_CREATE_LLM_OPERATION, CONFIG_MUTATION_CREATE_VIDEO_OPERATION,
    CONFIG_MUTATION_DELETE_EMBEDDING_OPERATION, CONFIG_MUTATION_DELETE_IMAGE_OPERATION,
    CONFIG_MUTATION_DELETE_LLM_OPERATION, CONFIG_MUTATION_DELETE_VIDEO_OPERATION,
    CONFIG_MUTATION_SET_DEFAULT_EMBEDDING_OPERATION, CONFIG_MUTATION_SET_DEFAULT_IMAGE_OPERATION,
    CONFIG_MUTATION_SET_DEFAULT_LLM_OPERATION, CONFIG_MUTATION_SET_DEFAULT_VIDEO_OPERATION,
    CONFIG_MUTATION_UPDATE_EMBEDDING_OPERATION, CONFIG_MUTATION_UPDATE_IMAGE_OPERATION,
    CONFIG_MUTATION_UPDATE_LLM_OPERATION, CONFIG_MUTATION_UPDATE_VIDEO_OPERATION,
    CONFIG_READ_EMBEDDING_CONNECTIVITY_OPERATION, CONFIG_READ_EMBEDDING_LIST_OPERATION,
    CONFIG_READ_IMAGE_CONNECTIVITY_OPERATION, CONFIG_READ_IMAGE_EXISTS_OPERATION,
    CONFIG_READ_IMAGE_LIST_OPERATION, CONFIG_READ_LLM_CONNECTIVITY_OPERATION,
    CONFIG_READ_LLM_EXISTS_OPERATION, CONFIG_READ_LLM_LIST_OPERATION,
    CONFIG_READ_SAVED_API_KEY_OPERATION, CONFIG_READ_VIDEO_EXISTS_OPERATION,
    CONFIG_READ_VIDEO_LIST_OPERATION, CONFIG_READ_WORKER_ID, ConfigIdInput, ConfigProviderKindWire,
    CreateEmbeddingCommand, CreateImageCommand, CreateLlmCommand, CreateVideoCommand, EmptyInput,
    SavedApiKeyInput, UpdateEmbeddingCommand, UpdateImageCommand, UpdateLlmCommand,
    UpdateVideoCommand,
};
use a4n_error::AppError;
use a4n_provider_contract::{EmbeddingProviderConfig, LlmProviderConfig};
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47673";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct ConfigReadWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for ConfigReadWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ConfigReadWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl ConfigReadWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, CONFIG_READ_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_CONFIG_READ_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_CONFIG_READ_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        principal: WorkerPrincipal,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal,
            operation_id: operation.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: CONFIG_READ_WORKER_ID.into(),
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
                    "config read worker 完成响应缺少 payload: {operation}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation,
            )),
            status => Err(AppError::ConfigError(format!(
                "config read worker 返回未完成状态: operation={operation}, status={status:?}"
            ))),
        }
    }

    fn user(user_id: i64) -> Result<WorkerPrincipal, AppError> {
        if user_id <= 0 {
            return Err(AppError::InvalidArgument("user_id 必须大于 0".into()));
        }
        Ok(WorkerPrincipal {
            kind: WorkerPrincipalKind::User,
            subject: user_id.to_string(),
        })
    }

    fn service() -> WorkerPrincipal {
        WorkerPrincipal {
            kind: WorkerPrincipalKind::Service,
            subject: "runtime".into(),
        }
    }

    fn kind(kind: ConfigProviderKind) -> ConfigProviderKindWire {
        match kind {
            ConfigProviderKind::Llm => ConfigProviderKindWire::Llm,
            ConfigProviderKind::Image => ConfigProviderKindWire::Image,
            ConfigProviderKind::Video => ConfigProviderKindWire::Video,
        }
    }
}

#[async_trait::async_trait]
impl ConfigReadApplicationPort for ConfigReadWorkerClient {
    async fn llm_configs_exist(&self) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_LLM_EXISTS_OPERATION,
            EmptyInput {},
        )
        .await
    }
    async fn image_configs_exist(&self) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_IMAGE_EXISTS_OPERATION,
            EmptyInput {},
        )
        .await
    }
    async fn video_configs_exist(&self) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_VIDEO_EXISTS_OPERATION,
            EmptyInput {},
        )
        .await
    }
    async fn saved_api_key(
        &self,
        caller_user_id: i64,
        kind: ConfigProviderKind,
        config_id: i64,
    ) -> Result<Option<String>, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_READ_SAVED_API_KEY_OPERATION,
            SavedApiKeyInput {
                kind: Self::kind(kind),
                config_id,
            },
        )
        .await
    }
    async fn llm_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<LlmProviderConfig, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_READ_LLM_CONNECTIVITY_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }
    async fn embedding_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<EmbeddingProviderConfig, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_READ_EMBEDDING_CONNECTIVITY_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }
    async fn image_config_for_connectivity_test(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<ImageConfigForConnectivityTest, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_READ_IMAGE_CONNECTIVITY_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }
    async fn list_llm(&self) -> Result<Vec<LlmConfigSummary>, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_LLM_LIST_OPERATION,
            EmptyInput {},
        )
        .await
    }
    async fn list_image(&self) -> Result<Vec<ImageConfigSummary>, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_IMAGE_LIST_OPERATION,
            EmptyInput {},
        )
        .await
    }
    async fn list_video(&self) -> Result<Vec<VideoConfigSummary>, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_VIDEO_LIST_OPERATION,
            EmptyInput {},
        )
        .await
    }
    async fn list_embedding(&self) -> Result<Vec<EmbeddingConfigSummary>, AppError> {
        self.command(
            Self::service(),
            CONFIG_READ_EMBEDDING_LIST_OPERATION,
            EmptyInput {},
        )
        .await
    }
}

#[async_trait::async_trait]
impl ConfigMutationApplicationPort for ConfigReadWorkerClient {
    async fn insert_baseline_embedding_if_empty(&self) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_MUTATION_BASELINE_EMBEDDING_OPERATION,
            EmptyInput {},
        )
        .await
    }

    async fn insert_baseline_llm_if_empty(&self, input: CreateLlmInput) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_MUTATION_BASELINE_LLM_OPERATION,
            CreateLlmCommand(input),
        )
        .await
    }

    async fn insert_baseline_image_if_empty(
        &self,
        input: ImageConfigInput,
    ) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_MUTATION_BASELINE_IMAGE_OPERATION,
            CreateImageCommand(input),
        )
        .await
    }

    async fn insert_baseline_video_if_empty(
        &self,
        input: VideoConfigInput,
    ) -> Result<bool, AppError> {
        self.command(
            Self::service(),
            CONFIG_MUTATION_BASELINE_VIDEO_OPERATION,
            CreateVideoCommand(input),
        )
        .await
    }

    async fn create_llm(
        &self,
        caller_user_id: i64,
        input: CreateLlmInput,
    ) -> Result<LlmConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_CREATE_LLM_OPERATION,
            CreateLlmCommand(input),
        )
        .await
    }

    async fn update_llm(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateLlmInput,
    ) -> Result<LlmConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_UPDATE_LLM_OPERATION,
            UpdateLlmCommand { config_id, input },
        )
        .await
    }

    async fn set_default_llm(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_SET_DEFAULT_LLM_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn delete_llm(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_DELETE_LLM_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn create_image(
        &self,
        caller_user_id: i64,
        input: ImageConfigInput,
    ) -> Result<ImageConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_CREATE_IMAGE_OPERATION,
            CreateImageCommand(input),
        )
        .await
    }

    async fn update_image(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateImageConfigInput,
    ) -> Result<ImageConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_UPDATE_IMAGE_OPERATION,
            UpdateImageCommand { config_id, input },
        )
        .await
    }

    async fn set_default_image(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_SET_DEFAULT_IMAGE_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn delete_image(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_DELETE_IMAGE_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn create_video(
        &self,
        caller_user_id: i64,
        input: VideoConfigInput,
    ) -> Result<VideoConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_CREATE_VIDEO_OPERATION,
            CreateVideoCommand(input),
        )
        .await
    }

    async fn update_video(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateVideoConfigInput,
    ) -> Result<VideoConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_UPDATE_VIDEO_OPERATION,
            UpdateVideoCommand { config_id, input },
        )
        .await
    }

    async fn set_default_video(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_SET_DEFAULT_VIDEO_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn delete_video(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_DELETE_VIDEO_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn create_embedding(
        &self,
        caller_user_id: i64,
        input: CreateEmbeddingInput,
    ) -> Result<EmbeddingConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_CREATE_EMBEDDING_OPERATION,
            CreateEmbeddingCommand(input),
        )
        .await
    }

    async fn update_embedding(
        &self,
        caller_user_id: i64,
        config_id: i64,
        input: UpdateEmbeddingInput,
    ) -> Result<EmbeddingConfigSummary, AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_UPDATE_EMBEDDING_OPERATION,
            UpdateEmbeddingCommand { config_id, input },
        )
        .await
    }

    async fn set_default_embedding(
        &self,
        caller_user_id: i64,
        config_id: i64,
    ) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_SET_DEFAULT_EMBEDDING_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }

    async fn delete_embedding(&self, caller_user_id: i64, config_id: i64) -> Result<(), AppError> {
        self.command(
            Self::user(caller_user_id)?,
            CONFIG_MUTATION_DELETE_EMBEDDING_OPERATION,
            ConfigIdInput { config_id },
        )
        .await
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("config read worker 请求失败: {error}"))
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
            "config read worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = ConfigReadWorkerClient::new(
            "https://config-read-worker.example.test",
            Some("config-read-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("config-read-worker.example.test"));
        assert!(!debug.contains("config-read-token"));
    }
}
