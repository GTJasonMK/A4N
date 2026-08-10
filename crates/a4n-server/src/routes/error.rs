use a4n_api_contract::generation_tasks::HostErrorResponse;
use a4n_core::error::AppError;
use a4n_service_protocol::GenerationErrorCode;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    retryable: bool,
    details: Value,
}

/// Generation 固定路由使用生成合同的规范 Host 错误体。
#[derive(Debug)]
pub struct GenerationApiError {
    status: StatusCode,
    body: HostErrorResponse,
}

impl GenerationApiError {
    pub fn new(
        status: StatusCode,
        code: &'static str,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            status,
            body: HostErrorResponse {
                code: GenerationErrorCode::registered(code)
                    .expect("generation route error code must be registered"),
                message: message.into(),
                retryable,
                trace_id: None,
            },
        }
    }

    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message, false)
    }

    pub fn unauthorized(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, message, false)
    }

    pub fn forbidden(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, code, message, false)
    }

    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message, false)
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, message, false)
    }

    pub fn gone(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::GONE, code, message, false)
    }

    pub fn payload_too_large(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::PAYLOAD_TOO_LARGE, code, message, false)
    }

    pub fn internal(code: &'static str, message: impl Into<String>, retryable: bool) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, code, message, retryable)
    }
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
    retryable: bool,
    trace_id: Option<String>,
    details: Value,
}

impl ApiError {
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "unauthorized",
            message: message.into(),
            retryable: false,
            details: json!({}),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_request",
            message: message.into(),
            retryable: false,
            details: json!({}),
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "forbidden",
            message: message.into(),
            retryable: false,
            details: json!({}),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: message.into(),
            retryable: false,
            details: json!({}),
        }
    }
}

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        let (status, code, retryable) = match &error {
            AppError::InvalidArgument(_) => (StatusCode::BAD_REQUEST, "invalid_argument", false),
            AppError::InvalidState { .. } => (StatusCode::CONFLICT, "invalid_state", false),
            AppError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized", false),
            AppError::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden", false),
            AppError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found", false),
            AppError::ConfigError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "config_error", false),
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "database_error", true),
            AppError::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, "timeout", true),
            AppError::Cancelled => (StatusCode::CONFLICT, "cancelled", false),
            AppError::Internal(_)
            | AppError::Io(_)
            | AppError::Serialization(_)
            | AppError::LlmError(_)
            | AppError::VectorStoreError(_)
            | AppError::EmbeddingError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", false)
            }
        };
        Self {
            status,
            code,
            message: error.to_string(),
            retryable,
            details: json!({}),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                code: self.code,
                message: self.message,
                retryable: self.retryable,
                trace_id: None,
                details: self.details,
            }),
        )
            .into_response()
    }
}

impl IntoResponse for GenerationApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

#[cfg(test)]
#[tokio::test]
async fn api_error_serializes_only_the_canonical_message() {
    let response = ApiError::bad_request("failed").into_response();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["message"], "failed");
    assert!(value.get("error").is_none());
}
