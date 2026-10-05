use std::path::PathBuf;

use a4n_config::optional_non_empty_env_path;

use super::super::specs::SIDECAR_BIN_DIR_ENV;
use super::names::sidecar_executable_name;

pub(in crate::sidecar_clients::service_manager) fn sidecar_binary_path(
    binary_name: &str,
) -> Result<Option<PathBuf>, String> {
    let executable = sidecar_executable_name(binary_name);
    if let Some(directory) =
        optional_non_empty_env_path(SIDECAR_BIN_DIR_ENV).map_err(|error| error.to_string())?
    {
        let binary = directory.join(executable);
        return Ok(binary.is_file().then_some(binary));
    }
    let exe = std::env::current_exe()
        .map_err(|error| format!("无法解析当前 Host 可执行文件路径: {error}"))?;
    let bin_dir = exe
        .parent()
        .ok_or_else(|| "当前 Host 可执行文件路径缺少父目录".to_string())?;
    let services = bin_dir.join("services");
    let resource_bin = bin_dir.join("bin");
    Ok([bin_dir.to_path_buf(), resource_bin, services]
        .into_iter()
        .map(|directory| directory.join(&executable))
        .find(|binary| binary.is_file()))
}
