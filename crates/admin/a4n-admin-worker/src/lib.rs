//! PostgreSQL-backed administrative application worker.

use std::{net::SocketAddr, sync::Arc};

use a4n_admin_worker_contract::{
    ADMIN_ACTIVE_OPERATION, ADMIN_BROADCAST_NOTIFICATION_OPERATION, ADMIN_CREATE_INVITE_OPERATION,
    ADMIN_CREATE_USER_OPERATION, ADMIN_DATABASE_HEALTH_OPERATION,
    ADMIN_GENERATION_ANALYTICS_OPERATION, ADMIN_INVITES_OPERATION, ADMIN_OVERVIEW_OPERATION,
    ADMIN_PENDING_TASK_INDEX_OPERATION, ADMIN_PENDING_TASKS_OPERATION, ADMIN_PERMISSION_OPERATION,
    ADMIN_PROJECTS_OPERATION, ADMIN_READ_CAPABILITY, ADMIN_TOKEN_SUMMARY_OPERATION,
    ADMIN_UPDATE_ROLE_OPERATION, ADMIN_USAGE_TRENDS_OPERATION, ADMIN_USER_USAGE_OPERATION,
    ADMIN_USERS_OPERATION, ADMIN_WORKER_CONTRACT_VERSION, ADMIN_WORKER_ID, ADMIN_WRITE_CAPABILITY,
    AdminBroadcastNotificationInput, AdminCallerInput, AdminCreateInviteInput,
    AdminCreateUserInput, AdminGenerationAnalyticsInput, AdminPermissionInput,
    AdminUpdateRoleInput, AdminUsageTrendsInput, AdminUserInput, AdminUserUsageInput,
    BroadcastNotificationReceipt, CreatorNotificationInbox, NOTIFICATION_INBOX_OPERATION,
    NOTIFICATION_READ_OPERATION, NotificationCallerInput, NotificationReadInput, OperationAck,
};
use a4n_api_contract::admin::AdminPermission;
#[cfg(test)]
use a4n_db::DatabaseConnection;
use a4n_db::DbHandle;
use a4n_error::AppError;
use a4n_worker_contract::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerFailure, WorkerHealth, WorkerHealthStatus,
    WorkerPrincipal, WorkerPrincipalKind,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_ADMIN_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_ADMIN_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47679";

mod auth;
mod health;
mod mutations;
mod reads;
use health::{capabilities, health};

#[derive(Clone)]
pub struct AdminWorker {
    state: WorkerState,
}

#[derive(Clone)]
struct WorkerState {
    database: DbHandle,
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

impl AdminWorker {
    pub fn new(
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
            .route(
                "/v1/commands/admin.permission.require",
                post(require_permission),
            )
            .route("/v1/commands/admin.user.active", post(is_active_admin_user))
            .route("/v1/commands/admin.usage.user", post(user_usage))
            .route("/v1/commands/admin.token.summary", post(token_summary))
            .route("/v1/commands/admin.usage.trends", post(usage_trends))
            .route("/v1/commands/admin.overview", post(overview))
            .route("/v1/commands/admin.projects.list", post(projects))
            .route("/v1/commands/admin.tasks.pending", post(pending_tasks))
            .route(
                "/v1/commands/admin.tasks.pending.index",
                post(pending_task_index),
            )
            .route(
                "/v1/commands/admin.generation.analytics",
                post(generation_analytics),
            )
            .route("/v1/commands/admin.database.health", post(database_health))
            .route("/v1/commands/admin.users.list", post(users))
            .route("/v1/commands/admin.invites.list", post(invites))
            .route("/v1/commands/admin.users.create", post(create_user))
            .route("/v1/commands/admin.invites.create", post(create_invite))
            .route("/v1/commands/admin.users.role.update", post(update_role))
            .route(
                "/v1/commands/admin.notifications.broadcast",
                post(broadcast_notification),
            )
            .route("/v1/commands/notifications.inbox", post(notification_inbox))
            .route("/v1/commands/notifications.read", post(notification_read))
            .with_state(self.state.clone())
    }

