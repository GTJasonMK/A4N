use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use url::Url;

const DEFAULT_SERVER: &str = "http://127.0.0.1:8080";

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredConfig {
    pub(crate) server: Option<String>,
    pub(crate) access_token: Option<String>,
    pub(crate) refresh_token: Option<String>,
    pub(crate) session_id: Option<String>,
}

impl StoredConfig {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let _lock = lock_config(path)?;
        Self::load_unlocked(path)
    }

    pub(crate) fn load_unlocked(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(path)
            .with_context(|| format!("读取 CLI 配置失败: {}", path.display()))?;
        serde_json::from_str(&contents).with_context(|| format!("CLI 配置无效: {}", path.display()))
    }

    #[cfg(test)]
    pub(crate) fn save(&self, path: &Path) -> Result<()> {
        let _lock = lock_config(path)?;
        self.save_unlocked(path)
    }

    pub(crate) fn save_unlocked(&self, path: &Path) -> Result<()> {
        write_private_json_unlocked(path, self)
    }

    pub(crate) fn clear_auth(&mut self) {
        self.access_token = None;
        self.refresh_token = None;
        self.session_id = None;
    }
}

pub(crate) fn write_private_json_unlocked<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("创建 CLI 配置目录失败: {}", parent.display()))?;

    let mut temp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("创建临时配置文件失败: {}", parent.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .with_context(|| format!("设置 CLI 配置权限失败: {}", path.display()))?;
    }
    serde_json::to_writer_pretty(temp.as_file_mut(), value).context("序列化 CLI 配置失败")?;
    temp.as_file_mut()
        .write_all(b"\n")
        .context("写入 CLI 配置失败")?;
    temp.as_file_mut().sync_all().context("同步 CLI 配置失败")?;
    temp.persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("原子替换 CLI 配置失败: {}", path.display()))?;
    Ok(())
}

pub(crate) fn lock_config(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("创建 CLI 配置目录失败: {}", parent.display()))?;
    }
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    let lock_path = PathBuf::from(lock_path);
    let mut options = OpenOptions::new();
    options.create(true).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(&lock_path)
        .with_context(|| format!("打开 CLI 配置锁失败: {}", lock_path.display()))?;
    file.lock()
        .with_context(|| format!("锁定 CLI 配置失败: {}", lock_path.display()))?;
    Ok(file)
}

pub(crate) fn resolve_config_path(override_path: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = override_path {
        return Ok(path);
    }
    if let Some(path) = non_empty_env_path("A4N_CLI_CONFIG")? {
        return Ok(path);
    }

    #[cfg(windows)]
    if let Some(root) = non_empty_env_path("APPDATA")? {
        return Ok(root.join("A4N").join("cli.json"));
    }

    if let Some(root) = non_empty_env_path("XDG_CONFIG_HOME")? {
        return Ok(root.join("a4n").join("cli.json"));
    }
    if let Some(root) = non_empty_env_path("HOME")? {
        return Ok(root.join(".config").join("a4n").join("cli.json"));
    }
    bail!("无法确定 CLI 配置目录；请使用 --config 或 A4N_CLI_CONFIG")
}

pub(crate) fn resolve_api_base(
    cli_server: Option<&str>,
    stored_server: Option<&str>,
) -> Result<String> {
    let env_server = non_empty_env_string("A4N_SERVER_URL")?;
    normalize_api_base(
        cli_server
            .or(env_server.as_deref())
            .or(stored_server)
            .unwrap_or(DEFAULT_SERVER),
    )
}

pub(crate) fn normalize_api_base(raw: &str) -> Result<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        bail!("Server URL 不能为空");
    }
    let mut url = Url::parse(raw).context("Server URL 无效")?;
    if !matches!(url.scheme(), "http" | "https") {
        bail!("Server URL 必须使用 http:// 或 https://");
    }
    if !url.username().is_empty() || url.password().is_some() {
        bail!("Server URL 不能包含用户名或密码");
    }
    if url.query().is_some() || url.fragment().is_some() {
        bail!("Server URL 不能包含查询参数或 fragment");
    }

    let path = url.path().trim_end_matches('/');
    let api_path = if path.is_empty() {
        "/api".to_string()
    } else if path.ends_with("/api") {
        path.to_string()
    } else {
        format!("{path}/api")
    };
    url.set_path(&api_path);
    Ok(url.as_str().trim_end_matches('/').to_string())
}

fn non_empty_env_path(key: &str) -> Result<Option<PathBuf>> {
    Ok(non_empty_env_string(key)?.map(PathBuf::from))
}

pub(crate) fn non_empty_env_string(key: &str) -> Result<Option<String>> {
    match std::env::var(key) {
        Ok(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                bail!("{key} 不能为空");
            }
            Ok(Some(value))
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => bail!("读取 {key} 失败: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_api_base;

    #[test]
    fn normalizes_server_root_and_existing_api_path() {
        assert_eq!(
            normalize_api_base("http://127.0.0.1:8080").unwrap(),
            "http://127.0.0.1:8080/api"
        );
        assert_eq!(
            normalize_api_base("https://example.com/a4n/api/").unwrap(),
            "https://example.com/a4n/api"
        );
    }

    #[test]
    fn rejects_credentials_and_non_http_urls() {
        assert!(normalize_api_base("file:///tmp/a4n").is_err());
        assert!(normalize_api_base("https://user:secret@example.com").is_err());
        let error = normalize_api_base("://embedded-secret").unwrap_err();
        assert!(!format!("{error:#}").contains("embedded-secret"));
    }
}
