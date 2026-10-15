use super::agents::AgentRegistry;
use super::auth::authenticated_user;
use super::error::ApiError;
use super::validate_positive_id;
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
use parking_lot::{Mutex, RwLock};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const PAGE_LEASE_SECONDS: u64 = 30;
const PAGE_EXPIRY_SWEEP_INTERVAL: Duration = Duration::from_secs(5);
const AUTHORIZATION_SECONDS: u64 = 120;
const MAX_OPERATIONS: usize = 50;
const MAX_ROUTE_LENGTH: usize = 2048;
const MAX_STATE_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub(crate) struct PageSessionRegistry {
    records: Arc<RwLock<HashMap<String, PageRecord>>>,
    page_cli_gate: Arc<tokio::sync::RwLock<()>>,
    events: tokio::sync::broadcast::Sender<PageEvent>,
    last_expiry_sweep: Arc<Mutex<Option<Instant>>>,
}

#[derive(Clone, Debug)]
struct PageRecord {
    view: PageSessionView,
    token: String,
    owner_user_id: i64,
    authorization: Option<PageAuthorizationRecord>,
    operations: VecDeque<PageOperationView>,
    operation_cancellation: Option<CancellationToken>,
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

include!("registry.rs");
include!("routes.rs");
include!("helpers.rs");

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