    pub async fn serve(self, address: SocketAddr) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(address).await?;
        axum::serve(listener, self.router())
            .with_graceful_shutdown(a4n_worker_runtime::wait_for_shutdown_signal())
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
    let database = DbHandle::connect(database_url).await?;
    AdminWorker::new(database, ADMIN_WORKER_ID, auth_token)
        .serve(address)
        .await
}

macro_rules! handler {
    ($name:ident, $input:ty, $method:ident) => {
        async fn $name(
            State(state): State<WorkerState>,
            headers: HeaderMap,
            Json(request): Json<WorkerEnvelope<$input>>,
        ) -> Response {
            command_response(&state, &headers, request, |request| state.$method(request)).await
        }
    };
}

handler!(require_permission, AdminPermissionInput, require_permission);
handler!(is_active_admin_user, AdminUserInput, is_active_admin_user);
handler!(user_usage, AdminUserUsageInput, user_usage);
handler!(token_summary, AdminUserInput, token_summary);
handler!(usage_trends, AdminUsageTrendsInput, usage_trends);
handler!(overview, AdminCallerInput, overview);
handler!(projects, AdminCallerInput, projects);
handler!(pending_tasks, AdminCallerInput, pending_tasks);
handler!(pending_task_index, AdminCallerInput, pending_task_index);
handler!(
    generation_analytics,
    AdminGenerationAnalyticsInput,
    generation_analytics
);
handler!(database_health, AdminCallerInput, database_health);
handler!(users, AdminCallerInput, users);
handler!(invites, AdminCallerInput, invites);
handler!(create_user, AdminCreateUserInput, create_user);
handler!(create_invite, AdminCreateInviteInput, create_invite);
handler!(update_role, AdminUpdateRoleInput, update_role);
handler!(
    broadcast_notification,
    AdminBroadcastNotificationInput,
    broadcast_notification
);
handler!(
    notification_inbox,
    NotificationCallerInput,
    notification_inbox
);
handler!(
    notification_read,
    NotificationReadInput,
    mark_notification_read
);

async fn command_response<T, O, F, Fut>(
    state: &WorkerState,
    headers: &HeaderMap,
    request: WorkerEnvelope<T>,
    command: F,
) -> Response
where
    T: serde::Serialize + Clone,
    O: serde::Serialize,
    F: FnOnce(WorkerEnvelope<T>) -> Fut,
    Fut: std::future::Future<Output = Result<O, WorkerFailure>>,
{
    a4n_worker_runtime::worker_command_response(
        state.authorized(headers),
        request,
        command,
        unauthorized,
    )
    .await
}

fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request.validate_worker_command(operation, ADMIN_WORKER_ID, "admin worker")
}

fn admin_error(error: a4n_db::admin::AdminDbError) -> WorkerFailure {
    match error {
        a4n_db::admin::AdminDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::admin::AdminDbError::Database(error)
        | a4n_db::admin::AdminDbError::DatabaseOperation { source: error, .. } => {
            a4n_worker_db_adapter::map_database_error(error)
        }
    }
}

fn app_error(error: AppError) -> WorkerFailure {
    match error {
        AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        AppError::Forbidden(message) | AppError::Unauthorized(message) => {
            WorkerFailure::unauthorized(message)
        }
        AppError::NotFound(message) => WorkerFailure::not_found(message),
        other => WorkerFailure::unavailable(other.to_string()),
    }
}

fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    #[test]
    fn caller_must_be_a_matching_user_principal() {
        let state = AdminWorker {
            state: WorkerState {
                database: DbHandle::new(Arc::new(DatabaseConnection::Disconnected)),
                worker_id: ADMIN_WORKER_ID.into(),
                auth_token: "token".into(),
            },
        };
        let principal = WorkerPrincipal {
            kind: WorkerPrincipalKind::User,
            subject: "7".into(),
        };
        assert_eq!(state.state.caller(&principal, 7).unwrap(), 7);
        assert!(state.state.caller(&principal, 8).is_err());
    }

    #[tokio::test]
    async fn pending_task_index_route_checks_auth_and_reads_the_index() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let mut users = Vec::new();
        for (name, role) in [("index_admin", "admin"), ("index_creator", "user")] {
            users.push(
                a4n_db::auth::create_user_summary(
                    &db,
                    name,
                    &format!("{name}@example.test"),
                    "Index-test-password-42",
                    role,
                )
                .await
                .unwrap()
                .id,
            );
        }
        let router =
            AdminWorker::new(DbHandle::new(Arc::new(db)), ADMIN_WORKER_ID, "token").router();
        for (caller, token, permitted) in [
            (users[0], "wrong-token", false),
            (users[1], "token", false),
            (users[0], "token", true),
        ] {
            let envelope = WorkerEnvelope {
                schema_version: WORKER_SCHEMA_VERSION,
                request_id: "pending-index-test".into(),
                trace_id: "pending-index-test".into(),
                principal: WorkerPrincipal {
                    kind: WorkerPrincipalKind::User,
                    subject: caller.to_string(),
                },
                operation_id: ADMIN_PENDING_TASK_INDEX_OPERATION.into(),
                idempotency_key: "pending-index-test".into(),
                project_id: None,
                root_project_id: None,
                target_revision: None,
                runtime_config_snapshot_ref: None,
                lease: a4n_worker_contract::WorkerLease::control_request(ADMIN_WORKER_ID),
                payload: AdminCallerInput {
                    caller_user_id: caller,
                },
            };
            let response = router
                .clone()
                .oneshot(
                    Request::post(format!("/v1/commands/{ADMIN_PENDING_TASK_INDEX_OPERATION}"))
                        .header(WORKER_AUTH_HEADER, token)
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&envelope).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
            if token == "wrong-token" {
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
                continue;
            }
            assert_eq!(response.status(), StatusCode::OK);
            let body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
            let result: a4n_worker_contract::WorkerResponse<
                Vec<a4n_admin_worker_contract::PendingTaskIndexSummary>,
            > = serde_json::from_slice(&body).unwrap();
            if permitted {
                assert!(result.error.is_none(), "{:?}", result.error);
                assert!(result.payload.unwrap().is_empty());
            } else {
                assert!(result.payload.is_none());
                assert!(result.error.unwrap().message.contains("当前用户不是管理员"));
            }
        }
    }
}
