use super::agents::AgentRegistry;
use super::auth::authenticated_user;
use super::error::ApiError;
use crate::state::ServerState;
use a4n_api_contract::command_manifest::command_manifest;
use a4n_api_contract::page::{
    PAGE_SESSION_PROTOCOL_VERSION, PageAttachResponse, PageAuthorization, PageCommandAvailability,
    PageCommandManifest, PageContext, PageNavigationInput, PageOperationInput, PageOperationStatus,
    PageOperationUpdate, PageOperationView, PageSessionCreateResponse, PageSessionEvent,
    PageSessionStatus, PageSessionView,
};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::{Stream, stream};
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const PAGE_LEASE_SECONDS: u64 = 30;
const AUTHORIZATION_SECONDS: u64 = 120;
const MAX_OPERATIONS: usize = 50;
const MAX_ROUTE_LENGTH: usize = 2048;
const MAX_STATE_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub(crate) struct PageSessionRegistry {
    records: Arc<RwLock<HashMap<String, PageRecord>>>,
    events: tokio::sync::broadcast::Sender<PageEvent>,
}

#[derive(Clone, Debug)]
struct PageRecord {
    view: PageSessionView,
    token: String,
    owner_user_id: i64,
    authorization: Option<PageAuthorizationRecord>,
    operations: VecDeque<PageOperationView>,
}

#[derive(Clone, Debug)]
struct PageAuthorizationRecord {
    code: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug)]
