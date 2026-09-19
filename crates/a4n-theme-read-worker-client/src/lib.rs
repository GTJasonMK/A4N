//! Typed Host adapter for the theme read worker.

use a4n_api_contract::theme::{
    CreateThemeInput, PersistThemeResourceInput, ThemeMutationApplicationPort,
    ThemePreferenceSummary, ThemeReadApplicationPort, ThemeResourceRecord, ThemeSummary,
    UpdateThemeInput, UpdateThemePreferenceInput,
};
use a4n_error::AppError;
use a4n_theme_read_worker_contract::{
    ResourceIdInput, THEME_ACTIVATE_OPERATION, THEME_CREATE_OPERATION, THEME_DELETE_OPERATION,
    THEME_EXPORT_OPERATION, THEME_IMPORT_OPERATION, THEME_LIST_OPERATION,
    THEME_PREFERENCE_OPERATION, THEME_PREFERENCE_UPDATE_OPERATION, THEME_READ_WORKER_ID,
    THEME_RESOURCE_DELETE_OPERATION, THEME_RESOURCE_UPSERT_OPERATION, THEME_RESOURCES_OPERATION,
    THEME_UPDATE_OPERATION, ThemeIdInput, ThemeUpdateInput, ThemeUserInput,
};
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47663";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct ThemeReadWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for ThemeReadWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ThemeReadWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl ThemeReadWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, THEME_READ_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_THEME_READ_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_THEME_READ_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<O>(
        &self,
        user_id: i64,
        operation_id: &str,
        payload: impl serde::Serialize,
    ) -> Result<O, AppError>
    where
        O: serde::de::DeserializeOwned,
    {
        if user_id <= 0 {
            return Err(AppError::InvalidArgument("user_id 必须大于 0".into()));
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
            operation_id: operation_id.to_string(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: THEME_READ_WORKER_ID.into(),
                epoch: 1,
                fence_token: CONTROL_LEASE_FENCE.into(),
                expires_at: CONTROL_LEASE_EXPIRY.into(),
            },
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation_id, request)
            .await
            .map_err(worker_client_error)?;
        match response.status {
            WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
                AppError::ConfigError(format!(
                    "theme read worker 完成响应缺少 payload: {operation_id}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation_id,
            )),
            status => Err(AppError::ConfigError(format!(
                "theme read worker 返回未完成状态: operation={operation_id}, status={status:?}"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl ThemeReadApplicationPort for ThemeReadWorkerClient {
    async fn list_themes(&self, user_id: i64) -> Result<Vec<ThemeSummary>, AppError> {
        self.command(user_id, THEME_LIST_OPERATION, ThemeUserInput {})
            .await
    }

    async fn export_theme(&self, user_id: i64, theme_id: i64) -> Result<ThemeSummary, AppError> {
        self.command(user_id, THEME_EXPORT_OPERATION, ThemeIdInput { theme_id })
            .await
    }

    async fn list_resources(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<Vec<ThemeResourceRecord>, AppError> {
        self.command(
            user_id,
            THEME_RESOURCES_OPERATION,
            ThemeIdInput { theme_id },
        )
        .await
    }

    async fn get_preference(&self, user_id: i64) -> Result<ThemePreferenceSummary, AppError> {
        self.command(user_id, THEME_PREFERENCE_OPERATION, ThemeUserInput {})
            .await
    }
}

#[async_trait::async_trait]
impl ThemeMutationApplicationPort for ThemeReadWorkerClient {
    async fn create_theme(
        &self,
        user_id: i64,
        input: CreateThemeInput,
    ) -> Result<ThemeSummary, AppError> {
        self.command(user_id, THEME_CREATE_OPERATION, input).await
    }

    async fn update_theme(
        &self,
        user_id: i64,
        theme_id: i64,
        input: UpdateThemeInput,
    ) -> Result<ThemeSummary, AppError> {
        self.command(
            user_id,
            THEME_UPDATE_OPERATION,
            ThemeUpdateInput { theme_id, input },
        )
        .await
    }

    async fn delete_theme(&self, user_id: i64, theme_id: i64) -> Result<(), AppError> {
        let _: serde_json::Value = self
            .command(user_id, THEME_DELETE_OPERATION, ThemeIdInput { theme_id })
            .await?;
        Ok(())
    }

    async fn activate_theme(&self, user_id: i64, theme_id: i64) -> Result<ThemeSummary, AppError> {
        self.command(user_id, THEME_ACTIVATE_OPERATION, ThemeIdInput { theme_id })
            .await
    }

    async fn import_theme(
        &self,
        user_id: i64,
        input: CreateThemeInput,
    ) -> Result<ThemeSummary, AppError> {
        self.command(user_id, THEME_IMPORT_OPERATION, input).await
    }

    async fn upsert_resource(
        &self,
        user_id: i64,
        input: PersistThemeResourceInput,
    ) -> Result<ThemeResourceRecord, AppError> {
        self.command(user_id, THEME_RESOURCE_UPSERT_OPERATION, input)
            .await
    }

    async fn delete_resource(
        &self,
        user_id: i64,
        resource_id: i64,
    ) -> Result<ThemeResourceRecord, AppError> {
        self.command(
            user_id,
            THEME_RESOURCE_DELETE_OPERATION,
            ResourceIdInput { resource_id },
        )
        .await
    }

    async fn update_preference(
        &self,
        user_id: i64,
        input: UpdateThemePreferenceInput,
    ) -> Result<ThemePreferenceSummary, AppError> {
        self.command(user_id, THEME_PREFERENCE_UPDATE_OPERATION, input)
            .await
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("theme read worker 请求失败: {error}"))
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict => AppError::InvalidState {
            current: error.message.clone(),
            operation: operation.to_string(),
        },
        _ => AppError::ConfigError(format!(
            "theme read worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = ThemeReadWorkerClient::new(
            "https://theme-read-worker.example.test",
            Some("theme-read-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("theme-read-worker.example.test"));
        assert!(!debug.contains("theme-read-token"));
    }
}
