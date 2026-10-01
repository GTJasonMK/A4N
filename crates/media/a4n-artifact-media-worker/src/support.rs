use super::*;
use std::collections::{HashMap, HashSet};

use a4n_api_contract::image::GeneratedImageSummary;
use a4n_domain::media::ImagePublicationRecoveryRecord;
use a4n_worker_contract::{WorkerError, WorkerOperationStatus};

pub(super) fn parse_payload<T: serde::de::DeserializeOwned>(payload: Value) -> Result<T, String> {
    serde_json::from_value(payload).map_err(|error| format!("请求 payload 无效: {error}"))
}

pub(super) fn actor_user_id(request: &WorkerEnvelope<Value>) -> Result<i64, String> {
    request
        .principal
        .subject
        .parse::<i64>()
        .ok()
        .filter(|user_id| *user_id > 0)
        .ok_or_else(|| "user principal subject must be a positive integer".to_string())
}

pub(super) fn completed_response(
    request: &WorkerEnvelope<Value>,
    payload: Value,
) -> WorkerResponse<Value> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: request.operation_ref(),
        status: WorkerOperationStatus::Completed,
        payload: Some(payload),
        resource_refs: Vec::new(),
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: None,
    }
}

pub(super) fn failed_response(
    request: &WorkerEnvelope<Value>,
    code: WorkerErrorCode,
    message: impl Into<String>,
) -> WorkerResponse<Value> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: request.operation_ref(),
        status: WorkerOperationStatus::Failed,
        payload: None,
        resource_refs: Vec::new(),
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: Some(WorkerError {
            code,
            message: message.into(),
            retryable: matches!(
                code,
                WorkerErrorCode::Unavailable | WorkerErrorCode::Timeout
            ),
        }),
    }
}

pub(super) fn classify_error(error: &str) -> WorkerErrorCode {
    if error.contains("无权") || error.contains("权限") || error.contains("unauthorized") {
        WorkerErrorCode::Unauthorized
    } else if error.contains("不存在") || error.contains("not found") {
        WorkerErrorCode::NotFound
    } else if error.contains("版本") || error.contains("状态") || error.contains("不一致") {
        WorkerErrorCode::Conflict
    } else if error.contains("数据库") || error.contains("database") {
        WorkerErrorCode::Unavailable
    } else {
        WorkerErrorCode::Internal
    }
}

pub(super) fn authorized(state: &WorkerState, headers: &HeaderMap) -> bool {
    headers
        .get(AUTH_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == state.token.as_ref())
}

pub(super) fn generated_image_summary(
    image: &a4n_db_models::models::ai::generated_image::Model,
) -> GeneratedImageSummary {
    GeneratedImageSummary {
        id: image.id,
        project_id: image.project_id,
        image_type: image.image_type.clone(),
        file_path: image.file_path.clone(),
        prompt: image.prompt.clone(),
        model: image.model.clone(),
        width: image.width,
        height: image.height,
        file_size: image.file_size,
        ai_score: image.ai_score,
        is_favorite: image.is_favorite,
        is_system_managed: a4n_domain::image::is_system_managed_image_type(&image.image_type),
        chapter_id: image.chapter_id,
        panel_id: image.panel_id.clone(),
        metadata_json: image.metadata_json.clone(),
        created_at: image.created_at.and_utc().to_rfc3339(),
    }
}

pub(super) fn normalized_project_ids(project_ids: Vec<i64>) -> Result<Vec<i64>, String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(project_ids.len());
    for project_id in project_ids {
        if project_id <= 0 {
            return Err("project_ids 必须全部大于 0".to_string());
        }
        if seen.insert(project_id) {
            normalized.push(project_id);
        }
    }
    if normalized.is_empty() {
        return Err("project_ids 不能为空".to_string());
    }
    Ok(normalized)
}

pub(super) fn sort_images_by_project_order(
    images: &mut [a4n_db_models::models::ai::generated_image::Model],
    project_ids: &[i64],
) {
    let project_order = project_ids
        .iter()
        .enumerate()
        .map(|(index, project_id)| (*project_id, index))
        .collect::<HashMap<_, _>>();
    images.sort_by(|left, right| {
        project_order[&left.project_id]
            .cmp(&project_order[&right.project_id])
            .then_with(|| right.created_at.cmp(&left.created_at))
            .then_with(|| right.id.cmp(&left.id))
    });
}

pub(super) async fn verify_image_permission(
    database: &a4n_db::DbHandle,
    user_id: i64,
    project_id: i64,
    role: &str,
) -> Result<(), String> {
    if user_id <= 0 || project_id <= 0 {
        return Err("user_id 和 project_id must be positive".to_string());
    }
    let required = match role {
        "viewer" => a4n_domain::collaboration::ProjectAccessRole::Viewer,
        "editor" => a4n_domain::collaboration::ProjectAccessRole::Editor,
        _ => return Err(format!("unsupported image permission role: {role}")),
    };
    a4n_db::collaboration::verify_project_permission(
        database.connection(),
        project_id,
        user_id,
        required,
    )
    .await
    .map(|_| ())
    .map_err(|error| error.to_string())
}

pub(super) fn recovery_record(
    publication: a4n_db_models::models::media::image_publication::Model,
) -> ImagePublicationRecoveryRecord {
    ImagePublicationRecoveryRecord {
        publication_id: publication.publication_id,
        project_id: publication.project_id,
        requested_by_user_id: publication.requested_by_user_id,
        staging_path: publication.staging_path,
        final_path: publication.final_path,
        image_type: publication.image_type,
        model: publication.model,
        expected_file_size: publication.expected_file_size,
        expected_sha256: publication.expected_sha256,
        file_size: publication.file_size,
        status: publication.status,
        version: publication.version,
        image_id: publication.image_id,
    }
}

pub(super) fn facts_match(
    facts: &(PathBuf, i64, String),
    publication: &ImagePublicationRecoveryRecord,
) -> bool {
    facts.1 == publication.expected_file_size && facts.2 == publication.expected_sha256
}
