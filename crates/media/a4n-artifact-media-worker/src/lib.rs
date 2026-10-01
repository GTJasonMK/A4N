use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use a4n_api_contract::image::{
    DeleteImageInput, ListImagesByChapterInput, ListImagesByProjectInput,
    ListImagesByProjectsInput, UpdateGeneratedImageInput,
};
use a4n_artifact_media_contract::{
    ARTIFACT_MEDIA_CAPABILITY_VERSION, ARTIFACT_MEDIA_IMAGE_BY_ID_COMMAND,
    ARTIFACT_MEDIA_IMAGE_DELETE_COMMAND, ARTIFACT_MEDIA_IMAGE_LIST_CHAPTER_COMMAND,
    ARTIFACT_MEDIA_IMAGE_LIST_PROJECT_COMMAND, ARTIFACT_MEDIA_IMAGE_LIST_PROJECTS_COMMAND,
    ARTIFACT_MEDIA_IMAGE_REFERENCE_IDS_COMMAND, ARTIFACT_MEDIA_IMAGE_UPDATE_COMMAND,
    ARTIFACT_MEDIA_MAINTENANCE_COMMAND, ARTIFACT_MEDIA_PATH_PREFIX_REFERENCED_COMMAND,
    ARTIFACT_MEDIA_PROJECT_PUBLICATION_COMMAND, ARTIFACT_MEDIA_PROTECTED_PATHS_COMMAND,
    ARTIFACT_MEDIA_PUBLISH_IMAGE_COMMAND, ARTIFACT_MEDIA_UNREFERENCED_PATHS_COMMAND,
    ARTIFACT_MEDIA_WORKER_ID, ArtifactMediaMaintenanceRequest, ImageReferenceIdsRequest,
    MediaPathPrefixRequest, MediaPathsRequest, ProjectPathsRequest, ProjectPublicationRequest,
    PublishImageRequest, PublishedImageResponse,
};
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerErrorCode, WorkerHealth, WorkerHealthStatus,
    WorkerResponse,
};
use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseBackend, Statement};
use serde_json::{Value, json};
use tokio::time::MissedTickBehavior;

const DEFAULT_ADDR: &str = "127.0.0.1:47652";
const RECOVERY_LIMIT: usize = 64;
const CLEANUP_LIMIT: usize = 64;
const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(30);
const AUTH_HEADER: &str = "x-a4n-worker-token";

#[derive(Clone)]
struct WorkerState {
    db: a4n_db::DbHandle,
    data_dir: Arc<PathBuf>,
    token: Arc<str>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ImageIdRequest {
    image_id: i64,
}

pub fn run() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    init_tracing();
    let database_url = a4n_config::required_non_empty_env_string("A4N_DATABASE_URL")?;
    a4n_config::validate_postgres_database_url("A4N_DATABASE_URL", &database_url)?;
    let data_dir = a4n_config::app_config::AppConfig::default().data_dir;
    std::fs::create_dir_all(&data_dir)?;
    let token = a4n_config::required_non_empty_env_string(
        a4n_artifact_media_contract::ARTIFACT_MEDIA_WORKER_TOKEN_ENV,
    )?;
    let addr = worker_addr()?;
    let db = Database::connect(ConnectOptions::new(database_url)).await?;
    let state = WorkerState {
        db: a4n_db::DbHandle::new(Arc::new(db)),
        data_dir: Arc::new(data_dir),
        token: Arc::from(token),
    };
    let maintenance_state = state.clone();
    let maintenance = tokio::spawn(async move {
        let mut interval = tokio::time::interval(MAINTENANCE_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(error) = maintenance_state
                .maintain(ArtifactMediaMaintenanceRequest {
                    publication_limit: RECOVERY_LIMIT as u16,
                    cleanup_limit: CLEANUP_LIMIT as u16,
                })
                .await
            {
                tracing::warn!(error = %error, "artifact media worker maintenance failed");
            }
        }
    });
    let app = Router::new()
        .route(WORKER_HEALTH_PATH, get(health))
        .route(WORKER_CAPABILITIES_PATH, get(capabilities))
        .route("/v1/commands/{command}", post(command))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(worker_id = ARTIFACT_MEDIA_WORKER_ID, %addr, "artifact media worker listening");
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;
    maintenance.abort();
    result?;
    Ok(())
}

