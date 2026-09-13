mod agent_runtime;
pub(crate) mod agents;
mod auth;
mod commands;
mod error;
mod generation;
mod media;
mod meta;
pub(crate) mod page_sessions;
mod temp_media;

pub use commands::{dispatch_runtime_command, dispatch_runtime_command_value};
pub use generation::dispatch_generation_start;

use crate::state::ServerState;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, patch, post};

const COMMAND_BODY_LIMIT_BYTES: usize = 32 * 1024 * 1024;
const TEMP_MEDIA_BODY_LIMIT_BYTES: usize = 32 * 1024 * 1024;
const VIDEO_GENERATION_BODY_LIMIT_BYTES: usize = 128 * 1024 * 1024;

pub fn api_router(state: ServerState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/meta", get(meta::get))
        .route("/meta/commands", get(meta::commands))
        .route("/meta/commands/{command}", get(meta::command))
        .route("/meta/packages", get(meta::packages))
        .route("/agents", get(agents::list).post(agents::register))
        .route(
            "/agents/{agent_id}",
            patch(agents::update).delete(agents::unregister),
        )
        .route("/agents/{agent_id}/heartbeat", post(agents::update))
        .route("/agents/events", get(agents::events))
        .route(
            "/agent-threads",
            get(agent_runtime::list_threads).post(agent_runtime::create_thread),
        )
        .route(
            "/agent-threads/default",
            post(agent_runtime::ensure_main_thread),
        )
        .route(
            "/agent-threads/{thread_id}",
            get(agent_runtime::show_thread).patch(agent_runtime::update_thread),
        )
        .route(
            "/agent-threads/{thread_id}/fork",
            post(agent_runtime::fork_thread),
        )
        .route(
            "/agent-threads/{thread_id}/archive",
            post(agent_runtime::archive_thread),
        )
        .route(
            "/agent-threads/{thread_id}/messages",
            get(agent_runtime::list_messages).post(agent_runtime::send_message),
        )
        .route(
            "/agent-threads/{thread_id}/events",
            get(agent_runtime::events),
        )
        .route(
            "/agent-runs/{run_id}/control",
            post(agent_runtime::control_run),
        )
        .route("/agent-runs/{run_id}", get(agent_runtime::show_run))
        .route(
            "/agent-runs/{parent_run_id}/children",
            post(agent_runtime::spawn_agent),
        )
        .route(
            "/agent-delegations/{delegation_id}/messages",
            post(agent_runtime::send_message_to_child),
        )
        .route(
            "/agent-delegations/{delegation_id}/control",
            post(agent_runtime::control_child_agent),
        )
        .route("/agent-tools", get(agent_runtime::search_tools))
        .route("/agent-tools/{name}", get(agent_runtime::describe_tool))
        .route(
            "/agent-memories",
            get(agent_runtime::list_memories).post(agent_runtime::create_memory),
        )
        .route(
            "/agent-memories/{memory_id}/retract",
            post(agent_runtime::retract_memory),
        )
        .route(
            "/agent-approvals/{approval_id}",
            post(agent_runtime::resolve_approval),
        )
        .route(
            "/ui-sessions",
            get(page_sessions::list).post(page_sessions::create),
        )
        .route("/ui-sessions/events", get(page_sessions::events))
        .route("/ui-sessions/{session_id}", get(page_sessions::get))
        .route(
            "/ui-sessions/{session_id}/authorize",
            post(page_sessions::authorize),
        )
        .route(
            "/ui-sessions/{session_id}/attach",
            post(page_sessions::attach),
        )
        .route("/ui-sessions/attach", post(page_sessions::attach_by_code))
        .route(
            "/ui-sessions/{session_id}/commands",
            get(page_sessions::commands),
        )
        .route(
            "/ui-sessions/{session_id}/context",
            patch(page_sessions::context),
        )
        .route(
            "/ui-sessions/{session_id}/heartbeat",
            post(page_sessions::heartbeat),
        )
        .route(
            "/ui-sessions/{session_id}/operations",
            post(page_sessions::start_operation),
        )
        .route(
            "/ui-sessions/{session_id}/operations/{operation_id}",
            patch(page_sessions::update_operation),
        )
        .route(
            "/ui-sessions/{session_id}/navigate",
            post(page_sessions::navigate),
        )
        .route(
            "/ui-sessions/{session_id}/control/{action}",
            post(page_sessions::control),
        )
        .route(
            "/commands/{cmd}",
            post(commands::invoke).layer(DefaultBodyLimit::max(COMMAND_BODY_LIMIT_BYTES)),
        )
        .route("/generation-tasks/chapter", post(generation::start_chapter))
        .route(
            "/generation-tasks/inspiration",
            post(generation::start_inspiration),
        )
        .route(
            "/generation-tasks/blueprint",
            post(generation::start_blueprint),
        )
        .route(
            "/generation-tasks/blueprint-refinement",
            post(generation::start_blueprint_refinement),
        )
        .route("/generation-tasks/outline", post(generation::start_outline))
        .route(
            "/generation-tasks/portrait",
            post(generation::start_portrait),
        )
        .route("/generation-tasks/image", post(generation::start_image))
        .route(
            "/generation-tasks/video",
            post(generation::start_video)
                .layer(DefaultBodyLimit::max(VIDEO_GENERATION_BODY_LIMIT_BYTES)),
        )
        .route(
            "/generation-tasks/optimize-chat",
            post(generation::start_optimize_chat),
        )
        .route(
            "/generation-tasks/optimize-analysis",
            post(generation::start_optimize_analysis),
        )
        .route(
            "/generation-tasks/optimize-patch",
            post(generation::start_optimize_patch),
        )
        .route(
            "/generation-tasks/{task_id}/events",
            get(generation::events),
        )
        .route(
            "/generation-tasks/{task_id}/snapshot",
            get(generation::snapshot),
        )
        .route(
            "/generation-tasks/{task_id}/cancel",
            post(generation::cancel),
        )
        .route("/generation-tasks/active", get(generation::active))
        .route("/generation-tasks/history", get(generation::history))
        .route("/generation-tasks/{task_id}", get(generation::task))
        .route("/media", get(media::serve))
        .route(
            "/temp-media",
            post(temp_media::upload).layer(DefaultBodyLimit::max(TEMP_MEDIA_BODY_LIMIT_BYTES)),
        )
        .route(
            "/temp-media/{file_name}",
            get(temp_media::serve).delete(temp_media::delete),
        )
        .with_state(state)
}

