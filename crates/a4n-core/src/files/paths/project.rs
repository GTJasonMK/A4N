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
}

fn chapter_asset_dir_name(chapter_id: i64) -> String {
    format!("chapter_{chapter_id}")
}
