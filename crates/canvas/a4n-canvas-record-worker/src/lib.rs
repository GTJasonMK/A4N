//! PostgreSQL-backed Canvas graph document worker.

use std::{future::Future, net::SocketAddr, sync::Arc};

use a4n_canvas_record_worker_contract::{
    CANVAS_GRAPH_EDIT_OPERATION, CANVAS_GRAPH_GET_OPERATION, CANVAS_GRAPH_MUTATION_CAPABILITY,
    CANVAS_GRAPH_READ_CAPABILITY, CANVAS_GRAPH_READ_OPERATION, CANVAS_GRAPH_SAVE_OPERATION,
    CANVAS_RECORD_WORKER_CONTRACT_VERSION, CANVAS_RECORD_WORKER_ID,
    CANVAS_TEXT_ASSET_LIST_OPERATION, CANVAS_TEXT_ASSET_SAVE_OPERATION,
    CANVAS_TEXT_ASSET_VERSIONS_OPERATION, CanvasGraphDocument, CanvasGraphEditResult,
    CanvasGraphReadResult, CanvasGraphSaveResult, CanvasTextAssetSummary,
    CanvasTextAssetVersionSummary, EditCanvasGraphInput, GetCanvasGraphInput,
    ListCanvasTextAssetVersionsInput, ListCanvasTextAssetsInput, ReadCanvasGraphInput,
    SaveCanvasGraphInput, SaveCanvasTextAssetInput,
};
use a4n_db::DbHandle;
use a4n_db::canvas::CanvasDbError;
use a4n_domain::{canvas::CanvasTextAssetRecordInput, collaboration::ProjectAccessRole};
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerFailure, WorkerHealth, WorkerHealthStatus,
    WorkerResponse,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::Utc;
use serde::Serialize;

mod graph;
mod mutations;
mod reads;

const WORKER_ADDR_ENV: &str = "A4N_CANVAS_RECORD_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_CANVAS_RECORD_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47677";

#[derive(Clone)]
pub struct CanvasRecordWorker {
    state: WorkerState,
}

#[derive(Clone)]
struct WorkerState {
    database: DbHandle,
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

#[derive(Clone)]
struct RequestMeta {
    request_id: String,
    trace_id: String,
    operation_id: String,
}

impl CanvasRecordWorker {
    pub fn new(
        database: a4n_db::DatabaseConnection,
        worker_id: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self::new_with_handle(DbHandle::new(Arc::new(database)), worker_id, auth_token)
    }

    pub fn new_with_handle(
        database: DbHandle,
        worker_id: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            state: WorkerState {
                database,
                worker_id: Arc::<str>::from(worker_id.into()),
                auth_token: Arc::<str>::from(auth_token.into()),
            },
        }
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route(WORKER_HEALTH_PATH, get(health))
            .route(WORKER_CAPABILITIES_PATH, get(capabilities))
            .route("/v1/commands/canvas.graph.get", post(get_graph))
            .route("/v1/commands/canvas.graph.read", post(read_graph))
            .route("/v1/commands/canvas.graph.save", post(save_graph))
            .route("/v1/commands/canvas.graph.edit", post(edit_graph))
            .route(
                "/v1/commands/canvas.text_asset.list",
                post(list_text_assets),
            )
            .route(
                "/v1/commands/canvas.text_asset.versions",
                post(list_text_asset_versions),
            )
            .route("/v1/commands/canvas.text_asset.save", post(save_text_asset))
            .with_state(self.state.clone())
    }

    pub async fn serve(self, address: SocketAddr) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(address).await?;
        axum::serve(listener, self.router())
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(anyhow::Error::from)
    }
}

pub fn run() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    a4n_worker_runtime::init_worker_tracing();
    let database_url = a4n_config::required_non_empty_env_string(DATABASE_URL_ENV)?;
    a4n_config::validate_postgres_database_url(DATABASE_URL_ENV, &database_url)?;
    let auth_token = a4n_config::required_non_empty_env_string(WORKER_TOKEN_ENV)?;
    let address = a4n_config::worker_listen_addr(WORKER_ADDR_ENV, DEFAULT_ADDR)?;
    let database = DbHandle::connect_default(database_url).await?;
    CanvasRecordWorker::new_with_handle(database, CANVAS_RECORD_WORKER_ID, auth_token)
        .serve(address)
        .await
}

async fn health(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let status = match state.database.ping().await {
        Ok(()) => WorkerHealthStatus::Healthy,
        Err(_) => WorkerHealthStatus::Degraded,
    };
    Json(WorkerHealth {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        status,
        version: env!("CARGO_PKG_VERSION").into(),
    })
    .into_response()
}

