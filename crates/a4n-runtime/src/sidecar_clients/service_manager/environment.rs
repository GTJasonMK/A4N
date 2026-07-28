use std::collections::HashMap;
use std::path::PathBuf;

use a4n_core::config::app_config::AppConfig;
use a4n_core::config::env::optional_non_empty_env_path;

pub(super) fn apply_common_sidecar_env(
    env: &mut HashMap<String, String>,
    config: &AppConfig,
) -> Result<(), String> {
    env.insert(
        "A4N_DATA_DIR".to_string(),
        path_string(config.data_dir.clone()),
    );
    env.insert(
        "A4N_CACHE_DIR".to_string(),
        path_string(config.cache_dir.clone()),
    );
    env.insert(
        "A4N_TEMP_DIR".to_string(),
        path_string(config.temp_dir.clone()),
    );
    env.insert(
        "A4N_LOG_DIR".to_string(),
        path_string(config.log_dir.clone()),
    );
    let rag_data_dir = configured_path("A4N_RAG_SERVICE_DATA_DIR", config.data_dir.clone())?;
    env.insert(
        "A4N_RAG_SERVICE_DATA_DIR".to_string(),
        path_string(rag_data_dir),
    );
    let embedding_cache_dir = configured_path(
        "A4N_EMBEDDING_CACHE_DIR",
        config.cache_dir.join("embedding-models"),
    )?;
    env.insert(
        "A4N_EMBEDDING_CACHE_DIR".to_string(),
        path_string(embedding_cache_dir),
    );
    env.insert(
        "A4N_PROMPT_SERVICE_PROMPTS_DIR".to_string(),
        path_string(prompt_service_prompts_dir(config)?),
    );
    for key in [
        "PATH",
        "LD_LIBRARY_PATH",
        "DYLD_LIBRARY_PATH",
        "A4N_ONNXRUNTIME_ROOT",
        "ORT_DYLIB_PATH",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
        "RUST_LOG",
    ] {
        if let Ok(value) = std::env::var(key)
            && !value.trim().is_empty()
        {
            env.insert(key.to_string(), value);
        }
    }
    Ok(())
}

fn configured_path(key: &str, fallback: PathBuf) -> Result<PathBuf, String> {
    optional_non_empty_env_path(key)
        .map(|path| path.unwrap_or(fallback))
        .map_err(|error| error.to_string())
}

fn prompt_service_prompts_dir(config: &AppConfig) -> Result<PathBuf, String> {
    if let Some(path) = optional_non_empty_env_path("A4N_PROMPT_SERVICE_PROMPTS_DIR")
        .map_err(|error| error.to_string())?
    {
        return Ok(path);
    }
    if let Some(home) =
        optional_non_empty_env_path("A4N_PORTABLE_HOME").map_err(|error| error.to_string())?
    {
        return Ok(home.join("prompts"));
    }
    if let Ok(current) = std::env::current_dir() {
        let prompts = current.join("prompts");
        if prompts.exists() {
            return Ok(prompts);
        }
    }
    Ok(config.data_dir.join("prompts"))
}

fn path_string(path: PathBuf) -> String {
    path.display().to_string()
}