struct PageEvent {
    owner_user_id: i64,
    event: PageSessionEvent,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreatePageSessionInput {
    client: String,
    tab_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttachPageSessionInput {
    authorization_code: String,
    agent_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PageHeartbeatInput {
    #[serde(default)]
    context: Option<PageContext>,
}

impl PageSessionRegistry {
    pub(crate) fn new() -> Self {
        let (events, _) = tokio::sync::broadcast::channel(256);
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            events,
        }
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<PageEvent> {
        self.events.subscribe()
    }

    fn create(
        &self,
        owner_user_id: i64,
        input: CreatePageSessionInput,
        page_cli_enabled: bool,
    ) -> Result<PageSessionCreateResponse, ApiError> {
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        let client = required_text(input.client, "client")?;
        let tab_id = required_text(input.tab_id, "tab_id")?;
        let now = chrono::Utc::now();
        let now_text = now.to_rfc3339();
        let (response, event_kind) = {
            let mut records = self.records.write();
            if let Some(record) = records.values_mut().find(|record| {
                record.owner_user_id == owner_user_id
                    && record.view.client == client
                    && record.view.tab_id == tab_id
                    && record.view.status != PageSessionStatus::Offline
            }) {
                touch(&mut record.view, now);
                (
                    PageSessionCreateResponse {
                        session: record.view.clone(),
                        ui_session_token: record.token.clone(),
                        lease_seconds: PAGE_LEASE_SECONDS,
                    },
                    "resumed",
                )
            } else {
                let id = Uuid::new_v4().to_string();
                let token = Uuid::new_v4().to_string();
                let view = PageSessionView {
                    protocol_version: PAGE_SESSION_PROTOCOL_VERSION,
                    ui_session_id: id.clone(),
                    mode: "page".into(),
                    status: PageSessionStatus::AwaitingAttach,
                    client: client.clone(),
                    tab_id: tab_id.clone(),
                    page_kind: None,
                    route: None,
                    project_id: None,
                    chapter_id: None,
                    active_tab: None,
                    selected_ids: Value::Object(Default::default()),
                    dirty_state: false,
                    local_state: Value::Object(Default::default()),
                    context_revision: 0,
                    agent_id: None,
                    operation_id: None,
                    last_operation: None,
                    created_at: now_text.clone(),
                    updated_at: now_text.clone(),
                    last_seen_at: now_text.clone(),
                    expires_at: lease_expiry(now),
                };
                let response = PageSessionCreateResponse {
                    session: view.clone(),
                    ui_session_token: token.clone(),
                    lease_seconds: PAGE_LEASE_SECONDS,
                };
                records.insert(
                    id,
                    PageRecord {
                        view: view.clone(),
                        token,
                        owner_user_id,
                        authorization: None,
                        operations: VecDeque::new(),
                    },
                );
                (response, "created")
            }
        };
        self.publish(
            owner_user_id,
            event_kind,
            response.session.clone(),
            None,
            None,
            None,
        );
        Ok(response)
    }

    fn list(&self, owner_user_id: i64) -> Vec<PageSessionView> {
        self.expire();
        let mut sessions: Vec<_> = self
            .records
            .read()
            .values()
            .filter(|record| record.owner_user_id == owner_user_id)
            .map(|record| record.view.clone())
            .collect();
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        sessions
    }

    fn get(&self, owner_user_id: i64, session_id: &str) -> Result<PageSessionView, ApiError> {
        self.expire();
        self.records
            .read()
            .get(session_id)
            .filter(|record| record.owner_user_id == owner_user_id)
            .map(|record| record.view.clone())
            .ok_or_else(|| ApiError::not_found("Page Session 不存在"))
    }

    fn commands(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        is_admin: bool,
        page_cli_enabled: bool,
    ) -> Result<PageCommandManifest, ApiError> {
        self.expire();
        let records = self.records.read();
        let session = records
            .get(session_id)
            .filter(|record| record.owner_user_id == owner_user_id)
            .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
        authorize_token(session, owner_user_id, token)?;
        let view = &session.view;
        let commands = command_manifest(None, None, None)
            .commands
            .into_iter()
            .filter(|descriptor| {
                descriptor.auth != "public"
                    && descriptor
                        .page_scopes
                        .iter()
                        .any(|scope| page_scope_matches(scope, view.page_kind.as_deref()))
            })
            .map(|descriptor| {
                let reason = if !page_cli_enabled {
                    Some("当前账户未启用 Page CLI".into())
                } else if descriptor.auth == "admin" && !is_admin {
                    Some("需要管理员权限".into())
                } else {
                    missing_page_context(&descriptor.required_ui_context, view)
                };
                PageCommandAvailability {
                    available: reason.is_none(),
                    reason,
                    descriptor,
                }
            })
            .collect();
        Ok(PageCommandManifest {
            protocol_version: PAGE_SESSION_PROTOCOL_VERSION,
            ui_session_id: session_id.to_string(),
            page_kind: view.page_kind.clone(),
            route: view.route.clone(),
            commands,
        })
    }

    fn attach_by_code(
        &self,
        owner_user_id: i64,
        input: AttachPageSessionInput,
        agent_registry: &AgentRegistry,
        agent_token: &str,
        page_cli_enabled: bool,
    ) -> Result<PageAttachResponse, ApiError> {
        self.expire();
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        let session_id = self
            .records
            .read()
            .values()
            .find(|record| {
                record.owner_user_id == owner_user_id
                    && record.authorization.as_ref().is_some_and(|authorization| {
                        authorization.code == input.authorization_code
                            && authorization.expires_at > chrono::Utc::now()
                    })
            })
            .map(|record| record.view.ui_session_id.clone())
            .ok_or_else(|| ApiError::unauthorized("Page Session 授权码无效或已过期"))?;
        self.attach(
            owner_user_id,
            &session_id,
            input,
            agent_registry,
            agent_token,
            page_cli_enabled,
        )
    }

    fn authorize(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        page_cli_enabled: bool,
    ) -> Result<PageAuthorization, ApiError> {
        self.expire();
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        let now = chrono::Utc::now();
        let code = format!("A4N-{}-{}", random_code_part(), random_code_part());
        let expires_at = now + chrono::Duration::seconds(AUTHORIZATION_SECONDS as i64);
        let view = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            if !matches!(
                record.view.status,
                PageSessionStatus::AwaitingAttach | PageSessionStatus::Idle
            ) {
                return Err(ApiError::conflict("当前 Page Session 不允许重新授权"));
            }
            record.authorization = Some(PageAuthorizationRecord {
                code: code.clone(),
                expires_at,
            });
            record.view.status = PageSessionStatus::AwaitingAttach;
            touch(&mut record.view, now);
            record.view.clone()
        };
        self.publish(owner_user_id, "authorization_ready", view, None, None, None);
        Ok(PageAuthorization {
            ui_session_id: session_id.to_string(),
            authorization_code: code,
            expires_in: AUTHORIZATION_SECONDS,
        })
    }

    fn attach(
        &self,
        owner_user_id: i64,
        session_id: &str,
        input: AttachPageSessionInput,
        agent_registry: &AgentRegistry,
        agent_token: &str,
        page_cli_enabled: bool,
    ) -> Result<PageAttachResponse, ApiError> {
        self.expire();
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        agent_registry.authorize(owner_user_id, &input.agent_id, agent_token)?;
        let now = chrono::Utc::now();
        let response = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            if record.owner_user_id != owner_user_id {
                return Err(ApiError::forbidden("无权绑定该 Page Session"));
            }
            let authorization = record
                .authorization
                .as_ref()
                .ok_or_else(|| ApiError::unauthorized("Page Session 尚未授权或授权码已使用"))?;
            if authorization.code != input.authorization_code || authorization.expires_at <= now {
                return Err(ApiError::unauthorized("Page Session 授权码无效或已过期"));
            }
            if record.view.agent_id.is_some() {
                return Err(ApiError::conflict("Page Session 已被其他 Agent 接管"));
            }
            record.authorization = None;
            record.view.agent_id = Some(input.agent_id);
            record.view.status = PageSessionStatus::Attached;
            touch(&mut record.view, now);
            PageAttachResponse {
                session: record.view.clone(),
                ui_session_token: record.token.clone(),
            }
        };
        self.publish(
            owner_user_id,
            "attached",
            response.session.clone(),
            None,
            None,
            None,
        );
        Ok(response)
    }

    fn heartbeat(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        context: Option<PageContext>,
    ) -> Result<PageSessionView, ApiError> {
        self.expire();
        let now = chrono::Utc::now();
        let view = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            if record.view.status == PageSessionStatus::Offline {
                return Err(ApiError::conflict("Page Session 已失效"));
            }
            if let Some(context) = context {
                apply_context(&mut record.view, context)?;
            }
            touch(&mut record.view, now);
            record.view.clone()
        };
        self.publish(owner_user_id, "heartbeat", view.clone(), None, None, None);
        Ok(view)
    }

