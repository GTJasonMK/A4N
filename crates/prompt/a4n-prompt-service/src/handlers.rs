mod read;
mod write;

use a4n_service_protocol::{
    HealthStatus, PROMPT_SERVICE_NAME, ServiceHealth, known_service_health,
};
use axum::Json;

pub(crate) use read::{
    categories, get_prompt, list_prompts, render_prompt, render_prompt_ab, validate_prompt,
    version_diff, version_history,
};
pub(crate) use write::{
    hydrate_overrides, load_defaults, remove_project_override, reset_prompt, set_project_override,
    update_many_prompts, update_prompt,
};

pub(crate) async fn healthz() -> Json<ServiceHealth> {
    Json(known_service_health(
        PROMPT_SERVICE_NAME,
        env!("CARGO_PKG_VERSION"),
        HealthStatus::Healthy,
    ))
}