    fn attach_agent(
        registry: &PageSessionRegistry,
        agents: &AgentRegistry,
        created: &PageSessionCreateResponse,
    ) -> (String, String) {
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
                agents,
                &agent_token,
                true,
            )
            .unwrap();
        (agent_id, agent_token)
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
    fn detached_or_expired_sessions_cannot_consume_old_authorization_codes() {
        let registry = PageSessionRegistry::new();
        let agents = AgentRegistry::new();
        let detached = registry.create(1, create_input(), true).unwrap();
        let detached_code = registry
            .authorize(
                1,
                &detached.session.ui_session_id,
                &detached.ui_session_token,
                true,
            )
            .unwrap()
            .authorization_code;
        registry
            .control(
                1,
                &detached.session.ui_session_id,
                &detached.ui_session_token,
                "detach",
            )
            .unwrap();
        assert!(
            registry
                .attach_by_code(
                    1,
                    AttachPageSessionInput {
                        authorization_code: detached_code,
                        agent_id: "agent-1".into(),
                    },
                    &agents,
                    "agent-token",
                    true,
                )
                .is_err()
        );

        let mut second_input = create_input();
        second_input.tab_id = "tab-2".into();
        let expired = registry.create(1, second_input, true).unwrap();
        let expired_code = registry
            .authorize(
                1,
                &expired.session.ui_session_id,
                &expired.ui_session_token,
                true,
            )
            .unwrap()
            .authorization_code;
        registry
            .records
            .write()
            .get_mut(&expired.session.ui_session_id)
            .unwrap()
            .view
            .expires_at = (chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339();
        registry.expire();
        let expired_view = registry.get(1, &expired.session.ui_session_id).unwrap();
        assert_eq!(expired_view.status, PageSessionStatus::Offline);
        assert!(
            registry
                .attach_by_code(
                    1,
                    AttachPageSessionInput {
                        authorization_code: expired_code,
                        agent_id: "agent-1".into(),
                    },
                    &agents,
                    "agent-token",
                    true,
                )
                .is_err()
        );
    }

    #[test]
    fn disabling_page_cli_revokes_all_owner_sessions() {
        let registry = PageSessionRegistry::new();
        let agents = AgentRegistry::new();
        let active = registry.create(1, create_input(), true).unwrap();
        registry
            .update_context(
                1,
                &active.session.ui_session_id,
                &active.ui_session_token,
                context("novel-list", "/", 1, None),
            )
            .unwrap();
        let (agent_id, agent_token) = attach_agent(&registry, &agents, &active);
        registry
            .start_operation(
                1,
                &active.session.ui_session_id,
                &active.ui_session_token,
                agent_id,
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
        let mut pending_input = create_input();
        pending_input.tab_id = "tab-2".into();
        let pending = registry.create(1, pending_input, true).unwrap();
        let pending_code = registry
            .authorize(
                1,
                &pending.session.ui_session_id,
                &pending.ui_session_token,
                true,
            )
            .unwrap()
            .authorization_code;
        let other_user = registry.create(2, create_input(), true).unwrap();

        registry.revoke_user_sessions(1);

        let revoked = registry.get(1, &active.session.ui_session_id).unwrap();
        assert_eq!(revoked.status, PageSessionStatus::Offline);
        assert!(revoked.agent_id.is_none());
        assert!(revoked.operation_id.is_none());
        assert_eq!(
            revoked.last_operation.as_ref().unwrap().status,
            PageOperationStatus::Cancelled
        );
        assert!(
            registry
                .records
                .read()
                .get(&pending.session.ui_session_id)
                .unwrap()
                .authorization
                .is_none()
        );
        assert!(
            registry
                .attach_by_code(
                    1,
                    AttachPageSessionInput {
                        authorization_code: pending_code,
                        agent_id: "agent-1".into(),
                    },
                    &agents,
                    "agent-token",
                    true,
                )
                .is_err()
        );
        assert!(registry.create(1, create_input(), false).is_err());
        assert_eq!(
            registry
                .get(2, &other_user.session.ui_session_id)
                .unwrap()
                .status,
            PageSessionStatus::AwaitingAttach
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
                true,
            )
            .unwrap();
        assert!(disabled.commands.iter().all(
            |item| !item.available && item.reason.as_deref() == Some("当前账户未启用 Page CLI")
        ));
    }

    #[test]
    fn command_manifest_tracks_binding_pause_and_running_operation_state() {
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

        let unbound = registry
            .commands(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                false,
                true,
            )
            .unwrap();
        assert!(unbound.commands.iter().all(|item| !item.available
            && item.reason.as_deref() == Some("当前 Page Session 尚未绑定 Agent")));

        let (agent_id, agent_token) = attach_agent(&registry, &agents, &created);
        let attached = registry
            .commands(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                false,
                true,
            )
            .unwrap();
        assert!(attached.commands.iter().any(|item| item.available));

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
        let busy = registry
            .commands(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                false,
                true,
            )
            .unwrap();
        assert!(busy.commands.iter().all(|item| !item.available
            && item.reason.as_deref() == Some("当前 Page Session 正在执行操作")));

        registry
            .control(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                "pause",
            )
            .unwrap();
        let route = registry.navigate(
            1,
            &created.session.ui_session_id,
            &created.ui_session_token,
            &agent_id,
            &agents,
            &agent_token,
            true,
            PageNavigationInput {
                route: "/settings".into(),
                page_kind: Some("settings".into()),
                reason: Some("navigate while paused".into()),
            },
        );
        assert!(route.is_err());
        assert_eq!(
            registry
                .get(1, &created.session.ui_session_id)
                .unwrap()
                .route
                .as_deref(),
            Some("/novel/1")
        );
        let paused = registry
            .commands(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                false,
                true,
            )
            .unwrap();
        assert!(
            paused.commands.iter().all(|item| !item.available
                && item.reason.as_deref() == Some("当前 Page Session 已暂停"))
        );

        registry
            .update_operation(
                1,
                &created.session.ui_session_id,
                &operation.operation_id,
                &created.ui_session_token,
                &agent_id,
                &agents,
                &agent_token,
                true,
                PageOperationUpdate {
                    status: Some(PageOperationStatus::Succeeded),
                    phase: None,
                    progress: Some(100.0),
                    summary: Some("调用完成".into()),
                    result: Some(json!({"items": []})),
                    error: None,
                },
            )
            .unwrap();
        assert_eq!(
            registry
                .get(1, &created.session.ui_session_id)
                .unwrap()
                .status,
            PageSessionStatus::Paused
        );
        let still_paused = registry
            .commands(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                false,
                true,
            )
            .unwrap();
        assert!(
            still_paused.commands.iter().all(|item| !item.available
                && item.reason.as_deref() == Some("当前 Page Session 已暂停"))
        );

        registry
            .control(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                "resume",
            )
            .unwrap();
        assert!(
            registry
                .commands(
                    1,
                    &created.session.ui_session_id,
                    &created.ui_session_token,
                    false,
                    true
                )
                .unwrap()
                .commands
                .iter()
                .any(|item| item.available)
        );
        registry
            .control(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                "detach",
            )
            .unwrap();
        let detached = registry
            .commands(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                false,
                true,
            )
            .unwrap();
        assert!(detached.commands.iter().all(|item| !item.available
            && item.reason.as_deref() == Some("当前 Page Session 尚未绑定 Agent")));
    }

    #[test]
    fn cancelling_page_operation_notifies_host_command() {
        let registry = PageSessionRegistry::new();
        let agents = AgentRegistry::new();
        let created = registry.create(1, create_input(), true).unwrap();
        registry
            .update_context(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                context("novel-list", "/", 1, None),
            )
            .unwrap();
        let (agent_id, agent_token) = attach_agent(&registry, &agents, &created);
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
        let cancellation = registry
            .command_cancellation_token(
                1,
                &created.session.ui_session_id,
                &operation.operation_id,
                "list_projects",
                &created.ui_session_token,
                &agent_id,
                &agent_token,
                &agents,
            )
            .unwrap();

        registry
            .control(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                "cancel",
            )
            .unwrap();

        assert!(cancellation.is_cancelled());
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
    fn page_navigation_rejects_dirty_editor_context_without_changing_route() {
        let registry = PageSessionRegistry::new();
        let agents = AgentRegistry::new();
        let created = registry.create(1, create_input(), true).unwrap();
        let mut current_context = context("video-project", "/video-project/7", 1, Some(7));
        current_context.dirty_state = true;
        registry
            .update_context(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                current_context,
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

        let result = registry.navigate(
            1,
            &created.session.ui_session_id,
            &created.ui_session_token,
            &agent_id,
            &agents,
            &agent_token,
            true,
            PageNavigationInput {
                route: "/settings".into(),
                page_kind: Some("settings".into()),
                reason: Some("user requested settings".into()),
            },
        );

        assert!(result.is_err());
        assert_eq!(
            registry
                .get(1, &created.session.ui_session_id)
                .unwrap()
                .route
                .as_deref(),
            Some("/video-project/7")
        );

        let mut clean_context = context("video-project", "/video-project/7", 2, Some(7));
        clean_context.dirty_state = false;
        registry
            .update_context(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                clean_context,
            )
            .unwrap();
        let navigated = registry
            .navigate(
                1,
                &created.session.ui_session_id,
                &created.ui_session_token,
                &agent_id,
                &agents,
                &agent_token,
                true,
                PageNavigationInput {
                    route: "/settings".into(),
                    page_kind: Some("settings".into()),
                    reason: Some("user requested settings".into()),
                },
            )
            .unwrap();
        assert_eq!(navigated.route.as_deref(), Some("/settings"));
        assert_eq!(navigated.context_revision, 3);
        assert!(
            registry
                .heartbeat(
                    1,
                    &created.session.ui_session_id,
                    &created.ui_session_token,
                    Some(context("video-project", "/video-project/7", 1, Some(7))),
                )
                .is_err()
        );
        assert_eq!(
            registry
                .get(1, &created.session.ui_session_id)
                .unwrap()
                .route
                .as_deref(),
            Some("/settings")
        );
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

    #[test]
    fn periodic_expiry_sweep_is_shared_but_heartbeat_expires_its_session_immediately() {
        let registry = PageSessionRegistry::new();
        let first = registry.create(1, create_input(), true).unwrap();
        let second = registry
            .create(
                1,
                CreatePageSessionInput {
                    client: "a4n-test".into(),
                    tab_id: "tab-2".into(),
                },
                true,
            )
            .unwrap();
        let expired_at = (chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339();
        {
            let mut records = registry.records.write();
            records
                .get_mut(&first.session.ui_session_id)
                .unwrap()
                .view
                .expires_at = expired_at.clone();
            let record = records.get_mut(&second.session.ui_session_id).unwrap();
            let operation = PageOperationView {
                operation_id: "operation-1".into(),
                ui_session_id: second.session.ui_session_id.clone(),
                agent_id: Some("agent-1".into()),
                command: "read_chapter".into(),
                page_kind: Some("novel-detail".into()),
                phase: "running".into(),
                target: json!({}),
                status: PageOperationStatus::Running,
                progress: Some(12.0),
                summary: None,
                result: None,
                error: None,
                started_at: expired_at.clone(),
                updated_at: expired_at.clone(),
            };
            record.view.status = PageSessionStatus::Running;
            record.view.agent_id = Some("agent-1".into());
            record.view.operation_id = Some(operation.operation_id.clone());
            record.view.expires_at = expired_at;
            record.operations.push_back(operation.clone());
            record.view.last_operation = Some(operation);
        }
        let mut events = registry.subscribe();
        *registry.last_expiry_sweep.lock() = Some(Instant::now());

        registry.expire_periodically();
        assert_eq!(
            registry.records.read()[&second.session.ui_session_id]
                .view
                .status,
            PageSessionStatus::Running
        );
        assert!(
            registry
                .heartbeat(
                    1,
                    &second.session.ui_session_id,
                    &second.ui_session_token,
                    None,
                )
                .is_err()
        );
        assert_eq!(
            registry.records.read()[&second.session.ui_session_id]
                .view
                .status,
            PageSessionStatus::Offline
        );
        let expired_event = events.try_recv().unwrap();
        assert_eq!(expired_event.event.kind, "offline");
        assert_eq!(
            expired_event.event.operation.unwrap().status,
            PageOperationStatus::Cancelled
        );

        *registry.last_expiry_sweep.lock() = None;
        registry.expire_periodically();
        assert_eq!(
            registry.records.read()[&first.session.ui_session_id]
                .view
                .status,
            PageSessionStatus::Offline
        );
    }

    #[test]
    #[ignore = "release benchmark for Page Session expiry scans"]
    fn benchmark_page_session_heartbeat_expiry_scan() {
        const SESSION_COUNT: usize = 1_000;
        const SAMPLE_COUNT: usize = 15;

        let registry = PageSessionRegistry::new();
        let sessions: Vec<_> = (0..SESSION_COUNT)
            .map(|index| {
                let created = registry
                    .create(
                        1,
                        CreatePageSessionInput {
                            client: "performance-benchmark".into(),
                            tab_id: format!("tab-{index}"),
                        },
                        true,
                    )
                    .unwrap();
                (created.session.ui_session_id, created.ui_session_token)
            })
            .collect();

        let run_batch = || {
            *registry.last_expiry_sweep.lock() = None;
            for (session_id, token) in &sessions {
                registry.heartbeat(1, session_id, token, None).unwrap();
            }
        };
        for _ in 0..2 {
            run_batch();
        }

        let mut samples_ms = Vec::with_capacity(SAMPLE_COUNT);
        for _ in 0..SAMPLE_COUNT {
            let started_at = std::time::Instant::now();
            run_batch();
            samples_ms.push(started_at.elapsed().as_secs_f64() * 1_000.0);
        }
        samples_ms.sort_by(f64::total_cmp);
        let p50 = samples_ms[SAMPLE_COUNT / 2];
        let p95 = samples_ms[(SAMPLE_COUNT * 95).div_ceil(100) - 1];
        println!(
            "sessions={SESSION_COUNT} heartbeats_per_sample={SESSION_COUNT} samples_ms={samples_ms:?} p50_ms={p50:.3} p95_ms={p95:.3}"
        );
    }
}