async fn health(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !authorized(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let status = match state
        .db
        .connection()
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT 1",
        ))
        .await
    {
        Ok(_) => WorkerHealthStatus::Healthy,
        Err(_) => WorkerHealthStatus::Degraded,
    };
    Json(WorkerHealth {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: ARTIFACT_MEDIA_WORKER_ID.to_string(),
        status,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
    .into_response()
}

async fn capabilities(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !authorized(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    Json(WorkerCapabilities {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: ARTIFACT_MEDIA_WORKER_ID.to_string(),
        capabilities: vec![WorkerCapability {
            name: "artifact-media".to_string(),
            version: ARTIFACT_MEDIA_CAPABILITY_VERSION.to_string(),
            enabled: true,
        }],
    })
    .into_response()
}

async fn command(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    AxumPath(command): AxumPath<String>,
    Json(request): Json<WorkerEnvelope<Value>>,
) -> Response {
    if !authorized(&state, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    if let Err(error) = request.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response();
    }
    if request.lease.worker_id != ARTIFACT_MEDIA_WORKER_ID {
        return Json(failed_response(
            &request,
            WorkerErrorCode::LeaseLost,
            "worker lease identity does not match artifact media worker",
        ))
        .into_response();
    }
    let user_command = command == ARTIFACT_MEDIA_PUBLISH_IMAGE_COMMAND
        || matches!(
            command.as_str(),
            ARTIFACT_MEDIA_IMAGE_BY_ID_COMMAND
                | ARTIFACT_MEDIA_IMAGE_LIST_PROJECT_COMMAND
                | ARTIFACT_MEDIA_IMAGE_LIST_PROJECTS_COMMAND
                | ARTIFACT_MEDIA_IMAGE_LIST_CHAPTER_COMMAND
                | ARTIFACT_MEDIA_IMAGE_UPDATE_COMMAND
                | ARTIFACT_MEDIA_IMAGE_DELETE_COMMAND
        );
    if (!user_command
        && request.principal.kind != a4n_worker_contract::WorkerPrincipalKind::Service)
        || (user_command
            && request.principal.kind != a4n_worker_contract::WorkerPrincipalKind::User)
    {
        return Json(failed_response(
            &request,
            WorkerErrorCode::Unauthorized,
            "artifact media command principal kind is invalid",
        ))
        .into_response();
    }
    let result = match command.as_str() {
        ARTIFACT_MEDIA_MAINTENANCE_COMMAND => {
            match parse_payload::<ArtifactMediaMaintenanceRequest>(request.payload.clone()) {
                Ok(payload) => state.maintain(payload).await.map(|payload| json!(payload)),
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_UNREFERENCED_PATHS_COMMAND => {
            match parse_payload::<MediaPathsRequest>(request.payload.clone()) {
                Ok(payload) => state
                    .unreferenced_paths(payload)
                    .await
                    .map(|payload| json!(payload)),
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_PATH_PREFIX_REFERENCED_COMMAND => {
            match parse_payload::<MediaPathPrefixRequest>(request.payload.clone()) {
                Ok(payload) => state
                    .path_prefix_referenced(payload)
                    .await
                    .map(|payload| json!(payload)),
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_PROJECT_PUBLICATION_COMMAND => {
            match parse_payload::<ProjectPublicationRequest>(request.payload.clone()) {
                Ok(payload) => state
                    .project_has_publication(payload)
                    .await
                    .map(|payload| json!(payload)),
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_PROTECTED_PATHS_COMMAND => {
            match parse_payload::<ProjectPathsRequest>(request.payload.clone()) {
                Ok(payload) => state
                    .protected_paths(payload)
                    .await
                    .map(|payload| json!(payload)),
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_PUBLISH_IMAGE_COMMAND => {
            match parse_payload::<PublishImageRequest>(request.payload.clone()) {
                Ok(payload) => match request.principal.subject.parse::<i64>() {
                    Ok(actor_user_id) if actor_user_id > 0 => state
                        .publish_image(actor_user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    _ => Err("user principal subject must be a positive integer".to_string()),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_BY_ID_COMMAND => {
            match parse_payload::<ImageIdRequest>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .image_by_id(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_LIST_PROJECT_COMMAND => {
            match parse_payload::<ListImagesByProjectInput>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .image_list_by_project(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_LIST_PROJECTS_COMMAND => {
            match parse_payload::<ListImagesByProjectsInput>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .image_list_by_projects(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_LIST_CHAPTER_COMMAND => {
            match parse_payload::<ListImagesByChapterInput>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .image_list_by_chapter(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_REFERENCE_IDS_COMMAND => {
            match parse_payload::<ImageReferenceIdsRequest>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .image_reference_by_ids(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_UPDATE_COMMAND => {
            match parse_payload::<UpdateGeneratedImageInput>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .update_image(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        ARTIFACT_MEDIA_IMAGE_DELETE_COMMAND => {
            match parse_payload::<DeleteImageInput>(request.payload.clone()) {
                Ok(payload) => match actor_user_id(&request) {
                    Ok(user_id) => state
                        .delete_image(user_id, payload)
                        .await
                        .map(|payload| json!(payload)),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        _ => Err(format!("unsupported artifact media command: {command}")),
    };
    match result {
        Ok(payload) => Json(completed_response(&request, payload)).into_response(),
        Err(error) => {
            Json(failed_response(&request, classify_error(&error), error)).into_response()
        }
    }
}

mod cleanup;
mod image;
mod publication;
mod recovery;
mod references;
mod support;
use support::*;

fn worker_addr() -> anyhow::Result<SocketAddr> {
    let value = std::env::var("A4N_ARTIFACT_MEDIA_WORKER_ADDR")
        .unwrap_or_else(|_| DEFAULT_ADDR.to_string());
    value
        .parse()
        .map_err(|error| anyhow::anyhow!("A4N_ARTIFACT_MEDIA_WORKER_ADDR 无效: {error}"))
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

#[cfg(test)]
mod tests {
    use super::facts_match;
    use a4n_domain::media::ImagePublicationRecoveryRecord;
    use std::path::PathBuf;

    #[test]
    fn recovery_only_accepts_exact_file_facts() {
        let publication = ImagePublicationRecoveryRecord {
            publication_id: "publication".to_string(),
            project_id: 1,
            requested_by_user_id: 2,
            staging_path: "projects/1/a.part".to_string(),
            final_path: "projects/1/a.png".to_string(),
            image_type: "project_reference".to_string(),
            model: a4n_domain::EXTERNAL_IMPORT_MODEL.to_string(),
            expected_file_size: 3,
            expected_sha256: "abc".to_string(),
            file_size: None,
            status: "pending".to_string(),
            version: 1,
            image_id: None,
        };
        assert!(facts_match(
            &(PathBuf::new(), 3, "abc".to_string()),
            &publication
        ));
        assert!(!facts_match(
            &(PathBuf::new(), 4, "abc".to_string()),
            &publication
        ));
    }
}
