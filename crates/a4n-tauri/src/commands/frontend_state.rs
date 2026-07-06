use a4n_core::files::paths::AppPaths;
use a4n_runtime::AppState;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;
use tauri::State;

const STATE_VERSION: u32 = 1;
const MAX_KEY_LEN: usize = 256;
const MAX_VALUE_BYTES: usize = 10 * 1024 * 1024;

static FRONTEND_STATE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FrontendStateFile {
    version: u32,
    #[serde(default)]
    entries: BTreeMap<String, String>,
}

impl Default for FrontendStateFile {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            entries: BTreeMap::new(),
        }
    }
}

#[tauri::command]
pub fn get_all_frontend_state(
    state: State<'_, AppState>,
) -> Result<BTreeMap<String, String>, String> {
    let _guard = frontend_state_lock().lock();
    let path = frontend_state_path(&state);
    Ok(read_state_file(&path)?.entries)
}

#[tauri::command]
pub fn set_frontend_state_many(
    state: State<'_, AppState>,
    entries: BTreeMap<String, String>,
) -> Result<(), String> {
    for (key, value) in &entries {
        validate_key(key)?;
        validate_value(value)?;
    }

    let _guard = frontend_state_lock().lock();
    let path = frontend_state_path(&state);
    let mut file = read_state_file(&path)?;
    for (key, value) in entries {
        file.entries.insert(key, value);
    }
    write_state_file(&path, &file)
}

#[tauri::command]
pub fn remove_frontend_state_many(
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> Result<(), String> {
    for key in &keys {
        validate_key(key)?;
    }

    let _guard = frontend_state_lock().lock();
    let path = frontend_state_path(&state);
    let mut file = read_state_file(&path)?;
    for key in keys {
        file.entries.remove(&key);
    }
    write_state_file(&path, &file)
}

fn frontend_state_lock() -> &'static Mutex<()> {
    FRONTEND_STATE_LOCK.get_or_init(|| Mutex::new(()))
}

fn frontend_state_path(state: &AppState) -> std::path::PathBuf {
    AppPaths::from_config(&state.config).frontend_state_file()
}

fn read_state_file(path: &Path) -> Result<FrontendStateFile, String> {
    if !path.exists() {
        return Ok(FrontendStateFile::default());
    }
    ensure_state_path_is_file(path)?;

    let content = std::fs::read_to_string(path).map_err(|e| format!("读取前端状态失败：{e}"))?;
    let file: FrontendStateFile =
        serde_json::from_str(&content).map_err(|e| format!("解析前端状态失败：{e}"))?;
    if file.version != STATE_VERSION {
        return Err(format!(
            "不支持的前端状态版本：{}，当前支持版本：{}",
            file.version, STATE_VERSION
        ));
    }
    for (key, value) in &file.entries {
        validate_key(key).map_err(|e| format!("前端状态文件包含非法 key {key:?}：{e}"))?;
        validate_value(value).map_err(|e| format!("前端状态文件包含非法值（key {key:?}）：{e}"))?;
    }
    Ok(file)
}

fn write_state_file(path: &Path, file: &FrontendStateFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建前端状态目录失败：{e}"))?;
    }
    ensure_state_path_is_file(path)?;

    let content =
        serde_json::to_string_pretty(file).map_err(|e| format!("序列化前端状态失败：{e}"))?;
    std::fs::write(path, content).map_err(|e| format!("写入前端状态失败：{e}"))
}

fn ensure_state_path_is_file(path: &Path) -> Result<(), String> {
    if path.exists() && !path.is_file() {
        return Err("前端状态路径不是文件".into());
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("前端状态 key 不能为空".into());
    }
    if key.len() > MAX_KEY_LEN {
        return Err("前端状态 key 过长".into());
    }
    if !(key.starts_with("a4n:") || key.starts_with("a4n-")) {
        return Err("前端状态 key 必须使用 a4n 前缀".into());
    }
    if key.chars().any(|ch| ch.is_control()) {
        return Err("前端状态 key 包含非法控制字符".into());
    }
    Ok(())
}

fn validate_value(value: &str) -> Result<(), String> {
    if value.len() > MAX_VALUE_BYTES {
        return Err("前端状态值过大".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unique_temp_file(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "{prefix}_{}_{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn frontend_state_file_roundtrips_a4n_entries() {
        let path = unique_temp_file("a4n_frontend_state");
        let mut file = FrontendStateFile::default();
        file.entries.insert("a4n:settings:tab".into(), "llm".into());
        file.entries.insert("a4n-theme-mode".into(), "dark".into());

        write_state_file(&path, &file).expect("write state");
        let reloaded = read_state_file(&path).expect("read state");

        assert_eq!(
            reloaded.entries.get("a4n:settings:tab"),
            Some(&"llm".into())
        );
        assert_eq!(reloaded.entries.get("a4n-theme-mode"), Some(&"dark".into()));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn frontend_state_file_replaces_existing_state() {
        let path = unique_temp_file("a4n_frontend_state_replace");
        let mut first = FrontendStateFile::default();
        first
            .entries
            .insert("a4n:settings:tab".into(), "llm".into());
        write_state_file(&path, &first).expect("write first state");

        let mut second = FrontendStateFile::default();
        second
            .entries
            .insert("a4n:settings:tab".into(), "video".into());
        write_state_file(&path, &second).expect("write second state");

        let reloaded = read_state_file(&path).expect("read replaced state");
        assert_eq!(
            reloaded.entries.get("a4n:settings:tab"),
            Some(&"video".into())
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn frontend_state_file_rejects_invalid_persisted_keys_instead_of_dropping_them() {
        let path = unique_temp_file("a4n_frontend_state_invalid_key");
        std::fs::write(
            &path,
            r#"{
  "version": 1,
  "entries": {
    "a4n:settings:tab": "llm",
    "other:key": "foreign"
  }
}"#,
        )
        .expect("write invalid state");

        let err = read_state_file(&path).unwrap_err();
        assert!(err.contains("前端状态文件包含非法 key"));
        assert!(err.contains("other:key"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn frontend_state_file_rejects_future_versions_instead_of_downgrading_them() {
        let path = unique_temp_file("a4n_frontend_state_future_version");
        std::fs::write(
            &path,
            r#"{
  "version": 2,
  "entries": {
    "a4n:settings:tab": "llm"
  }
}"#,
        )
        .expect("write future state");

        let err = read_state_file(&path).unwrap_err();
        assert!(err.contains("不支持的前端状态版本：2"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn frontend_state_rejects_directory_at_state_file_path_without_moving_it() {
        let path = unique_temp_file("a4n_frontend_state_directory");
        std::fs::create_dir_all(&path).expect("create directory at state file path");

        let mut file = FrontendStateFile::default();
        file.entries.insert("a4n:settings:tab".into(), "llm".into());

        let read_err = read_state_file(&path).unwrap_err();
        assert!(read_err.contains("前端状态路径不是文件"));
        let write_err = write_state_file(&path, &file).unwrap_err();
        assert!(write_err.contains("前端状态路径不是文件"));
        assert!(path.is_dir(), "directory at state path must not be moved");

        let _ = std::fs::remove_dir_all(&path);
    }

    #[test]
    fn frontend_state_rejects_non_a4n_keys() {
        assert!(validate_key("a4n:ok").is_ok());
        assert!(validate_key("a4n-theme-mode").is_ok());
        assert!(validate_key("other:key").is_err());
        assert!(validate_key("").is_err());
    }
}
