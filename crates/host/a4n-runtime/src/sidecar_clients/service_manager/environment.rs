use std::collections::HashMap;
use std::path::{Path, PathBuf};

use a4n_config::app_config::AppConfig;
use a4n_config::{optional_non_empty_env_path, optional_non_empty_env_string};
use a4n_service_protocol::{EMBEDDING_CACHE_DIR_ENV, PROMPT_SERVICE_PROMPTS_DIR_ENV};

pub(super) fn apply_common_sidecar_env(
    env: &mut HashMap<String, String>,
    config: &AppConfig,
) -> Result<(), String> {
    env.insert("A4N_DATA_DIR".to_string(), path_string(&config.data_dir)?);
    env.insert("A4N_CACHE_DIR".to_string(), path_string(&config.cache_dir)?);
    env.insert("A4N_TEMP_DIR".to_string(), path_string(&config.temp_dir)?);
    env.insert("A4N_LOG_DIR".to_string(), path_string(&config.log_dir)?);
    let rag_data_dir = configured_path("A4N_RAG_SERVICE_DATA_DIR", &config.data_dir)?;
    env.insert(
        "A4N_RAG_SERVICE_DATA_DIR".to_string(),
        path_string(&rag_data_dir)?,
    );
    let embedding_cache_dir =
        configured_path(EMBEDDING_CACHE_DIR_ENV, &config.embedding_models_dir())?;
    env.insert(
        EMBEDDING_CACHE_DIR_ENV.to_string(),
        path_string(&embedding_cache_dir)?,
    );
    let prompts_dir = crate::prompt_resources::bundled_prompts_dir(&config.data_dir)?;
    env.insert(
        PROMPT_SERVICE_PROMPTS_DIR_ENV.to_string(),
        path_string(&prompts_dir)?,
    );
    for key in [
        "PATH",
        "LD_LIBRARY_PATH",
        "DYLD_LIBRARY_PATH",
        "A4N_ONNXRUNTIME_ROOT",
        "ORT_LIB_PATH",
        "ORT_PREFER_DYNAMIC_LINK",
        "ORT_DYLIB_PATH",
        "A4N_QDRANT_URL",
        "A4N_QDRANT_COLLECTION",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
        "RUST_LOG",
    ] {
        if let Some(value) =
            optional_non_empty_env_string(key).map_err(|error| error.to_string())?
        {
            env.insert(key.to_string(), value);
        }
    }
    Ok(())
}

fn configured_path(key: &str, default_path: &Path) -> Result<PathBuf, String> {
    optional_non_empty_env_path(key)
        .map(|path| path.unwrap_or_else(|| default_path.to_path_buf()))
        .map_err(|error| error.to_string())
}

fn path_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| format!("Sidecar 环境路径不是有效 Unicode: {}", path.display()))
}
