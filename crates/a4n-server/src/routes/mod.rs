pub(crate) mod agents;
mod auth;
mod commands;
mod error;
mod generation;
mod media;
mod meta;
pub(crate) mod page_sessions;
mod temp_media;

pub use commands::dispatch_runtime_command;
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
        .route("/agents", get(agents::list).post(agents::register))
        .route(
            "/agents/{agent_id}",
            patch(agents::update).delete(agents::unregister),
        )
        .route("/agents/{agent_id}/heartbeat", post(agents::update))
        .route("/agents/events", get(agents::events))
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

async fn health() -> &'static str {
    "ok"
}