pub fn internal_router(state: ServerState) -> Router {
    Router::new()
        .route("/agent/claim", post(agent_runtime::claim_worker))
        .route("/agent/context", post(agent_runtime::load_agent_context))
        .route("/agent/lease/renew", post(agent_runtime::renew_agent_lease))
        .route(
            "/agent/activation",
            post(agent_runtime::activate_agent_turn),
        )
        .route("/agent/usage", post(agent_runtime::update_agent_usage))
        .route("/agent/wait-run", post(agent_runtime::wait_agent_run))
        .route(
            "/agent/context-summary",
            post(agent_runtime::update_agent_context_summary),
        )
        .route(
            "/agent/execution/lookup",
            post(agent_runtime::worker_execution_by_key),
        )
        .route(
            "/agent/execution/prepare",
            post(agent_runtime::create_prepared_execution),
        )
        .route(
            "/agent/execution/update",
            post(agent_runtime::update_worker_execution),
        )
        .route(
            "/agent/execution/result",
            post(agent_runtime::read_execution_result),
        )
        .route("/agent/memory", post(agent_runtime::read_memory))
        .route("/agent/history", post(agent_runtime::read_worker_history))
        .route("/agent/inbox/claim", post(agent_runtime::claim_agent_inbox))
        .route(
            "/agent/inbox/consume",
            post(agent_runtime::consume_agent_inbox),
        )
        .route(
            "/agent/inbox/persist",
            post(agent_runtime::persist_agent_inbox),
        )
        .route(
            "/agent/inbox/message",
            post(agent_runtime::load_agent_inbox_message),
        )
        .route("/agent/message", post(agent_runtime::apply_message_request))
        .route("/agent/budget", post(agent_runtime::worker_budget_snapshot))
        .route("/agent/failure", post(agent_runtime::fail_agent_worker_run))
        .route(
            "/agent/result-refs",
            post(agent_runtime::result_refs_for_agent_worker_run),
        )
        .route(
            "/agent/completion",
            post(agent_runtime::complete_agent_worker_run),
        )
        .route(
            "/agent/approval/check",
            post(agent_runtime::check_agent_worker_approval),
        )
        .route(
            "/agent/approval/request",
            post(agent_runtime::request_agent_worker_approval),
        )
        .route(
            "/agent/delegation/review",
            post(agent_runtime::review_agent_delegation),
        )
        .route(
            "/agent/delegation/thread",
            post(agent_runtime::ensure_agent_canonical_subagent_thread),
        )
        .route(
            "/agent/event",
            post(agent_runtime::append_agent_worker_update),
        )
        .route(
            "/agent/delegation/start",
            post(agent_runtime::create_agent_child_run),
        )
        .route(
            "/agent/run-status",
            post(agent_runtime::agent_worker_run_is_terminal),
        )
        .route("/agent/command", post(agent_runtime::command))
        .route(
            "/agent/package",
            get(agent_runtime::agent_package_execution_enabled),
        )
        .route(
            "/agent/generation/cancel",
            post(agent_runtime::cancel_agent_generation),
        )
        .route(
            "/agent/workflow/cancel",
            post(agent_runtime::cancel_agent_workflow),
        )
        .route("/agent/wait", post(agent_runtime::read_agent_wait))
        .route(
            "/agent/llm/capabilities",
            post(agent_runtime::agent_llm_capabilities),
        )
        .route("/agent/llm/chat", post(agent_runtime::agent_chat))
        .route(
            "/agent/editor/project",
            post(agent_runtime::project_agent_editor),
        )
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
