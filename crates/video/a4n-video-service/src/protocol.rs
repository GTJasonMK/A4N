use a4n_domain::ProviderError as AppError;
use a4n_service_protocol::{
    SecretMaterialScope, ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse,
    VIDEO_SERVICE_NAME, redact_sensitive_message,
};
use a4n_video_provider_contract::validate_video_provider_idempotency_key;
use a4n_video_sidecar_contract::{VideoPollPayload, VideoProviderPayload};

pub(crate) fn result_response<T>(
    request: &ServiceRequest<impl Sized>,
    result: Result<T, AppError>,
) -> ServiceResponse<T> {
    request.result_response(result, service_error)
}

pub(crate) fn validate_video_provider_request(
    request: &ServiceRequest<VideoProviderPayload>,
) -> Result<(), ServiceError> {
    request.validate_known_provider_request(VIDEO_SERVICE_NAME, Some(&request.payload.api_key))?;
    request
        .payload
        .request
        .validate_dimensions()
        .map_err(|error| {
            ServiceError::non_retryable(ServiceErrorCode::ConfigurationError, error.to_string())
        })?;
    validate_project_scope(request.payload.project_id, request.project_id, "generation")?;
    for artifact in request.payload.request.media.artifacts() {
        if request.secret_policy.scope != SecretMaterialScope::LocalSidecarOnly
            || request.user_id.is_none()
            || artifact.owner_user_id != request.user_id
            || (artifact.project_id != request.project_id
                && artifact.project_id != request.root_project_id)
        {
            return Err(ServiceError::non_retryable(
                ServiceErrorCode::Unauthorized,
                "本地视频图片引用必须属于当前用户和项目范围，且只能发送给本地 sidecar",
            ));
        }
    }
    request
        .payload
        .request
        .media
        .validate_artifacts()
        .map_err(|error| {
            ServiceError::non_retryable(ServiceErrorCode::ConfigurationError, error)
        })?;
    validate_video_provider_idempotency_key(
        request.payload.provider_idempotency_key_sha256.as_deref(),
    )
    .map_err(|error| {
        ServiceError::non_retryable(ServiceErrorCode::ConfigurationError, error.to_string())
    })?;
    Ok(())
}

pub(crate) fn validate_video_poll_request(
    request: &ServiceRequest<VideoPollPayload>,
) -> Result<(), ServiceError> {
    request.validate_known_provider_request(VIDEO_SERVICE_NAME, Some(&request.payload.api_key))?;
    validate_project_scope(
        request.payload.project_id,
        request.project_id,
        "poll/status",
    )
}

fn validate_project_scope(
    payload_project_id: Option<i64>,
    envelope_project_id: Option<i64>,
    operation: &str,
) -> Result<(), ServiceError> {
    if payload_project_id.is_some_and(|project_id| project_id <= 0)
        || payload_project_id != envelope_project_id
    {
        return Err(ServiceError::non_retryable(
            ServiceErrorCode::ConfigurationError,
            format!("video {operation} payload project_id 必须与 request envelope 一致且为正整数"),
        ));
    }
    Ok(())
}

fn service_error_code(err: &AppError) -> ServiceErrorCode {
    match err {
        AppError::InvalidArgument(_) | AppError::Configuration(_) => {
            ServiceErrorCode::ConfigurationError
        }
        AppError::Cancelled => ServiceErrorCode::Cancelled,
        AppError::Timeout(_) => ServiceErrorCode::ServiceUnavailable,
        _ => ServiceErrorCode::ProviderError,
    }
}

pub(crate) fn service_error(err: AppError) -> ServiceError {
    ServiceError::non_retryable(
        service_error_code(&err),
        redact_sensitive_message(err.to_string()),
    )
}
