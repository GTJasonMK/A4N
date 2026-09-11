use std::path::PathBuf;

/// 应用级路径
pub struct AppPaths {
    /// 数据根目录
    pub data_dir: PathBuf,
    /// 缓存目录
    pub cache_dir: PathBuf,
    /// 临时文件目录
    pub temp_dir: PathBuf,
    /// 日志目录
    pub log_dir: PathBuf,
    /// 配置文件路径
    pub config_file: PathBuf,
}

impl AppPaths {
    /// 从 AppConfig 构建
    pub fn new(data_dir: PathBuf, cache_dir: PathBuf, temp_dir: PathBuf, log_dir: PathBuf) -> Self {
        Self {
            config_file: data_dir.join("config.toml"),
            data_dir,
            cache_dir,
            temp_dir,
            log_dir,
        }
    }

    /// 导出目录
    pub fn exports_dir(&self) -> PathBuf {
        self.data_dir.join("exports")
    }

    /// 备份目录
    pub fn backups_dir(&self) -> PathBuf {
        self.data_dir.join("backups")
    }

    /// WebView 数据目录
    pub fn webview_dir(&self) -> PathBuf {
        self.data_dir.join("webview")
    }

    /// 前端便携状态文件
    pub fn frontend_state_file(&self) -> PathBuf {
        self.data_dir.join("frontend-state.json")
    }

    /// 确保所有应用级目录存在
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        for dir in &[
            &self.data_dir,
            &self.cache_dir,
            &self.temp_dir,
            &self.log_dir,
            &self.exports_dir(),
            &self.backups_dir(),
            &self.webview_dir(),
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}