async fn capabilities(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    Json(WorkerCapabilities {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        capabilities: vec![
            WorkerCapability {
                name: CANVAS_GRAPH_READ_CAPABILITY.into(),
                version: CANVAS_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: CANVAS_GRAPH_MUTATION_CAPABILITY.into(),
                version: CANVAS_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn get_graph(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<GetCanvasGraphInput>>,
) -> Response {
    dispatch(
        state,
        headers,
        request,
        CANVAS_GRAPH_GET_OPERATION,
        |state, request| async move { state.get_graph(request).await },
    )
    .await
}

async fn read_graph(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ReadCanvasGraphInput>>,
) -> Response {
    dispatch(
        state,
        headers,
        request,
        CANVAS_GRAPH_READ_OPERATION,
        |state, request| async move { state.read_graph(request).await },
    )
    .await
}

async fn save_graph(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<SaveCanvasGraphInput>>,
) -> Response {
    dispatch(
        state,
        headers,
        request,
        CANVAS_GRAPH_SAVE_OPERATION,
        |state, request| async move { state.save_graph(request).await },
    )
    .await
}

async fn edit_graph(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<EditCanvasGraphInput>>,
) -> Response {
    dispatch(
        state,
        headers,
        request,
        CANVAS_GRAPH_EDIT_OPERATION,
        |state, request| async move { state.edit_graph(request).await },
    )
    .await
}

async fn save_text_asset(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<SaveCanvasTextAssetInput>>,
) -> Response {
    dispatch(
        state,
        headers,
        request,
        CANVAS_TEXT_ASSET_SAVE_OPERATION,
        |state, request| async move { state.save_text_asset(request).await },
    )
    .await
}

async fn list_text_assets(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ListCanvasTextAssetsInput>>,
) -> Response {
    dispatch(
        state,
        headers,
        request,
        CANVAS_TEXT_ASSET_LIST_OPERATION,
        |state, request| async move { state.list_text_assets(request).await },
    )
    .await
}

async fn list_text_asset_versions(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ListCanvasTextAssetVersionsInput>>,
) -> Response {
    dispatch(
        state,
        headers,
        request,
        CANVAS_TEXT_ASSET_VERSIONS_OPERATION,
        |state, request| async move { state.list_text_asset_versions(request).await },
    )
    .await
}

async fn dispatch<T, O, F, Fut>(
    state: WorkerState,
    headers: HeaderMap,
    request: WorkerEnvelope<T>,
    expected_operation: &'static str,
    action: F,
) -> Response
where
    T: Serialize,
    O: Serialize,
    F: FnOnce(WorkerState, WorkerEnvelope<T>) -> Fut,
    Fut: Future<Output = Result<O, WorkerFailure>>,
{
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let meta = RequestMeta::from(&request);
    let response = match validate_request(&request, expected_operation) {
        Ok(()) => match action(state, request).await {
            Ok(payload) => completed(meta, payload),
            Err(error) => failed(meta, error),
        },
        Err(error) => failed(meta, error),
    };
    Json(response).into_response()
}

impl WorkerState {
    async fn verify_project_permission(
        &self,
        project_id: i64,
        user_id: i64,
        role: ProjectAccessRole,
    ) -> Result<(), WorkerFailure> {
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            role,
        )
        .await
        .map(|_| ())
        .map_err(a4n_worker_db_adapter::map_collaboration_db_error_preserving_internal)
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        a4n_worker_runtime::worker_token_authorized(
            headers,
            WORKER_AUTH_HEADER,
            self.auth_token.as_bytes(),
        )
    }
}

fn validate_request<T: Serialize>(
    request: &WorkerEnvelope<T>,
    expected_operation: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(
        expected_operation,
        CANVAS_RECORD_WORKER_ID,
        "Canvas record worker",
    )
}

fn canvas_db_error(error: CanvasDbError) -> WorkerFailure {
    match error {
        CanvasDbError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        CanvasDbError::NotFound(message) => WorkerFailure::not_found(message),
        CanvasDbError::Forbidden(message) => WorkerFailure::unauthorized(message),
        CanvasDbError::VersionConflict {
            graph_id,
            expected_version,
            current_version,
        } => WorkerFailure::conflict(format!(
            "canvas_graph_version_conflict:graph_id={graph_id},expected={expected_version},current={current_version}"
        )),
        CanvasDbError::TextAssetVersionConflict {
            canvas_text_asset_id,
            expected_version,
            current_version,
        } => WorkerFailure::conflict(format!(
            "canvas_text_asset_version_conflict:asset_id={canvas_text_asset_id},expected={expected_version},current={current_version}"
        )),
        CanvasDbError::Database(error) => a4n_worker_db_adapter::map_database_error(error),
    }
}

fn completed<T: Serialize>(meta: RequestMeta, payload: T) -> WorkerResponse<T> {
    WorkerResponse::completed_for_parts(meta.request_id, meta.trace_id, meta.operation_id, payload)
}

fn failed<T>(meta: RequestMeta, failure: WorkerFailure) -> WorkerResponse<T> {
    WorkerResponse::failed_for_parts(
        meta.request_id,
        meta.trace_id,
        meta.operation_id,
        failure.into(),
    )
}

impl RequestMeta {
    fn from<T>(request: &WorkerEnvelope<T>) -> Self {
        Self {
            request_id: request.request_id.clone(),
            trace_id: request.trace_id.clone(),
            operation_id: request.operation_id.clone(),
        }
    }
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "Canvas record worker authentication required",
    )
        .into_response()
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
    use std::collections::BTreeMap;

    use super::graph::{apply_operation, read_canvas_graph};
    use super::validate_request;
    use a4n_canvas_record_worker_contract::{
        CANVAS_GRAPH_EDIT_OPERATION, CANVAS_RECORD_WORKER_ID, CanvasEdgeInput, CanvasEdgeSummary,
        CanvasGraphDocument, CanvasGraphEditOperation, CanvasGraphProjection, CanvasGraphSummary,
        CanvasNodeSummary, GetCanvasGraphInput, ReadCanvasGraphInput,
    };
    use a4n_worker_contract::{
        WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerLease, WorkerPrincipal, WorkerPrincipalKind,
    };

    fn request(project_id: Option<i64>, operation: &str) -> WorkerEnvelope<GetCanvasGraphInput> {
        WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: "request".into(),
            trace_id: "trace".into(),
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: "7".into(),
            },
            operation_id: operation.into(),
            idempotency_key: "key".into(),
            project_id,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: CANVAS_RECORD_WORKER_ID.into(),
                epoch: 1,
                fence_token: "control".into(),
                expires_at: "9999-12-31T23:59:59Z".into(),
            },
            payload: GetCanvasGraphInput {
                project_id: 1,
                graph_id: Some(1),
            },
        }
    }

    fn graph() -> CanvasGraphDocument {
        CanvasGraphDocument {
            graph: CanvasGraphSummary {
                id: 1,
                name: "graph".into(),
                graph_kind: "video".into(),
                version: 1,
                viewport_json: None,
            },
            nodes: vec![1, 2]
                .into_iter()
                .map(|id| CanvasNodeSummary {
                    id,
                    node_kind: "TextInputNode".into(),
                    ref_type: None,
                    ref_id: None,
                    config_json: BTreeMap::new(),
                    position_json: BTreeMap::new(),
                    state: "draft".into(),
                })
                .collect(),
            edges: Vec::new(),
        }
    }

    fn read_input(
        node_ids: Option<Vec<i64>>,
        offset: u32,
        limit: Option<u32>,
    ) -> ReadCanvasGraphInput {
        ReadCanvasGraphInput {
            project_id: 1,
            graph_id: 1,
            expected_version: Some(1),
            node_ids,
            offset,
            limit,
        }
    }

    #[test]
    fn request_scope_and_operation_are_enforced() {
        let request = request(Some(1), CANVAS_GRAPH_EDIT_OPERATION);
        assert!(validate_request(&request, CANVAS_GRAPH_EDIT_OPERATION).is_ok());
        assert!(request.require_project_scope(1).is_ok());
        assert!(request.require_project_scope(2).is_err());
        assert!(validate_request(&request, "canvas.graph.read").is_err());
    }

    #[test]
    fn graph_edit_rejects_duplicate_edges() {
        let mut document = graph();
        let edge = CanvasGraphEditOperation::Connect {
            edge: CanvasEdgeInput {
                source_client_id: "1".into(),
                target_client_id: "2".into(),
                source_port: "text".into(),
                target_port: "text".into(),
                role: "input".into(),
                config_json: BTreeMap::new(),
            },
        };
        apply_operation(&mut document, edge.clone()).expect("first edge should be added");
        assert!(apply_operation(&mut document, edge).is_err());
        assert_eq!(document.edges.len(), 1);
    }

    #[test]
    fn graph_reads_preserve_node_neighbors_and_pagination() {
        let mut document = graph();
        document.edges.push(CanvasEdgeSummary {
            id: 3,
            source_node_id: 1,
            target_node_id: 2,
            source_port: "text".into(),
            target_port: "text".into(),
            role: "input".into(),
            config_json: BTreeMap::new(),
        });

        let details = read_canvas_graph(document, read_input(Some(vec![1]), 0, None))
            .expect("node details should be projected");
        match details.projection {
            CanvasGraphProjection::Nodes {
                nodes,
                edges,
                adjacent_nodes,
            } => {
                assert_eq!(
                    nodes.iter().map(|node| node.id).collect::<Vec<_>>(),
                    vec![1]
                );
                assert_eq!(
                    edges.iter().map(|edge| edge.id).collect::<Vec<_>>(),
                    vec![3]
                );
                assert_eq!(
                    adjacent_nodes
                        .iter()
                        .map(|node| node.id)
                        .collect::<Vec<_>>(),
                    vec![2]
                );
            }
            CanvasGraphProjection::Index { .. } => panic!("expected node projection"),
        }

        let page = read_canvas_graph(graph(), read_input(None, 1, Some(1)))
            .expect("node index page should be projected");
        match page.projection {
            CanvasGraphProjection::Index {
                nodes,
                total,
                next_offset,
            } => {
                assert_eq!(
                    nodes.iter().map(|node| node.id).collect::<Vec<_>>(),
                    vec![2]
                );
                assert_eq!(total, 2);
                assert_eq!(next_offset, None);
            }
            CanvasGraphProjection::Nodes { .. } => panic!("expected index projection"),
        }
    }
}