    fn update_context(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        context: PageContext,
    ) -> Result<PageSessionView, ApiError> {
        self.expire();
        let now = chrono::Utc::now();
        let view = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            apply_context(&mut record.view, context)?;
            touch(&mut record.view, now);
            record.view.clone()
        };
        self.publish(
            owner_user_id,
            "context_updated",
            view.clone(),
            None,
            None,
            None,
        );
        Ok(view)
    }

    fn start_operation(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        agent_id: String,
        agent_registry: &AgentRegistry,
        agent_token: &str,
        page_cli_enabled: bool,
        is_admin: bool,
        input: PageOperationInput,
    ) -> Result<PageOperationView, ApiError> {
        self.expire();
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        agent_registry.authorize(owner_user_id, &agent_id, agent_token)?;
        let now = chrono::Utc::now();
        let operation = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            if record.view.agent_id.as_deref() != Some(agent_id.as_str()) {
                return Err(ApiError::forbidden("当前 Agent 未绑定该 Page Session"));
            }
            if record.view.operation_id.is_some() {
                return Err(ApiError::conflict("当前 Page Session 已有正在执行的操作"));
            }
            if matches!(
                record.view.status,
                PageSessionStatus::Paused | PageSessionStatus::Offline
            ) {
                return Err(ApiError::conflict("当前 Page Session 不允许开始操作"));
            }
            if let Some(expected) = input.context_revision
                && expected != record.view.context_revision
            {
                return Err(ApiError::conflict(
                    "Page Context 已变化，请重新读取当前页面上下文",
                ));
            }
            validate_operation_input(&input)?;
            validate_page_command(
                &input.command,
                &record.view,
                is_admin,
                input.allow_destructive,
            )?;
            let operation = PageOperationView {
                operation_id: Uuid::new_v4().to_string(),
                ui_session_id: session_id.to_string(),
                agent_id: Some(agent_id),
                command: input.command,
                page_kind: record.view.page_kind.clone(),
                phase: input.phase.unwrap_or_else(|| "running".into()),
                target: input.target,
                status: PageOperationStatus::Running,
                progress: None,
                summary: None,
                result: None,
                error: None,
                started_at: now.to_rfc3339(),
                updated_at: now.to_rfc3339(),
            };
            push_operation(record, operation.clone());
            record.view.operation_id = Some(operation.operation_id.clone());
            record.view.status = PageSessionStatus::Running;
            touch(&mut record.view, now);
            operation
        };
        let session = self.get(owner_user_id, session_id)?;
        self.publish(
            owner_user_id,
            "operation_started",
            session,
            Some(operation.clone()),
            None,
            None,
        );
        Ok(operation)
    }

    fn update_operation(
        &self,
        owner_user_id: i64,
        session_id: &str,
        operation_id: &str,
        token: &str,
        agent_id: &str,
        agent_registry: &AgentRegistry,
        agent_token: &str,
        page_cli_enabled: bool,
        update: PageOperationUpdate,
    ) -> Result<PageOperationView, ApiError> {
        self.expire();
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        agent_registry.authorize(owner_user_id, agent_id, agent_token)?;
        let now = chrono::Utc::now();
        let (operation, session) = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            let operation = record
                .operations
                .iter_mut()
                .find(|operation| operation.operation_id == operation_id)
                .ok_or_else(|| ApiError::not_found("Page Operation 不存在"))?;
            if operation.agent_id.as_deref() != Some(agent_id) {
                return Err(ApiError::forbidden("当前 Agent 未创建该 Page Operation"));
            }
            apply_operation_update(operation, update)?;
            operation.updated_at = now.to_rfc3339();
            let operation = operation.clone();
            if matches!(
                operation.status,
                PageOperationStatus::Succeeded
                    | PageOperationStatus::Failed
                    | PageOperationStatus::Cancelled
            ) {
                record.view.operation_id = None;
                record.view.status = if record.view.agent_id.is_some() {
                    PageSessionStatus::Attached
                } else {
                    PageSessionStatus::Idle
                };
            }
            record.view.last_operation = Some(operation.clone());
            touch(&mut record.view, now);
            (operation, record.view.clone())
        };
        self.publish(
            owner_user_id,
            "operation_updated",
            session,
            Some(operation.clone()),
            None,
            None,
        );
        Ok(operation)
    }

    fn navigate(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        agent_id: &str,
        agent_registry: &AgentRegistry,
        agent_token: &str,
        page_cli_enabled: bool,
        input: PageNavigationInput,
    ) -> Result<PageSessionView, ApiError> {
        self.expire();
        if !page_cli_enabled {
            return Err(ApiError::forbidden("当前账户未启用 Page CLI"));
        }
        agent_registry.authorize(owner_user_id, agent_id, agent_token)?;
        let now = chrono::Utc::now();
        validate_route(&input.route)?;
        let view = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            if record.view.agent_id.as_deref() != Some(agent_id) {
                return Err(ApiError::conflict("Page Session 尚未绑定 Agent"));
            }
            record.view.route = Some(input.route.clone());
            if input.page_kind.is_some() {
                record.view.page_kind = input.page_kind.clone();
            }
            touch(&mut record.view, now);
            record.view.clone()
        };
        self.publish(
            owner_user_id,
            "navigate",
            view.clone(),
            None,
            Some(input.route),
            input.reason,
        );
        Ok(view)
    }

    fn control(
        &self,
        owner_user_id: i64,
        session_id: &str,
        token: &str,
        action: &str,
    ) -> Result<PageSessionView, ApiError> {
        self.expire();
        let now = chrono::Utc::now();
        let (view, operation) = {
            let mut records = self.records.write();
            let record = records
                .get_mut(session_id)
                .ok_or_else(|| ApiError::not_found("Page Session 不存在"))?;
            authorize_token(record, owner_user_id, token)?;
            match action {
                "pause" => {
                    if record.view.agent_id.is_none() {
                        return Err(ApiError::conflict("Page Session 尚未绑定 Agent"));
                    }
                    record.view.status = PageSessionStatus::Paused;
                    touch(&mut record.view, now);
                    (record.view.clone(), None)
                }
                "resume" => {
                    if record.view.agent_id.is_none() {
                        return Err(ApiError::conflict("Page Session 尚未绑定 Agent"));
                    }
                    record.view.status = if record.view.operation_id.is_some() {
                        PageSessionStatus::Running
                    } else {
                        PageSessionStatus::Attached
                    };
                    touch(&mut record.view, now);
                    (record.view.clone(), None)
                }
                "cancel" => {
                    let operation_id = record.view.operation_id.clone();
                    let operation = cancel_operation(
                        record,
                        operation_id.as_deref(),
                        "Page Session 已由用户 cancel",
                        now,
                    );
                    record.view.operation_id = None;
                    record.view.status = if record.view.agent_id.is_some() {
                        PageSessionStatus::Attached
                    } else {
                        PageSessionStatus::Idle
                    };
                    record.view.last_operation = operation.clone();
                    touch(&mut record.view, now);
                    (record.view.clone(), operation)
                }
                "takeover" | "detach" => {
                    let operation_id = record.view.operation_id.clone();
                    let operation = cancel_operation(
                        record,
                        operation_id.as_deref(),
                        &format!("Page Session 已由用户 {action}"),
                        now,
                    );
                    record.view.status = PageSessionStatus::Idle;
                    record.view.agent_id = None;
                    record.view.operation_id = None;
                    record.view.last_operation = operation.clone();
                    touch(&mut record.view, now);
                    (record.view.clone(), operation)
                }
                _ => return Err(ApiError::bad_request("未知 Page Session 控制操作")),
            }
        };
        self.publish(owner_user_id, action, view.clone(), operation, None, None);
        Ok(view)
    }

    fn expire(&self) {
        let now = chrono::Utc::now();
        let mut expired = Vec::new();
        {
            let mut records = self.records.write();
            for record in records.values_mut() {
                if record.view.expires_at <= now.to_rfc3339()
                    && record.view.status != PageSessionStatus::Offline
                {
                    let operation_id = record.view.operation_id.clone();
                    record.view.status = PageSessionStatus::Offline;
                    record.view.agent_id = None;
                    record.view.operation_id = None;
                    let operation = cancel_operation(
                        record,
                        operation_id.as_deref(),
                        "Page Session 心跳已过期",
                        now,
                    );
                    record.view.last_operation = operation.clone();
                    touch(&mut record.view, now);
                    expired.push((record.owner_user_id, record.view.clone(), operation));
                }
            }
        }
        for (owner, view, operation) in expired {
            self.publish(
                owner,
                "offline",
                view,
                operation,
                None,
                Some("heartbeat_expired".into()),
            );
        }
    }

    fn publish(
        &self,
        owner_user_id: i64,
        kind: &str,
        session: PageSessionView,
        operation: Option<PageOperationView>,
        route: Option<String>,
        reason: Option<String>,
    ) {
        let _ = self.events.send(PageEvent {
            owner_user_id,
            event: PageSessionEvent {
                kind: kind.into(),
                session,
                operation,
                route,
                reason,
            },
        });
    }
}

