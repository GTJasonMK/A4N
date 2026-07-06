//! 项目目录结构与路径解析
//!
//! 定义 A4N 项目文件系统的标准目录结构：
//!
//! ```text
//! {data_dir}/
//! └── projects/
//!     └── {project_id}/
//!         ├── chapters/           # 章节内容
//!         │   └── {number}.md
//!         ├── blueprints/         # 蓝图版本
//!         │   └── {version}.json
//!         ├── outlines/           # 大纲
//!         │   ├── parts/
//!         │   └── chapters/
//!         ├── images/             # 项目级图片与章节图片
//!         │   └── chapter_{id}/
//!         ├── videos/             # 章节视频
//!         │   └── chapter_{id}/
//!         ├── portraits/          # 角色立绘
//!         ├── exports/            # 导出文件
//!         ├── snapshots/          # 角色快照
//!         └── project.toml        # 项目元数据
//! ```

use crate::config::app_config::AppConfig;
use std::path::{Path, PathBuf};

/// 项目路径管理器
///
/// 提供项目内各子目录和文件的标准路径。
#[derive(Debug, Clone)]
pub struct ProjectPaths {
    /// 项目根目录
    root: PathBuf,
}

impl ProjectPaths {
    /// 创建项目路径管理器
    pub fn new(data_dir: &Path, project_id: i64) -> Self {
        Self {
            root: data_dir.join("projects").join(project_id.to_string()),
        }
    }

    /// 项目根目录
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 章节目录
    pub fn chapters_dir(&self) -> PathBuf {
        self.root.join("chapters")
    }

    /// 指定章节文件路径
    pub fn chapter_file(&self, number: u32) -> PathBuf {
        self.chapters_dir().join(format!("{number}.md"))
    }

    /// 蓝图目录
    pub fn blueprints_dir(&self) -> PathBuf {
        self.root.join("blueprints")
    }

    /// 指定版本蓝图文件
    pub fn blueprint_file(&self, version: &str) -> PathBuf {
        self.blueprints_dir().join(format!("{version}.json"))
    }

    /// 分部大纲目录
    pub fn part_outlines_dir(&self) -> PathBuf {
        self.root.join("outlines").join("parts")
    }

    /// 章节大纲目录
    pub fn chapter_outlines_dir(&self) -> PathBuf {
        self.root.join("outlines").join("chapters")
    }

    /// 图片目录
    pub fn images_dir(&self) -> PathBuf {
        self.root.join("images")
    }

    /// 指定章节图片目录
    pub fn chapter_images_dir(&self, chapter_id: i64) -> PathBuf {
        self.images_dir().join(chapter_asset_dir_name(chapter_id))
    }

    /// 视频目录
    pub fn videos_dir(&self) -> PathBuf {
        self.root.join("videos")
    }

    /// 指定章节视频目录
    pub fn chapter_videos_dir(&self, chapter_id: i64) -> PathBuf {
        self.videos_dir().join(chapter_asset_dir_name(chapter_id))
    }

    /// 角色立绘目录
    pub fn portraits_dir(&self) -> PathBuf {
        self.root.join("portraits")
    }

    /// 导出目录
    pub fn exports_dir(&self) -> PathBuf {
        self.root.join("exports")
    }

    /// 角色快照目录
    pub fn snapshots_dir(&self) -> PathBuf {
        self.root.join("snapshots")
    }

    /// 项目元数据文件
    pub fn metadata_file(&self) -> PathBuf {
        self.root.join("project.toml")
    }

    /// 确保所有子目录存在
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        let dirs = [
            self.root(),
            &self.chapters_dir(),
            &self.blueprints_dir(),
            &self.part_outlines_dir(),
            &self.chapter_outlines_dir(),
            &self.images_dir(),
            &self.videos_dir(),
            &self.portraits_dir(),
            &self.exports_dir(),
            &self.snapshots_dir(),
        ];
        for dir in &dirs {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    /// 计算目录总大小（递归）
    pub fn total_size(&self) -> std::io::Result<u64> {
        dir_size(&self.root)
    }
}

fn chapter_asset_dir_name(chapter_id: i64) -> String {
    format!("chapter_{chapter_id}")
}

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
    /// 提示词模板目录
    pub prompts_dir: PathBuf,
    /// 配置文件路径
    pub config_file: PathBuf,
}

impl AppPaths {
    /// 从 AppConfig 构建
    pub fn new(data_dir: PathBuf, cache_dir: PathBuf, temp_dir: PathBuf, log_dir: PathBuf) -> Self {
        Self {
            prompts_dir: data_dir.join("prompts"),
            config_file: data_dir.join("config.toml"),
            data_dir,
            cache_dir,
            temp_dir,
            log_dir,
        }
    }

    /// 从应用配置构建
    pub fn from_config(config: &AppConfig) -> Self {
        Self::new(
            config.data_dir.clone(),
            config.cache_dir.clone(),
            config.temp_dir.clone(),
            config.log_dir.clone(),
        )
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
            &self.prompts_dir,
            &self.exports_dir(),
            &self.backups_dir(),
            &self.webview_dir(),
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

/// 递归计算目录大小
fn dir_size(path: &Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if meta.is_dir() {
                total += dir_size(&entry.path())?;
            } else {
                total += meta.len();
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_paths() {
        let paths = ProjectPaths::new(Path::new("/tmp/a4n"), 42);
        assert_eq!(paths.root(), Path::new("/tmp/a4n/projects/42"));
        assert_eq!(
            paths.chapter_file(5),
            Path::new("/tmp/a4n/projects/42/chapters/5.md")
        );
        assert_eq!(
            paths.blueprint_file("v1.0"),
            Path::new("/tmp/a4n/projects/42/blueprints/v1.0.json")
        );
        assert_eq!(
            paths.chapter_images_dir(7),
            Path::new("/tmp/a4n/projects/42/images/chapter_7")
        );
        assert_eq!(
            paths.chapter_videos_dir(7),
            Path::new("/tmp/a4n/projects/42/videos/chapter_7")
        );
        assert_eq!(
            paths.portraits_dir(),
            Path::new("/tmp/a4n/projects/42/portraits")
        );
    }

    #[test]
    fn test_ensure_dirs_creates_structure() {
        let tmp = std::env::temp_dir().join("a4n_test_paths");
        let _ = std::fs::remove_dir_all(&tmp);
        let paths = ProjectPaths::new(&tmp, 1);
        paths.ensure_dirs().unwrap();
        assert!(paths.chapters_dir().exists());
        assert!(paths.blueprints_dir().exists());
        assert!(paths.images_dir().exists());
        assert!(paths.videos_dir().exists());
        assert!(paths.portraits_dir().exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn app_paths_create_all_portable_dirs() {
        let tmp = std::env::temp_dir().join(format!(
            "a4n_app_paths_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let data_dir = tmp.join("data");
        let paths = AppPaths::new(
            data_dir.clone(),
            data_dir.join("cache"),
            data_dir.join("tmp"),
            data_dir.join("logs"),
        );

        paths.ensure_dirs().unwrap();

        for dir in [
            &paths.data_dir,
            &paths.cache_dir,
            &paths.temp_dir,
            &paths.log_dir,
            &paths.prompts_dir,
            &paths.exports_dir(),
            &paths.backups_dir(),
            &paths.webview_dir(),
        ] {
            assert!(dir.is_dir(), "expected directory to exist: {dir:?}");
            assert!(
                dir.starts_with(&data_dir),
                "directory escaped data dir: {dir:?}"
            );
        }
        assert_eq!(
            paths.frontend_state_file(),
            data_dir.join("frontend-state.json")
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
