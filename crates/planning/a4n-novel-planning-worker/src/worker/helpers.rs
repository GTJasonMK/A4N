async fn verify_family_permission(
    db: &DatabaseConnection,
    project_id: i64,
    user_id: i64,
    required: ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    let (project_id, root_project_id) =
        a4n_db::project::project_or_derived_family_access_ids(db, project_id)
            .await
            .map_err(project_family_error)?;
    if let Some(root_project_id) = root_project_id {
        a4n_db::collaboration::verify_project_permissions(
            db,
            &[project_id, root_project_id],
            user_id,
            required,
        )
        .await
        .map_err(collaboration_error)?;
    } else {
        a4n_db::collaboration::verify_project_permission(db, project_id, user_id, required)
            .await
            .map_err(collaboration_error)?;
    }
    Ok(())
}

fn validate_request<T>(request: &WorkerEnvelope<T>, operation: &str) -> Result<(), WorkerFailure> {
    request
        .validate()
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
    if request.operation_id != operation {
        return Err(WorkerFailure::invalid_request(format!(
            "operation_id 不匹配: expected={operation}, actual={}",
            request.operation_id
        )));
    }
    if request.lease.worker_id != NOVEL_PLANNING_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 novel-planning worker",
        ));
    }
    Ok(())
}

fn validate_project_scope<T>(
    request: &WorkerEnvelope<T>,
    project_id: i64,
) -> Result<(), WorkerFailure> {
    if request.project_id != Some(project_id) {
        return Err(WorkerFailure::invalid_request(format!(
            "worker scope project_id 与 payload 不一致: scope={:?}, payload={project_id}",
            request.project_id
        )));
    }
    Ok(())
}

fn validate_target_revision<T>(
    request: &WorkerEnvelope<T>,
    expected_revision: i32,
) -> Result<(), WorkerFailure> {
    if request.target_revision != Some(expected_revision as u64) {
        return Err(WorkerFailure::invalid_request(format!(
            "worker target_revision 与 payload 不一致: target={:?}, payload={expected_revision}",
            request.target_revision
        )));
    }
    Ok(())
}

fn validate_optional_target_revision<T>(
    request: &WorkerEnvelope<T>,
    expected_revision: Option<i32>,
) -> Result<(), WorkerFailure> {
    match expected_revision {
        Some(expected_revision) => validate_target_revision(request, expected_revision),
        None if request.target_revision.is_some() => Err(WorkerFailure::invalid_request(
            "新建拍摄剧本不能携带 target_revision",
        )),
        None => Ok(()),
    }
}

fn user_id<T>(request: &WorkerEnvelope<T>) -> Result<i64, WorkerFailure> {
    if request.principal.kind != a4n_worker_contract::WorkerPrincipalKind::User {
        return Err(WorkerFailure::unauthorized("仅允许用户 principal"));
    }
    let user_id = request
        .principal
        .subject
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| WorkerFailure::unauthorized("用户 principal subject 无效"))?;
    Ok(user_id)
}

async fn resolve_source(
    state: &WorkerState,
    project_id: i64,
    source_chapter_id: i64,
) -> Result<a4n_domain::chapter::ResolvedChapterSource, WorkerFailure> {
    a4n_db::project::resolve_video_chapter_source(
        state.database.connection(),
        project_id,
        source_chapter_id,
    )
    .await
    .map_err(source_error)
}

fn completed<T>(request: &WorkerEnvelope<impl serde::Serialize>, payload: T) -> WorkerResponse<T> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: WorkerOperationRef {
            operation_id: request.operation_id.clone(),
            task_id: None,
        },
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

fn failed<T>(
    request: &WorkerEnvelope<impl serde::Serialize>,
    failure: WorkerFailure,
) -> WorkerResponse<T> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: WorkerOperationRef {
            operation_id: request.operation_id.clone(),
            task_id: None,
        },
        status: WorkerOperationStatus::Failed,
        payload: None,
        resource_refs: Vec::new(),
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: Some(WorkerError {
            code: failure.code,
            message: failure.message,
            retryable: failure.retryable,
        }),
    }
}

impl WorkerFailure {
    fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::InvalidRequest,
            message: message.into(),
            retryable: false,
        }
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Unauthorized,
            message: message.into(),
            retryable: false,
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::NotFound,
            message: message.into(),
            retryable: false,
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Conflict,
            message: message.into(),
            retryable: false,
        }
    }

    fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Unavailable,
            message: message.into(),
            retryable: true,
        }
    }
}

fn source_error(error: a4n_db::project::ResolveChapterSourceError) -> WorkerFailure {
    match error {
        a4n_db::project::ResolveChapterSourceError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ResolveChapterSourceError::InvalidState(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::project::ResolveChapterSourceError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::project::ResolveChapterSourceError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn project_family_error(error: a4n_db::project::ProjectFamilyDbError) -> WorkerFailure {
    match error {
        a4n_db::project::ProjectFamilyDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::project::ProjectFamilyDbError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::project::ProjectFamilyDbError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn collaboration_error(error: a4n_db::collaboration::CollaborationDbError) -> WorkerFailure {
    match error {
        a4n_db::collaboration::CollaborationDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::collaboration::CollaborationDbError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::collaboration::CollaborationDbError::Forbidden(message) => {
            WorkerFailure::unauthorized(message)
        }
        a4n_db::collaboration::CollaborationDbError::InvalidState(message)
        | a4n_db::collaboration::CollaborationDbError::Internal(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::collaboration::CollaborationDbError::Database(error) => {
            WorkerFailure::unavailable(error.to_string())
        }
    }
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "novel-planning worker authentication required",
    )
        .into_response()
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