pub(super) async fn create(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<CreatePageSessionInput>,
) -> Result<(StatusCode, Json<PageSessionCreateResponse>), ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .page_sessions()
                .create(user.id, input, user.page_cli_enabled)?,
        ),
    ))
}

pub(super) async fn list(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PageSessionView>>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().list(user.id)))
}

pub(super) async fn get(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<PageSessionView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().get(user.id, &session_id)?))
}

pub(super) async fn authorize(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<PageAuthorization>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().authorize(
        user.id,
        &session_id,
        ui_token(&headers)?,
        user.page_cli_enabled,
    )?))
}

pub(super) async fn attach(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(input): Json<AttachPageSessionInput>,
) -> Result<Json<PageAttachResponse>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().attach(
        user.id,
        &session_id,
        input,
        state.agents(),
        agent_token(&headers)?,
        user.page_cli_enabled,
    )?))
}

pub(super) async fn attach_by_code(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Json(input): Json<AttachPageSessionInput>,
) -> Result<Json<PageAttachResponse>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().attach_by_code(
        user.id,
        input,
        state.agents(),
        agent_token(&headers)?,
        user.page_cli_enabled,
    )?))
}

pub(super) async fn commands(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<Json<PageCommandManifest>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().commands(
        user.id,
        &session_id,
        ui_token(&headers)?,
        user.role == "admin",
        user.page_cli_enabled,
    )?))
}

