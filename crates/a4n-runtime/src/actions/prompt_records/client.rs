use crate::AppState;
use a4n_capabilities::PromptClient;
use std::sync::Arc;

mod hydration;
mod operations;

pub(super) use hydration::hydrate_prompt_sidecar_project_overrides;
pub(super) use operations::{
    sidecar_get_prompt, sidecar_prompt_list, sidecar_remove_project_override,
    sidecar_set_project_override,
};

pub(super) fn prompt_client(state: &AppState) -> Result<Option<Arc<dyn PromptClient>>, String> {
    prompt_client_for_profile(state.runtime_profile, state.capabilities())
}

pub(super) fn prompt_client_for_profile(
    runtime_profile: crate::state::RuntimeProfile,
    capabilities: &a4n_capabilities::A4nCapabilities,
) -> Result<Option<Arc<dyn PromptClient>>, String> {
    if !runtime_profile.uses_sidecars() {
        return Ok(None);
    }
    capabilities.prompt.clone().map(Some).ok_or_else(|| {
        "prompt sidecar capability 未配置，不能回退到 embedded prompt engine".to_string()
    })
}

fn prompt_client_required(state: &AppState) -> Result<Arc<dyn PromptClient>, String> {
    prompt_client(state)?.ok_or_else(|| "当前 runtime profile 未启用 prompt sidecar".to_string())
}

pub(super) fn prompt_sidecar_error(error: a4n_service_protocol::ServiceError) -> String {
    error.message
}

pub(super) fn prompt_service_request(
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    payload: impl serde::Serialize,
) -> Result<a4n_capabilities::JsonRequest, String> {
    use crate::sidecar_clients::{SidecarRequestContext, service_request_with_context};
    use a4n_service_protocol::SecretForwardingPolicy;
    match (project_id, root_project_id) {
        (None, None) | (Some(_), Some(_)) => {}
        (Some(project_id), None) => {
            return Err(format!(
                "prompt sidecar 项目请求缺少 root_project_id: project_id={project_id}"
            ));
        }
        (None, Some(root_project_id)) => {
            return Err(format!(
                "prompt sidecar 全局请求不能单独携带 root_project_id={root_project_id}"
            ));
        }
    }
    service_request_with_context(
        SidecarRequestContext::current_global()
            .with_project_scope(project_id, root_project_id)
            .with_secret_policy(SecretForwardingPolicy::deny_all()),
        a4n_service_protocol::PROMPT_RENDER_CAPABILITY_VERSION,
        payload,
    )
}
