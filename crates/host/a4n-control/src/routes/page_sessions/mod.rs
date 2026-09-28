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
        assert_eq!(navigated.context_revision, 2);
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
}
