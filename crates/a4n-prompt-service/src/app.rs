use crate::handlers::{
    categories, dependency_graph, get_prompt, healthz, hydrate_overrides, list_prompts,
    load_defaults, remove_project_override, render_prompt, render_prompt_ab, reset_prompt,
    set_project_override, update_many_prompts, update_prompt, validate_prompt, version_diff,
    version_get, version_history,
};
use a4n_core::config::env::optional_non_empty_env_path;
use a4n_prompt::tera_engine::TeraPromptEngine;
use axum::{Router, routing::get, routing::post};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};

#[derive(Clone)]
pub(crate) struct PromptServiceState {
    pub(crate) engine: Arc<TeraPromptEngine>,
}

pub(crate) async fn run() -> anyhow::Result<()> {
    init_tracing()?;

    let prompts_dir = prompts_dir()?;
    let git_dir = git_dir()?;
    let engine = TeraPromptEngine::from_directory(&prompts_dir)?.with_git_store(Arc::new(
        a4n_prompt_git::GitPromptStore::open_or_init(git_dir.clone())?,
    ))?;
    let state = PromptServiceState {
        engine: Arc::new(engine),
    };
    let addr = service_addr()?;
    let app = router(state);

    tracing::info!(%addr, prompts_dir = ?prompts_dir, git_dir = ?git_dir, "starting prompt sidecar");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_PROMPT_SERVICE_LOG",
        "a4n_prompt_service=info,tower_http=warn",
    )
}

fn prompts_dir() -> anyhow::Result<PathBuf> {
    Ok(
        optional_non_empty_env_path("A4N_PROMPT_SERVICE_PROMPTS_DIR")?
            .unwrap_or_else(|| PathBuf::from("prompts")),
    )
}

fn git_dir() -> anyhow::Result<PathBuf> {
    if let Some(path) = optional_non_empty_env_path("A4N_PROMPT_SERVICE_GIT_DIR")? {
        return Ok(path);
    }
    Ok(optional_non_empty_env_path("A4N_DATA_DIR")?
        .map(|dir| dir.join("prompts.git"))
        .unwrap_or_else(|| PathBuf::from("data/prompts.git")))
}

fn service_addr() -> anyhow::Result<SocketAddr> {
    a4n_sidecar_bootstrap::socket_addr("A4N_PROMPT_SERVICE_ADDR", "127.0.0.1:47641")
}

fn router(state: PromptServiceState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/prompt/list", post(list_prompts))
        .route("/prompt/get", post(get_prompt))
        .route("/prompt/render", post(render_prompt))
        .route("/prompt/validate", post(validate_prompt))
        .route("/prompt/render_ab", post(render_prompt_ab))
        .route("/prompt/override/hydrate", post(hydrate_overrides))
        .route("/prompt/override/set", post(set_project_override))
        .route("/prompt/override/remove", post(remove_project_override))
        .route("/prompt/load_defaults", post(load_defaults))
        .route("/prompt/update", post(update_prompt))
        .route("/prompt/update_many", post(update_many_prompts))
        .route("/prompt/reset", post(reset_prompt))
        .route("/prompt/version_history", post(version_history))
        .route("/prompt/version_get", post(version_get))
        .route("/prompt/version_diff", post(version_diff))
        .route("/prompt/dependency_graph", post(dependency_graph))
        .route("/prompt/categories", post(categories))
        .with_state(state)
}
