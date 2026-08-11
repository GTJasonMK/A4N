use std::fs;
use std::path::{Path, PathBuf};

use super::super::specs::SIDECAR_BIN_DIR_ENV;
use super::names::{
    sidecar_executable_candidate_names, sidecar_file_matches_tauri_suffix, sidecar_package_name,
};

pub(in crate::sidecar_clients::service_manager) fn sidecar_binary_path(
    service_name: &str,
) -> Option<PathBuf> {
    sidecar_binary_path_in_dirs(service_name, sidecar_binary_dirs())
}

pub(in crate::sidecar_clients::service_manager) fn sidecar_binary_path_in_dirs<I, P>(
    service_name: &str,
    dirs: I,
) -> Option<PathBuf>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let package = sidecar_package_name(service_name)?;
    let executable_names = sidecar_executable_candidate_names(&package);
    for dir in dirs {
        let dir = dir.as_ref();
        for executable in &executable_names {
            let candidate = dir.join(executable);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        if let Some(candidate) = unique_tauri_suffixed_sidecar_in_dir(dir, &package) {
            return Some(candidate);
        }
    }
    None
}

fn unique_tauri_suffixed_sidecar_in_dir(dir: &Path, package: &str) -> Option<PathBuf> {
    let mut matches = fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| sidecar_file_matches_tauri_suffix(name, package))
        })
        .collect::<Vec<_>>();
    matches.sort();
    if matches.len() == 1 {
        matches.pop()
    } else {
        None
    }
}

fn sidecar_binary_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = non_empty_env_path(SIDECAR_BIN_DIR_ENV) {
        dirs.push(dir);
    }
    if let Some(home) = non_empty_env_path("A4N_PORTABLE_HOME") {
        dirs.push(home.join("bin").join("services"));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(bin_dir) = exe.parent()
    {
        dirs.push(bin_dir.join("services"));
        dirs.push(bin_dir.to_path_buf());
        if let Some(contents_dir) = bin_dir.parent()
            && contents_dir
                .file_name()
                .is_some_and(|name| name == "Contents")
        {
            dirs.push(contents_dir.join("Resources").join("services"));
            dirs.push(contents_dir.join("Resources"));
        }
        if let Some(prefix) = bin_dir.parent() {
            dirs.push(prefix.join("lib").join("a4n").join("services"));
        }
    }
    dirs
}

fn non_empty_env_path(key: &str) -> Option<PathBuf> {
    std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}
