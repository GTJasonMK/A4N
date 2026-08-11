mod auth;
mod commands;
mod error;
mod generation;
mod media;
mod meta;
mod temp_media;

pub use commands::dispatch_runtime_command;
pub use generation::dispatch_generation_start;

use crate::state::ServerState;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

const COMMAND_BODY_LIMIT_BYTES: usize = 32 * 1024 * 1024;
const TEMP_MEDIA_BODY_LIMIT_BYTES: usize = 32 * 1024 * 1024;
const VIDEO_GENERATION_BODY_LIMIT_BYTES: usize = 128 * 1024 * 1024;

pub fn api_router(state: ServerState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/meta", get(meta::get))
        .route("/meta/commands", get(meta::commands))
        .route("/meta/commands/{command}", get(meta::command))
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
