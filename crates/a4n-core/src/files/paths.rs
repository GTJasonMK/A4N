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
//! ```

mod app;
mod project;

pub use app::AppPaths;
pub use project::ProjectPaths;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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
