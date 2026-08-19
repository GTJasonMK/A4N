use std::path::PathBuf;

pub(super) const MAX_UPLOAD_VIDEO_BYTES: usize = 512 * 1024 * 1024;

pub use a4n_api_contract::video::{
    UploadAssemblyInsertVideoInput, UploadChapterVideoInput, UploadReferenceMediaVideoInput,
};

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
