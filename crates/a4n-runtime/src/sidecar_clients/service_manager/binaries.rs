mod discovery;
mod names;

pub(super) use discovery::sidecar_binary_path;
#[cfg(test)]
pub(super) use discovery::sidecar_binary_path_in_dirs;
pub(super) use names::sidecar_executable_name;
#[cfg(test)]
pub(super) use names::{
    current_tauri_target_triples, sidecar_platform_executable_name, tauri_sidecar_executable_name,
};