pub(super) async fn context(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(input): Json<PageContext>,
) -> Result<Json<PageSessionView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().update_context(
        user.id,
        &session_id,
        ui_token(&headers)?,
        input,
    )?))
}

pub(super) async fn heartbeat(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(input): Json<PageHeartbeatInput>,
) -> Result<Json<PageSessionView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().heartbeat(
        user.id,
        &session_id,
        ui_token(&headers)?,
        input.context,
    )?))
}

pub(super) async fn start_operation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(input): Json<PageOperationInput>,
) -> Result<(StatusCode, Json<PageOperationView>), ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok((
        StatusCode::CREATED,
        Json(state.page_sessions().start_operation(
            user.id,
            &session_id,
            ui_token(&headers)?,
            header_required(&headers, "x-agent-id")?.to_string(),
            state.agents(),
            agent_token(&headers)?,
            user.page_cli_enabled,
            user.role == "admin",
            input,
        )?),
    ))
}

pub(super) async fn update_operation(
    State(state): State<ServerState>,
    headers: HeaderMap,
    path: Path<(String, String)>,
    Json(input): Json<PageOperationUpdate>,
) -> Result<Json<PageOperationView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let (session_id, operation_id) = path.0;
    Ok(Json(state.page_sessions().update_operation(
        user.id,
        &session_id,
        &operation_id,
        ui_token(&headers)?,
        header_required(&headers, "x-agent-id")?,
        state.agents(),
        agent_token(&headers)?,
        user.page_cli_enabled,
        input,
    )?))
}

pub(super) async fn navigate(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(input): Json<PageNavigationInput>,
) -> Result<Json<PageSessionView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    Ok(Json(state.page_sessions().navigate(
        user.id,
        &session_id,
        ui_token(&headers)?,
        header_required(&headers, "x-agent-id")?,
        state.agents(),
        agent_token(&headers)?,
        user.page_cli_enabled,
        input,
    )?))
}

pub(super) async fn control(
    State(state): State<ServerState>,
    headers: HeaderMap,
    path: Path<(String, String)>,
) -> Result<Json<PageSessionView>, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let (session_id, action) = path.0;
    Ok(Json(state.page_sessions().control(
        user.id,
        &session_id,
        ui_token(&headers)?,
        &action,
    )?))
}

pub(super) async fn events(
    State(state): State<ServerState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let user = authenticated_user(&state, &headers).await?;
    let stream = page_sse_stream(state.page_sessions().clone(), user.id);
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive"),
    ))
}

