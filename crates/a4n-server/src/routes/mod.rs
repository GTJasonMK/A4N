mod admin;
mod auth;
mod commands;
mod error;
mod events;
mod media;
mod projects;
mod source_sync;
mod temp_media;

use crate::state::ServerState;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

const COMMAND_BODY_LIMIT_BYTES: usize = 32 * 1024 * 1024;
const TEMP_MEDIA_BODY_LIMIT_BYTES: usize = 32 * 1024 * 1024;

pub fn api_router(state: ServerState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/auth/login", post(auth::login))
        .route("/auth/register", post(auth::register))
        .route("/auth/me", get(auth::me))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/refresh", post(auth::refresh))
        .route(
            "/commands/{cmd}",
            post(commands::invoke).layer(DefaultBodyLimit::max(COMMAND_BODY_LIMIT_BYTES)),
        )
        .route("/events/{channel}", get(events::subscribe))
        .route("/media", get(media::serve))
        .route(
            "/temp-media",
            post(temp_media::upload).layer(DefaultBodyLimit::max(TEMP_MEDIA_BODY_LIMIT_BYTES)),
        )
        .route("/temp-media/{file_name}", get(temp_media::serve))
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
        .route(
            "/source-sync/preview",
            post(source_sync::preview_source_sync),
        )
        .route(
            "/source-sync/apply",
            post(source_sync::apply_source_sync_decisions),
        )
        .route(
            "/source-sync/detach",
            post(source_sync::detach_derived_project),
        )
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
