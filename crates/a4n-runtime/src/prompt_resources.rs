use std::path::{Path, PathBuf};

use a4n_core::config::env::optional_non_empty_env_path;
use a4n_service_protocol::PROMPT_SERVICE_PROMPTS_DIR_ENV;

pub(crate) fn bundled_prompts_dir(data_dir: &Path) -> Result<PathBuf, String> {
    if let Some(path) = optional_non_empty_env_path(PROMPT_SERVICE_PROMPTS_DIR_ENV)
        .map_err(|error| error.to_string())?
    {
        return require_prompt_registry(path, PROMPT_SERVICE_PROMPTS_DIR_ENV);
    }
    if let Some(home) =
        optional_non_empty_env_path("A4N_PORTABLE_HOME").map_err(|error| error.to_string())?
    {
        return require_prompt_registry(home.join("prompts"), "A4N_PORTABLE_HOME");
    }

    let executable = std::env::current_exe()
        .map_err(|error| format!("无法解析当前 Host 可执行文件路径: {error}"))?;
    let bin_dir = executable
        .parent()
        .ok_or_else(|| "当前 Host 可执行文件路径缺少父目录".to_string())?;
    if bin_dir.file_name().is_some_and(|name| name == "MacOS")
        && let Some(contents) = bin_dir.parent()
        && contents.file_name().is_some_and(|name| name == "Contents")
    {
        return require_prompt_registry(contents.join("Resources/prompts"), "macOS bundle");
    }
    if bin_dir.file_name().is_some_and(|name| name == "bin")
        && let Some(package_root) = bin_dir.parent()
        && package_root.join("release-manifest.json").is_file()
    {
        return require_prompt_registry(package_root.join("prompts"), "portable package");
    }

    if let Some(path) = data_dir.parent().map(|home| home.join("prompts"))
        && path.join("_registry.yaml").is_file()
    {
        return Ok(path);
    }
    let path = std::env::current_dir()
        .map_err(|error| format!("无法解析当前工作目录: {error}"))?
        .join("prompts");
    if path.join("_registry.yaml").is_file() {
        return Ok(path);
    }
    Err("未找到随包 prompts 目录（已检查发布包、DATA_DIR 上级和 CWD）".to_string())
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
