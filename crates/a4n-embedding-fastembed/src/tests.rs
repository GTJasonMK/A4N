use a4n_core::error::AppError;

use crate::cache::{model_cache_path, required_files};
use crate::models::{
    DEFAULT_FASTEMBED_MODEL, model_status_from_info, remove_incomplete_fastembed_model_caches,
    resolve_model_info,
};
use crate::proxy::{
    HF_HOME_OVERRIDE_LOCK, ScopedEnvVar, SystemProxy, proxy_to_env_vars, proxy_url,
};
#[cfg(target_os = "linux")]
use crate::proxy::{parse_gsettings_array, parse_gsettings_string};

#[test]
fn resolves_model_by_enum_name() {
    let resolved = resolve_model_info(DEFAULT_FASTEMBED_MODEL).unwrap();
    assert_eq!(resolved.name, DEFAULT_FASTEMBED_MODEL);
    assert_eq!(resolved.info.dim, 512);
}

#[test]
fn rejects_ambiguous_model_code() {
    let err = resolve_model_info("Xenova/all-MiniLM-L12-v2").unwrap_err();
    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("多个 fastembed 变体"));
}

#[test]
fn builds_proxy_urls_only_when_host_and_port_are_present() {
    assert_eq!(
        proxy_url("http", "127.0.0.1", 7890),
        Some("http://127.0.0.1:7890".to_string())
    );
    assert_eq!(proxy_url("http", "", 7890), None);
    assert_eq!(proxy_url("http", "127.0.0.1", 0), None);
}

#[test]
fn system_proxy_only_fills_missing_explicit_protocols() {
    let _guard = HF_HOME_OVERRIDE_LOCK.lock().unwrap();
    let _explicit_http = ScopedEnvVar::set("HTTP_PROXY", "http://127.0.0.1:7001");
    let _explicit_http_lower = ScopedEnvVar::set("http_proxy", "http://127.0.0.1:7001");
    let _empty_https = ScopedEnvVar::set("HTTPS_PROXY", "");
    let _empty_https_lower = ScopedEnvVar::set("https_proxy", "");
    let _empty_all = ScopedEnvVar::set("ALL_PROXY", "");
    let _empty_all_lower = ScopedEnvVar::set("all_proxy", "");
    let _vars = proxy_to_env_vars(SystemProxy {
        http_proxy: Some("http://127.0.0.1:7002".to_string()),
        https_proxy: Some("http://127.0.0.1:7003".to_string()),
        all_proxy: Some("socks5h://127.0.0.1:7004".to_string()),
        ..SystemProxy::default()
    });

    assert_eq!(
        std::env::var("HTTP_PROXY").unwrap(),
        "http://127.0.0.1:7001"
    );
    assert_eq!(
        std::env::var("HTTPS_PROXY").unwrap(),
        "http://127.0.0.1:7003"
    );
    assert_eq!(
        std::env::var("ALL_PROXY").unwrap(),
        "socks5h://127.0.0.1:7004"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn parses_gsettings_proxy_values() {
    assert_eq!(parse_gsettings_string("'manual'"), "manual");
    assert_eq!(
        parse_gsettings_array("['localhost', '127.0.0.1', '*.local']"),
        vec!["localhost", "127.0.0.1", "*.local"]
    );
}

#[test]
fn reports_downloaded_when_required_snapshot_files_exist() {
    let dir = tempfile::tempdir().unwrap();
    let resolved = resolve_model_info(DEFAULT_FASTEMBED_MODEL).unwrap();
    let repo_path = model_cache_path(&resolved.info, dir.path());
    let snapshot = repo_path.join("snapshots").join("abc123");
    std::fs::create_dir_all(&snapshot).unwrap();
    std::fs::create_dir_all(repo_path.join("refs")).unwrap();
    std::fs::write(repo_path.join("refs").join("main"), "abc123").unwrap();

    for file in required_files(&resolved.info) {
        let path = snapshot.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "x").unwrap();
    }

    let status = model_status_from_info(resolved.info, dir.path()).unwrap();
    assert!(status.downloaded);
    assert!(status.size_bytes > 0);
}

#[test]
fn reports_missing_when_snapshot_is_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    let resolved = resolve_model_info(DEFAULT_FASTEMBED_MODEL).unwrap();
    let repo_path = model_cache_path(&resolved.info, dir.path());
    let snapshot = repo_path.join("snapshots").join("abc123");
    std::fs::create_dir_all(&snapshot).unwrap();
    std::fs::create_dir_all(repo_path.join("refs")).unwrap();
    std::fs::write(repo_path.join("refs").join("main"), "abc123").unwrap();
    let model_path = snapshot.join(&resolved.info.model_file);
    std::fs::create_dir_all(model_path.parent().unwrap()).unwrap();
    std::fs::write(model_path, "x").unwrap();

    let status = model_status_from_info(resolved.info, dir.path()).unwrap();
    assert!(!status.downloaded);
}

#[test]
fn removes_incomplete_model_cache_but_preserves_complete_cache() {
    let dir = tempfile::tempdir().unwrap();
    let resolved = resolve_model_info(DEFAULT_FASTEMBED_MODEL).unwrap();
    let repo_path = model_cache_path(&resolved.info, dir.path());
    std::fs::create_dir_all(repo_path.join("snapshots/incomplete")).unwrap();
    let lock_path = dir
        .path()
        .join(".locks")
        .join(crate::cache::repo_folder_name(&resolved.info.model_code));
    std::fs::create_dir_all(&lock_path).unwrap();

    let removed = remove_incomplete_fastembed_model_caches(dir.path()).unwrap();
    assert_eq!(removed.len(), 1);
    assert!(!repo_path.exists());
    assert!(!lock_path.exists());

    let snapshot = repo_path.join("snapshots/complete");
    std::fs::create_dir_all(&snapshot).unwrap();
    std::fs::create_dir_all(repo_path.join("refs")).unwrap();
    std::fs::write(repo_path.join("refs/main"), "complete").unwrap();
    for file in required_files(&resolved.info) {
        let path = snapshot.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "x").unwrap();
    }

    assert!(
        remove_incomplete_fastembed_model_caches(dir.path())
            .unwrap()
            .is_empty()
    );
    assert!(repo_path.exists());
}
