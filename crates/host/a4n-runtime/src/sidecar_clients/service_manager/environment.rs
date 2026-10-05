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
    let executable =
        std::env::current_exe().map_err(|error| format!("无法解析 Host 二进制路径: {error}"))?;
    apply_component_loader_paths(env, &executable)
}

fn apply_component_loader_paths(
    env: &mut HashMap<String, String>,
    executable: &Path,
) -> Result<(), String> {
    let runtime_dirs = component_runtime_dirs(executable);
    for key in ["LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH"] {
        let mut paths = runtime_dirs.clone();
        if let Some(current) = std::env::var_os(key) {
            paths.extend(std::env::split_paths(&current));
        }
        env.insert(
            key.to_string(),
            std::env::join_paths(paths)
                .map_err(|error| format!("构造 {key} 失败: {error}"))?
                .to_string_lossy()
                .into_owned(),
        );
    }
    if cfg!(windows) {
        let current = env
            .get("PATH")
            .cloned()
            .or_else(|| std::env::var("PATH").ok());
        let mut paths = runtime_dirs;
        if let Some(current) = current {
            paths.extend(std::env::split_paths(&current));
        }
        env.insert(
            "PATH".to_string(),
            std::env::join_paths(paths)
                .map_err(|error| format!("构造 PATH 失败: {error}"))?
                .to_string_lossy()
                .into_owned(),
        );
    }
    Ok(())
}

fn component_runtime_dirs(executable: &Path) -> Vec<PathBuf> {
    let Some(binary_dir) = executable.parent() else {
        return Vec::new();
    };
    let mut directories = vec![binary_dir.join("services"), binary_dir.join("bin/services")];
    if binary_dir.file_name().is_some_and(|name| name == "MacOS")
        && let Some(contents_dir) = binary_dir.parent()
        && contents_dir
            .file_name()
            .is_some_and(|name| name == "Contents")
    {
        directories.insert(0, contents_dir.join("Resources"));
    } else {
        directories.push(binary_dir.join("resources"));
    }
    directories
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_loader_paths_prefer_host_relative_shared_runtimes() {
        let executable = Path::new("/fixture/a4n/bin/a4n-server");
        let mut env = HashMap::new();
        apply_component_loader_paths(&mut env, executable)
            .expect("component loader paths should be valid");
        let runtime_dirs = component_runtime_dirs(executable);

        let loader_variables: &[&str] = if cfg!(windows) {
            &["PATH"]
        } else {
            &["LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH"]
        };
        for variable in loader_variables {
            let paths = std::env::split_paths(std::ffi::OsStr::new(
                env.get(*variable)
                    .expect("shared library path should be populated"),
            ))
            .collect::<Vec<_>>();
            assert!(paths.starts_with(&runtime_dirs));
        }
    }

    #[test]
    fn macos_bundle_resources_are_first_for_sidecar_libraries() {
        let executable = Path::new("/Applications/A4N.app/Contents/MacOS/a4n-server");
        let directories = component_runtime_dirs(executable);
        assert_eq!(
            directories.first(),
            Some(&PathBuf::from("/Applications/A4N.app/Contents/Resources"))
        );
    }
}
