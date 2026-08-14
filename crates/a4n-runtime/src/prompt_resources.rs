use std::path::{Path, PathBuf};

use a4n_core::config::env::optional_non_empty_env_path;

pub(crate) fn bundled_prompts_dir(data_dir: &Path) -> Result<PathBuf, String> {
    if let Some(path) = optional_non_empty_env_path("A4N_PROMPT_SERVICE_PROMPTS_DIR")
        .map_err(|error| error.to_string())?
    {
        return require_prompt_registry(path, "A4N_PROMPT_SERVICE_PROMPTS_DIR");
    }
    if let Some(home) =
        optional_non_empty_env_path("A4N_PORTABLE_HOME").map_err(|error| error.to_string())?
    {
        return require_prompt_registry(home.join("prompts"), "A4N_PORTABLE_HOME");
    }

    let mut candidates = Vec::new();
    if let Some(home) = data_dir.parent() {
        candidates.push(home.join("prompts"));
    }
    if let Ok(current) = std::env::current_dir() {
        candidates.push(current.join("prompts"));
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(bin_dir) = executable.parent()
    {
        candidates.push(bin_dir.join("prompts"));
        if let Some(package_root) = bin_dir.parent() {
            candidates.push(package_root.join("prompts"));
            candidates.push(package_root.join("Resources").join("prompts"));
            candidates.push(package_root.join("share").join("a4n").join("prompts"));
        }
    }

    candidates
        .into_iter()
        .find(|path| path.join("_registry.yaml").is_file())
        .ok_or_else(|| {
            "未找到随包 prompts 目录（已检查 DATA_DIR 上级、CWD、EXE_DIR、Resources 和 share/a4n）"
                .to_string()
        })
}

fn require_prompt_registry(path: PathBuf, source: &str) -> Result<PathBuf, String> {
    let registry = path.join("_registry.yaml");
    if registry.is_file() {
        Ok(path)
    } else {
        Err(format!(
            "{source} 指向的 prompts 目录缺少注册表：{}",
            registry.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::require_prompt_registry;

    #[test]
    fn explicit_prompt_directory_requires_registry() {
        let dir = tempfile::tempdir().unwrap();
        assert!(require_prompt_registry(dir.path().to_path_buf(), "test").is_err());

        std::fs::write(
            dir.path().join("_registry.yaml"),
            "prompts: {}\ncategories: {}\n",
        )
        .unwrap();
        assert_eq!(
            require_prompt_registry(dir.path().to_path_buf(), "test").unwrap(),
            dir.path()
        );
    }
}
