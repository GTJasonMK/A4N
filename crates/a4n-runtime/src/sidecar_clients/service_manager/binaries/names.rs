use a4n_service_protocol::{
    EMBEDDING_SERVICE_NAME, EXPORT_SERVICE_NAME, IMAGE_SERVICE_NAME, IMPORT_SERVICE_NAME,
    LLM_SERVICE_NAME, PROMPT_SERVICE_NAME, RAG_SERVICE_NAME, VIDEO_SERVICE_NAME,
};

pub(in crate::sidecar_clients::service_manager) fn sidecar_executable_name(
    service_name: &str,
) -> Option<String> {
    sidecar_package_name(service_name).map(sidecar_platform_executable_name)
}

pub(super) fn sidecar_package_name(service_name: &str) -> Option<&'static str> {
    Some(match service_name {
        PROMPT_SERVICE_NAME => "a4n-prompt-service",
        LLM_SERVICE_NAME => "a4n-llm-gateway",
        EMBEDDING_SERVICE_NAME => "a4n-embedding-service",
        RAG_SERVICE_NAME => "a4n-rag-service",
        IMAGE_SERVICE_NAME => "a4n-image-service",
        VIDEO_SERVICE_NAME => "a4n-video-service",
        IMPORT_SERVICE_NAME => "a4n-import-service",
        EXPORT_SERVICE_NAME => "a4n-export-service",
        _ => return None,
    })
}

pub(super) fn sidecar_executable_candidate_names(package: &str) -> Vec<String> {
    let mut names = Vec::new();
    push_unique_executable_name(&mut names, sidecar_platform_executable_name(package));
    for target in current_tauri_target_triples() {
        push_unique_executable_name(&mut names, tauri_sidecar_executable_name(package, target));
    }
    names
}

fn push_unique_executable_name(names: &mut Vec<String>, name: String) {
    if !names.iter().any(|existing| existing == &name) {
        names.push(name);
    }
}

pub(in crate::sidecar_clients::service_manager) fn sidecar_platform_executable_name(
    stem: &str,
) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

pub(in crate::sidecar_clients::service_manager) fn tauri_sidecar_executable_name(
    package: &str,
    target_triple: &str,
) -> String {
    sidecar_platform_executable_name(&format!("{package}-{target_triple}"))
}

pub(super) fn sidecar_file_matches_tauri_suffix(file_name: &str, package: &str) -> bool {
    let extension = sidecar_executable_extension();
    let stem = if extension.is_empty() {
        if file_name.ends_with(".exe") {
            return false;
        }
        file_name
    } else {
        match file_name.strip_suffix(extension) {
            Some(stem) => stem,
            None => return false,
        }
    };

    let Some(suffix) = stem.strip_prefix(package) else {
        return false;
    };
    suffix.starts_with('-') && suffix.len() > 1
}

fn sidecar_executable_extension() -> &'static str {
    if cfg!(windows) { ".exe" } else { "" }
}

pub(in crate::sidecar_clients::service_manager) fn current_tauri_target_triples()
-> &'static [&'static str] {
    if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        &["x86_64-unknown-linux-gnu"]
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "musl"
    )) {
        &["x86_64-unknown-linux-musl"]
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "aarch64",
        target_env = "gnu"
    )) {
        &["aarch64-unknown-linux-gnu"]
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "aarch64",
        target_env = "musl"
    )) {
        &["aarch64-unknown-linux-musl"]
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        &["x86_64-apple-darwin"]
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        &["aarch64-apple-darwin"]
    } else if cfg!(all(
        target_os = "windows",
        target_arch = "x86_64",
        target_env = "msvc"
    )) {
        &["x86_64-pc-windows-msvc"]
    } else if cfg!(all(
        target_os = "windows",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        &["x86_64-pc-windows-gnu"]
    } else if cfg!(all(
        target_os = "windows",
        target_arch = "aarch64",
        target_env = "msvc"
    )) {
        &["aarch64-pc-windows-msvc"]
    } else if cfg!(all(
        target_os = "windows",
        target_arch = "x86",
        target_env = "msvc"
    )) {
        &["i686-pc-windows-msvc"]
    } else if cfg!(all(
        target_os = "windows",
        target_arch = "x86",
        target_env = "gnu"
    )) {
        &["i686-pc-windows-gnu"]
    } else {
        &[]
    }
}
