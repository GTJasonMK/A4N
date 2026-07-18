use super::prelude::*;

pub(super) const MAX_UPLOAD_VIDEO_BYTES: usize = 512 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadChapterVideoInput {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub prompt_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadReferenceMediaVideoInput {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub prompt: String,
    pub duration_secs: i32,
    pub aspect_ratio: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadAssemblyInsertVideoInput {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    #[serde(default)]
    pub insert_after_video_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportCanvasVideoInput {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub data_base64: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
}

pub(in crate::actions::video_records) struct ImportedVideoSource {
    pub(in crate::actions::video_records) path: PathBuf,
    pub(super) file_size: i64,
    pub(in crate::actions::video_records) file_name: String,
    pub(super) extension: &'static str,
    pub(super) temporary_path: Option<PathBuf>,
}

impl Drop for ImportedVideoSource {
    fn drop(&mut self) {
        if let Some(path) = self.temporary_path.as_deref() {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => tracing::warn!(
                    path = %path.display(),
                    %error,
                    "清理上传视频临时文件失败"
                ),
            }
        }
    }
}
