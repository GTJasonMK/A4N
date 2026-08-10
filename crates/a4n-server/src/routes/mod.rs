mod admin;
mod auth;
mod commands;
mod error;
mod generation;
mod media;
mod meta;
mod projects;
mod temp_media;

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
        .route("/auth/login", post(auth::login))
        .route("/auth/register", post(auth::register))
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/refresh", post(auth::refresh))
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
        .route("/admin/users", get(admin::list_users))
        .route("/admin/users", post(admin::create_user))
        .route("/admin/invite-codes", get(admin::list_invite_codes))
        .route("/admin/invite-codes", post(admin::create_invite_code))
        .route("/admin/users/{user_id}/role", post(admin::update_user_role))
        .route("/admin/users/{user_id}/disable", post(admin::disable_user))
        .route("/projects", get(projects::list_projects))
        .route("/projects", post(projects::create_project))
        .route("/projects/derived", post(projects::create_derived_project))
        .route(
            "/projects/freeform-video",
            post(projects::create_freeform_video_project),
        )
        .route(
            "/projects/{project_id}/source-snapshot-chapters",
            get(projects::list_source_snapshot_chapters),
        )
        .route("/projects/{project_id}", get(projects::get_project))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
