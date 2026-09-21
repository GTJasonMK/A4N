//! Typed Host adapter for the object profile worker.

use a4n_api_contract::profile::{
    CreateObjectProfileInput, DeleteObjectProfileInput, ObjectProfileReadApplicationPort,
    ObjectProfileSummary, UpdateObjectProfileInput,
};
use a4n_error::AppError;
use a4n_object_profile_worker_contract::{
    OBJECT_PROFILE_CREATE_OPERATION, OBJECT_PROFILE_DELETE_OPERATION,
    OBJECT_PROFILE_LIST_OPERATION, OBJECT_PROFILE_UPDATE_OPERATION, OBJECT_PROFILE_WORKER_ID,
    ObjectProfileDeleteResponse, ObjectProfileMutationResponse, ObjectProfileProjectInput,
};
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47657";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct ObjectProfileWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for ObjectProfileWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ObjectProfileWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl ObjectProfileWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, OBJECT_PROFILE_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_OBJECT_PROFILE_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_OBJECT_PROFILE_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        operation_id: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        if user_id <= 0 || project_id.is_some_and(|id| id <= 0) {
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
            operation_id: operation_id.to_string(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: OBJECT_PROFILE_WORKER_ID.into(),
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
                    "object profile worker 完成响应缺少 payload: {operation_id}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation_id,
            )),
            status => Err(AppError::ConfigError(format!(
                "object profile worker 返回未完成状态: operation={operation_id}, status={status:?}"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl ObjectProfileReadApplicationPort for ObjectProfileWorkerClient {
    async fn list(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ObjectProfileSummary>, AppError> {
        self.command(
            user_id,
            Some(project_id),
            OBJECT_PROFILE_LIST_OPERATION,
            ObjectProfileProjectInput { project_id },
        )
        .await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::profile::ObjectProfileMutationApplicationPort for ObjectProfileWorkerClient {
    async fn create(
        &self,
        user_id: i64,
        input: CreateObjectProfileInput,
    ) -> Result<a4n_api_contract::profile::ObjectProfileMutationOutcome, AppError> {
        let response: ObjectProfileMutationResponse = self
            .command(
                user_id,
                Some(input.project_id),
                OBJECT_PROFILE_CREATE_OPERATION,
                input,
            )
            .await?;
        Ok(a4n_api_contract::profile::ObjectProfileMutationOutcome {
            result: response.result,
            downstream_image_ids: response.downstream_image_ids,
        })
    }

    async fn update(
        &self,
        user_id: i64,
        input: UpdateObjectProfileInput,
    ) -> Result<a4n_api_contract::profile::ObjectProfileMutationOutcome, AppError> {
        let response: ObjectProfileMutationResponse = self
            .command(user_id, None, OBJECT_PROFILE_UPDATE_OPERATION, input)
            .await?;
        Ok(a4n_api_contract::profile::ObjectProfileMutationOutcome {
            result: response.result,
            downstream_image_ids: response.downstream_image_ids,
        })
    }

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteObjectProfileInput,
    ) -> Result<a4n_api_contract::profile::ObjectProfileDeleteOutcome, AppError> {
        let response: ObjectProfileDeleteResponse = self
            .command(user_id, None, OBJECT_PROFILE_DELETE_OPERATION, input)
            .await?;
        Ok(a4n_api_contract::profile::ObjectProfileDeleteOutcome {
            result: response.result,
            downstream_image_ids: response.downstream_image_ids,
        })
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("object profile worker 请求失败: {error}"))
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
            "object profile worker {operation} 失败: {}",
            error.message
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = ObjectProfileWorkerClient::new(
            "https://object-profile-worker.example.test",
            Some("object-profile-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("object-profile-worker.example.test"));
        assert!(!debug.contains("object-profile-token"));
    }
}
