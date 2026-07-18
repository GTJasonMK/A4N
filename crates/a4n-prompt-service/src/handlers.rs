mod read;
mod write;

use a4n_service_protocol::{
    HealthStatus, PROMPT_RENDER_CAPABILITY_VERSION as PROMPT_CAPABILITY_VERSION,
    PROMPT_SERVICE_NAME, PROTOCOL_VERSION, ServiceHealth, ServiceVersion,
};
use axum::Json;

pub(crate) use read::{
    categories, dependency_graph, get_prompt, list_prompts, render_prompt, render_prompt_ab,
    validate_prompt, version_diff, version_get, version_history,
};
pub(crate) use write::{
    hydrate_overrides, load_defaults, remove_project_override, reset_prompt, set_project_override,
    update_many_prompts, update_prompt,
};

pub(crate) async fn healthz() -> Json<ServiceHealth> {
    Json(ServiceHealth {
        service: PROMPT_SERVICE_NAME.to_string(),
        status: HealthStatus::Healthy,
        version: ServiceVersion {
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: PROTOCOL_VERSION,
            capability_version: PROMPT_CAPABILITY_VERSION.to_string(),
        },
    })
}