fn page_sse_stream(
    registry: PageSessionRegistry,
    owner_user_id: i64,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static {
    let receiver = registry.subscribe();
    let interval = tokio::time::interval(Duration::from_secs(5));
    stream::unfold(
        (registry, receiver, interval),
        move |(registry, mut receiver, mut interval)| async move {
            loop {
                tokio::select! {
                    result = receiver.recv() => match result {
                        Ok(event) if event.owner_user_id == owner_user_id => {
                            let data = serde_json::to_string(&event.event).expect("page event is serializable");
                            return Some((Ok(Event::default().event("page_session").data(data)), (registry, receiver, interval)));
                        }
                        Ok(_) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
                    },
                    _ = interval.tick() => registry.expire(),
                }
            }
        },
    )
}

fn apply_context(view: &mut PageSessionView, context: PageContext) -> Result<(), ApiError> {
    if context.context_revision < view.context_revision {
        return Err(ApiError::conflict("Page Context revision 已过期"));
    }
    if let Some(route) = context.route.as_deref() {
        validate_route(route)?;
    }
    validate_positive_id(context.project_id, "project_id")?;
    validate_positive_id(context.chapter_id, "chapter_id")?;
    validate_json_size(&context.selected_ids, "selected_ids")?;
    validate_json_size(&context.local_state, "local_state")?;
    view.page_kind = context.page_kind;
    view.route = context.route;
    view.project_id = context.project_id;
    view.chapter_id = context.chapter_id;
    view.active_tab = context.active_tab;
    view.selected_ids = context.selected_ids;
    view.dirty_state = context.dirty_state;
    view.local_state = context.local_state;
    view.context_revision = context.context_revision;
    Ok(())
}

fn page_scope_matches(scope: &str, page_kind: Option<&str>) -> bool {
    if matches!(scope, "system" | "task" | "workflow") {
        return true;
    }
    let Some(page_kind) = page_kind else {
        return false;
    };
    let page_kind = page_kind.to_ascii_lowercase();
    page_kind.contains(scope)
        || match scope {
            "project" => page_kind.contains("novel") || page_kind.contains("workspace"),
            "auth" => page_kind.contains("setting") || page_kind.contains("account"),
            "chapter" | "outline" | "blueprint" | "profile" => {
                page_kind.contains("novel") || page_kind.contains("write")
            }
            "asset" | "video" => page_kind.contains("video") || page_kind.contains("media"),
            "settings" => page_kind.contains("setting"),
            _ => false,
        }
}

fn missing_page_context(fields: &[String], view: &PageSessionView) -> Option<String> {
    let missing = fields
        .iter()
        .filter(|field| match field.as_str() {
            "projectId" => view.project_id.is_none(),
            "chapterId" => view.chapter_id.is_none(),
            _ => true,
        })
        .cloned()
        .collect::<Vec<_>>();
    (!missing.is_empty()).then(|| format!("缺少当前页面上下文: {}", missing.join(", ")))
}

fn validate_page_command(
    command: &str,
    view: &PageSessionView,
    is_admin: bool,
    allow_destructive: bool,
) -> Result<(), ApiError> {
    let descriptor = command_manifest(None, None, None)
        .commands
        .into_iter()
        .find(|descriptor| descriptor.name == command)
        .ok_or_else(|| ApiError::bad_request(format!("当前页面未暴露 Host API 命令: {command}")))?;
    if !descriptor
        .page_scopes
        .iter()
        .any(|scope| page_scope_matches(scope, view.page_kind.as_deref()))
    {
        return Err(ApiError::conflict("当前页面未暴露该 Host API 命令"));
    }
    if descriptor.auth == "admin" && !is_admin {
        return Err(ApiError::forbidden("需要管理员权限"));
    }
    if let Some(reason) = missing_page_context(&descriptor.required_ui_context, view) {
        return Err(ApiError::conflict(reason));
    }
    if descriptor.effect == "destructive" && !allow_destructive {
        return Err(ApiError::bad_request(
            "危险 Page 命令需要 allow_destructive=true",
        ));
    }
    Ok(())
}

fn apply_operation_update(
    operation: &mut PageOperationView,
    update: PageOperationUpdate,
) -> Result<(), ApiError> {
    if is_terminal_operation_status(&operation.status)
        && (update.phase.is_some()
            || update.progress.is_some()
            || update.summary.is_some()
            || update.result.is_some()
            || update.error.is_some()
            || update
                .status
                .as_ref()
                .is_some_and(|status| status != &operation.status))
    {
        return Err(ApiError::conflict(
            "Page Operation 已进入终态，不能继续更新",
        ));
    }
    if let Some(progress) = update.progress {
        if !progress.is_finite() || !(0.0..=100.0).contains(&progress) {
            return Err(ApiError::bad_request("progress 必须在 0 到 100 之间"));
        }
        operation.progress = Some(progress);
    }
    if let Some(status) = update.status {
        if !operation_status_transition_allowed(&operation.status, &status) {
            return Err(ApiError::conflict(
                "Page Operation 已进入终态，不能再次改变状态",
            ));
        }
        operation.status = status;
    }
    if let Some(phase) = update.phase {
        operation.phase = required_text(phase, "phase")?;
    }
    if let Some(summary) = update.summary {
        operation.summary = Some(required_text(summary, "summary")?);
    }
    if let Some(error) = update.error {
        operation.error = Some(required_text(error, "error")?);
    }
    if let Some(result) = update.result {
        validate_json_size(&result, "result")?;
        operation.result = Some(result);
    }
    Ok(())
}

fn is_terminal_operation_status(status: &PageOperationStatus) -> bool {
    matches!(
        status,
        PageOperationStatus::Succeeded
            | PageOperationStatus::Failed
            | PageOperationStatus::Cancelled
    )
}

fn operation_status_transition_allowed(
    current: &PageOperationStatus,
    next: &PageOperationStatus,
) -> bool {
    match current {
        PageOperationStatus::Running => matches!(
            next,
            PageOperationStatus::Running
                | PageOperationStatus::Succeeded
                | PageOperationStatus::Failed
                | PageOperationStatus::Cancelled
        ),
        PageOperationStatus::Succeeded
        | PageOperationStatus::Failed
        | PageOperationStatus::Cancelled => current == next,
    }
}

fn push_operation(record: &mut PageRecord, operation: PageOperationView) {
    record.operations.push_back(operation.clone());
    while record.operations.len() > MAX_OPERATIONS {
        record.operations.pop_front();
    }
    record.view.last_operation = Some(operation);
}

fn cancel_operation(
    record: &mut PageRecord,
    operation_id: Option<&str>,
    reason: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Option<PageOperationView> {
    let operation = record
        .operations
        .iter_mut()
        .find(|operation| Some(operation.operation_id.as_str()) == operation_id)?;
    if !matches!(
        operation.status,
        PageOperationStatus::Succeeded
            | PageOperationStatus::Failed
            | PageOperationStatus::Cancelled
    ) {
        operation.status = PageOperationStatus::Cancelled;
        operation.error = Some(reason.to_string());
        operation.updated_at = now.to_rfc3339();
    }
    Some(operation.clone())
}

fn validate_operation_input(input: &PageOperationInput) -> Result<(), ApiError> {
    required_text(input.command.clone(), "command")?;
    validate_json_size(&input.target, "target")
}

fn validate_route(route: &str) -> Result<(), ApiError> {
    let route = required_text(route.to_string(), "route")?;
    if route.len() > MAX_ROUTE_LENGTH || !route.starts_with('/') || route.starts_with("//") {
        return Err(ApiError::bad_request("route 必须是站内绝对路径"));
    }
    Ok(())
}

fn validate_json_size(value: &Value, field: &str) -> Result<(), ApiError> {
    let size = serde_json::to_vec(value)
        .map_err(|_| ApiError::bad_request(format!("{field} 无效")))?
        .len();
    if size > MAX_STATE_BYTES {
        return Err(ApiError::bad_request(format!("{field} 不能超过 64 KiB")));
    }
    Ok(())
}

fn authorize_token(record: &PageRecord, owner_user_id: i64, token: &str) -> Result<(), ApiError> {
    if record.owner_user_id != owner_user_id || record.token != token {
        return Err(ApiError::unauthorized("Page Session token 无效"));
    }
    Ok(())
}

fn touch(view: &mut PageSessionView, now: chrono::DateTime<chrono::Utc>) {
    view.updated_at = now.to_rfc3339();
    view.last_seen_at = view.updated_at.clone();
    view.expires_at = lease_expiry(now);
}

fn lease_expiry(now: chrono::DateTime<chrono::Utc>) -> String {
    (now + chrono::Duration::seconds(PAGE_LEASE_SECONDS as i64)).to_rfc3339()
}

fn required_text(value: String, field: &str) -> Result<String, ApiError> {
    let value = value.trim().to_string();
    if value.is_empty() || value.chars().count() > 200 {
        return Err(ApiError::bad_request(format!("{field} 无效")));
    }
    Ok(value)
}

fn validate_positive_id(value: Option<i64>, field: &str) -> Result<(), ApiError> {
    if value.is_some_and(|value| value <= 0) {
        return Err(ApiError::bad_request(format!("{field} 必须是正整数")));
    }
    Ok(())
}

fn random_code_part() -> String {
    Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(4)
        .collect::<String>()
        .to_uppercase()
}

fn ui_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    header_required(headers, "x-ui-session-token")
}

fn agent_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    header_required(headers, "x-agent-token")
}

