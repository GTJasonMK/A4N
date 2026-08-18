use crate::handlers::{
    categories, get_prompt, healthz, hydrate_overrides, list_prompts, load_defaults,
    remove_project_override, render_prompt, render_prompt_ab, reset_prompt, set_project_override,
    update_many_prompts, update_prompt, validate_prompt, version_diff, version_history,
};
use a4n_core::config::env::optional_non_empty_env_path;
use a4n_prompt::tera_engine::TeraPromptEngine;
use a4n_service_protocol::PROMPT_SERVICE_PROMPTS_DIR_ENV;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use std::{path::PathBuf, sync::Arc};

const PROMPT_GIT_OWNER: &str = "prompt-git";
const PROMPT_GIT_CLASSIFICATION: &str = "persistent_business_data";
const PROMPT_GIT_RECOVERY: &str = "restore_from_backup_or_refuse_startup";

#[derive(Clone)]
pub(crate) struct PromptServiceState {
    pub(crate) engine: Arc<TeraPromptEngine>,
}

pub(crate) async fn run() -> anyhow::Result<()> {
    init_tracing()?;

    let prompts_dir = prompts_dir()?;
    let git_dir = git_dir()?;
    ensure_prompt_git_manifest(&git_dir)?;
    let engine = TeraPromptEngine::from_directory(&prompts_dir)?.with_git_store(Arc::new(
        a4n_prompt_git::GitPromptStore::open_or_init(git_dir.clone())?,
    ))?;
    let state = PromptServiceState {
        engine: Arc::new(engine),
    };
    let addr =
        a4n_sidecar_bootstrap::service_socket_addr(a4n_service_protocol::PROMPT_SERVICE_NAME)?;

    tracing::info!(%addr, prompts_dir = ?prompts_dir, git_dir = ?git_dir, "starting prompt sidecar");
    a4n_sidecar_bootstrap::serve_sidecar(
        router(state),
        addr,
        a4n_service_protocol::PROMPT_SERVICE_NAME,
    )
    .await
}

pub(crate) async fn run_from_args() -> anyhow::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let Some(command) = args.next() else {
        return run().await;
    };
    let path = args.next().ok_or_else(|| {
        anyhow::anyhow!(
            "用法: a4n-prompt-service [backup <backup-dir> | restore <backup-dir>]；备份/恢复前必须先停止 prompt service"
        )
    })?;
    if args.next().is_some() {
        return Err(anyhow::anyhow!("backup/restore 只接受一个备份目录参数"));
    }

    let git_dir = git_dir()?;
    let backup_dir = PathBuf::from(path);
    match command.to_str() {
        Some("backup") => {
            require_prompt_git_manifest(&git_dir)?;
            a4n_prompt_git::backup_prompt_repository(&git_dir, &backup_dir)?;
            require_prompt_git_manifest(&backup_dir)?;
        }
        Some("restore") => {
            require_prompt_git_manifest(&backup_dir)?;
            a4n_prompt_git::restore_prompt_repository(&backup_dir, &git_dir)?;
            require_prompt_git_manifest(&git_dir)?;
        }
        _ => {
            return Err(anyhow::anyhow!(
                "未知命令 {:?}；支持 backup 或 restore",
                command
            ));
        }
    }
    Ok(())
}

fn init_tracing() -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::init_tracing(
        "A4N_PROMPT_SERVICE_LOG",
        "a4n_prompt_service=info,tower_http=warn",
    )
}

fn prompts_dir() -> anyhow::Result<PathBuf> {
    optional_non_empty_env_path(PROMPT_SERVICE_PROMPTS_DIR_ENV)?.ok_or_else(|| {
        anyhow::anyhow!("缺少 {PROMPT_SERVICE_PROMPTS_DIR_ENV}，无法定位随包 prompts 目录")
    })
}

fn git_dir() -> anyhow::Result<PathBuf> {
    if let Some(path) = optional_non_empty_env_path("A4N_PROMPT_SERVICE_GIT_DIR")? {
        return Ok(path);
    }
    let data_dir = optional_non_empty_env_path("A4N_DATA_DIR")?
        .ok_or_else(|| anyhow::anyhow!("缺少 A4N_DATA_DIR，无法定位 prompts.git"))?;
    Ok(data_dir.join("prompts.git"))
}

fn ensure_prompt_git_manifest(path: &std::path::Path) -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::ensure_data_directory_manifest(
        path,
        PROMPT_GIT_OWNER,
        PROMPT_GIT_CLASSIFICATION,
        1,
        true,
        PROMPT_GIT_RECOVERY,
        prompt_git_manifest_details(),
    )
}

fn require_prompt_git_manifest(path: &std::path::Path) -> anyhow::Result<()> {
    a4n_sidecar_bootstrap::require_data_directory_manifest(
        path,
        PROMPT_GIT_OWNER,
        PROMPT_GIT_CLASSIFICATION,
        1,
        true,
        PROMPT_GIT_RECOVERY,
        prompt_git_manifest_details(),
    )
}

fn prompt_git_manifest_details() -> serde_json::Value {
    serde_json::json!({"contents": "prompt templates and version history"})
}

fn router(state: PromptServiceState) -> Router {
    let policy = a4n_service_protocol::sidecar_resource_policy(
        a4n_service_protocol::PROMPT_SERVICE_NAME,
    )
    .expect("prompt sidecar must have a resource policy");
    Router::new()
        .route(a4n_service_protocol::SIDECAR_HEALTH_PATH, get(healthz))
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
        .route("/prompt/version_diff", post(version_diff))
        .route("/prompt/categories", post(categories))
        .layer(DefaultBodyLimit::max(
            policy.body_limit(a4n_service_protocol::BodyLimitKind::Json),
        ))
        .layer(a4n_sidecar_bootstrap::concurrency_limit(
            policy.max_concurrent_requests,
        ))
        .with_state(state)
}
