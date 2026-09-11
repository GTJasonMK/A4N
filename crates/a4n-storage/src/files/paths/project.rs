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

    /// 项目音频目录
    pub fn audio_dir(&self) -> PathBuf {
        self.root.join("audio")
    }
}

fn chapter_asset_dir_name(chapter_id: i64) -> String {
    format!("chapter_{chapter_id}")
}