fn header_required<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, ApiError> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::unauthorized(format!("缺少 {} 头", name)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_input() -> CreatePageSessionInput {
        CreatePageSessionInput {
            client: "a4n-test".into(),
            tab_id: "tab-1".into(),
        }
    }

    fn context(
        page_kind: &str,
        route: &str,
        revision: u64,
        project_id: Option<i64>,
    ) -> PageContext {
        PageContext {
            page_kind: Some(page_kind.into()),
            route: Some(route.into()),
            project_id,
            selected_ids: json!({}),
            local_state: json!({}),
            context_revision: revision,
            ..Default::default()
        }
    }

    #[test]
    fn route_validation_only_accepts_internal_absolute_paths() {
        assert!(validate_route("/write/42").is_ok());
        assert!(validate_route("https://example.com").is_err());
        assert!(validate_route("//example.com").is_err());
    }

    #[test]
    fn unknown_required_page_context_is_unavailable() {
        let registry = PageSessionRegistry::new();
        let session = registry.create(1, create_input(), true).unwrap();
        assert_eq!(
            missing_page_context(&["futureContext".into()], &session.session),
            Some("缺少当前页面上下文: futureContext".into())
        );
    }

    #[test]
    fn context_revision_rejects_stale_updates() {
        let mut view = PageSessionView {
            protocol_version: PAGE_SESSION_PROTOCOL_VERSION,
            ui_session_id: "ui".into(),
            mode: "page".into(),
            status: PageSessionStatus::Attached,
            client: "web".into(),
            tab_id: "tab".into(),
            page_kind: None,
            route: None,
            project_id: None,
            chapter_id: None,
            active_tab: None,
            selected_ids: Value::Null,
            dirty_state: false,
            local_state: Value::Null,
            context_revision: 2,
            agent_id: None,
            operation_id: None,
            last_operation: None,
            created_at: String::new(),
            updated_at: String::new(),
            last_seen_at: String::new(),
            expires_at: String::new(),
        };
        let result = apply_context(
            &mut view,
            PageContext {
                context_revision: 1,
                ..Default::default()
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn sessions_are_scoped_by_user_and_create_is_idempotent_per_tab() {
        let registry = PageSessionRegistry::new();
        let first = registry.create(1, create_input(), true).unwrap();
        let resumed = registry.create(1, create_input(), true).unwrap();

        assert_eq!(first.session.ui_session_id, resumed.session.ui_session_id);
        assert_eq!(first.ui_session_token, resumed.ui_session_token);
        assert!(first.session.active_tab.is_none());
        assert_eq!(registry.list(1).len(), 1);
        assert!(registry.list(2).is_empty());
        assert!(registry.create(2, create_input(), false).is_err());
    }

    #[test]
    fn authorization_code_is_consumed_once_and_requires_the_agent_token() {
        let registry = PageSessionRegistry::new();
        let agents = AgentRegistry::new();
        let created = registry.create(1, create_input(), true).unwrap();
        let authorization = registry
            .authorize(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                true,
            )
            .unwrap();
        let (agent_id, agent_token) = agents.register_for_test(1, "page-agent");
        let attached = registry
            .attach_by_code(
                1,
                AttachPageSessionInput {
                    authorization_code: authorization.authorization_code.clone(),
                    agent_id: agent_id.clone(),
                },
                &agents,
                &agent_token,
                true,
            )
            .unwrap();
        assert_eq!(
            attached.session.agent_id.as_deref(),
            Some(agent_id.as_str())
        );
        assert!(
            registry
                .attach_by_code(
                    1,
                    AttachPageSessionInput {
                        authorization_code: authorization.authorization_code,
                        agent_id,
                    },
                    &agents,
                    &agent_token,
                    true,
                )
                .is_err()
        );
    }

    #[test]
    fn admin_manifest_respects_role_and_account_switch() {
        let registry = PageSessionRegistry::new();
        let created = registry.create(1, create_input(), true).unwrap();
        registry
            .update_context(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                context("settings", "/settings", 1, None),
            )
            .unwrap();

        let ordinary = registry
            .commands(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                false,
                true,
            )
            .unwrap();
        let ordinary_admin = ordinary
            .commands
            .iter()
            .find(|item| item.descriptor.name == "list_all_users")
            .expect("admin command should be exposed on settings page");
        assert!(!ordinary_admin.available);
        assert_eq!(ordinary_admin.reason.as_deref(), Some("需要管理员权限"));

        let admin = registry
            .commands(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                true,
                true,
            )
            .unwrap();
        assert!(
            admin
                .commands
                .iter()
                .find(|item| item.descriptor.name == "list_all_users")
                .is_some_and(|item| item.available)
        );

        let disabled = registry
            .commands(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                true,
                false,
            )
            .unwrap();
        assert!(disabled.commands.iter().all(
            |item| !item.available && item.reason.as_deref() == Some("当前账户未启用 Page CLI")
        ));
    }

    #[test]
    fn context_revision_status_transitions_and_takeover_cancel_operations() {
        let registry = PageSessionRegistry::new();
        let agents = AgentRegistry::new();
        let created = registry.create(1, create_input(), true).unwrap();
        let updated = registry
            .update_context(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                context("novel-list", "/", 1, None),
            )
            .unwrap();
        assert_eq!(updated.context_revision, 1);
        assert!(
            registry
                .update_context(
                    1,
                    &created.session.ui_session_id,
                    &created.ui_session_token,
                    context("novel-list", "/", 0, None),
                )
                .is_err()
        );

        let authorization = registry
            .authorize(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                true,
            )
            .unwrap();
        let (agent_id, agent_token) = agents.register_for_test(1, "page-agent");
        registry
            .attach_by_code(
                1,
                AttachPageSessionInput {
                    authorization_code: authorization.authorization_code,
                    agent_id: agent_id.clone(),
                },
                &agents,
                &agent_token,
                true,
            )
            .unwrap();
        let operation = registry
            .start_operation(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                agent_id.clone(),
                &agents,
                &agent_token,
                true,
                false,
                PageOperationInput {
                    command: "list_projects".into(),
                    allow_destructive: false,
                    phase: None,
                    target: json!({}),
                    context_revision: Some(1),
                },
            )
            .unwrap();
        assert_eq!(operation.status, PageOperationStatus::Running);
        let cancelled = registry
            .control(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                "cancel",
            )
            .unwrap();
        assert_eq!(cancelled.status, PageSessionStatus::Attached);
        assert_eq!(
            cancelled.last_operation.unwrap().status,
            PageOperationStatus::Cancelled
        );

        let released = registry
            .control(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                "takeover",
            )
            .unwrap();
        assert_eq!(released.status, PageSessionStatus::Idle);
        assert!(released.agent_id.is_none());

        let authorization = registry
            .authorize(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                true,
            )
            .unwrap();
        registry
            .attach_by_code(
                1,
                AttachPageSessionInput {
                    authorization_code: authorization.authorization_code,
                    agent_id,
                },
                &agents,
                &agent_token,
                true,
            )
            .unwrap();
        let taken_over = registry
            .control(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                "takeover",
            )
            .unwrap();
        assert_eq!(taken_over.status, PageSessionStatus::Idle);
        assert!(taken_over.agent_id.is_none());
    }

    #[test]
    fn operation_validation_rejects_unavailable_and_unconfirmed_destructive_commands() {
        let registry = PageSessionRegistry::new();
        let agents = AgentRegistry::new();
        let created = registry.create(1, create_input(), true).unwrap();
        registry
            .update_context(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                context("novel-detail", "/novel/1", 1, Some(1)),
            )
            .unwrap();
        let authorization = registry
            .authorize(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                true,
            )
            .unwrap();
        let (agent_id, agent_token) = agents.register_for_test(1, "page-agent");
        registry
            .attach_by_code(
                1,
                AttachPageSessionInput {
                    authorization_code: authorization.authorization_code,
                    agent_id: agent_id.clone(),
                },
                &agents,
                &agent_token,
                true,
            )
            .unwrap();

        let input = |allow_destructive| PageOperationInput {
            command: "delete_project".into(),
            allow_destructive,
            phase: None,
            target: json!({}),
            context_revision: Some(1),
        };
        assert!(
            registry
                .start_operation(
                    1,
                    &created.session.ui_session_id,
                    &created.ui_session_token,
                    agent_id.clone(),
                    &agents,
                    &agent_token,
                    true,
                    false,
                    input(false),
                )
                .is_err()
        );
        assert!(
            registry
                .start_operation(
                    1,
                    &created.session.ui_session_id,
                    &created.ui_session_token,
                    agent_id,
                    &agents,
                    &agent_token,
                    true,
                    false,
                    input(true),
                )
                .is_ok()
        );
    }
}
