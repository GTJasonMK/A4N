use crate::AppState;
use a4n_media_contract::MediaRoot;
use std::path::Path;

pub(in crate::actions::video_records) async fn probe_video_properties(
    state: &AppState,
    path: &Path,
    root: MediaRoot,
    label: &str,
) -> Result<a4n_media_contract::VideoProbeProperties, String> {
    state
        .probe_media_video_properties(path, root, label)
        .await
        .map_err(|error| format!("读取{label}元数据失败：{error}"))
}

#[cfg(test)]
pub(in crate::actions::video_records) fn video_probe_error_message(
    label: &str,
    error: a4n_media_processing_core::VideoProbeError,
) -> String {
    match error {
        a4n_media_processing_core::VideoProbeError::Command(
            a4n_media_processing_core::FfprobeCommandError::Unavailable { .. },
        ) => format!("未检测到 FFprobe，无法读取{label}元数据"),
        a4n_media_processing_core::VideoProbeError::Command(
            a4n_media_processing_core::FfprobeCommandError::Failed { stderr },
        ) => {
            format!("读取{label}元数据失败：{stderr}")
        }
        a4n_media_processing_core::VideoProbeError::InvalidJson { source } => {
            format!("{label}元数据不是有效 JSON：{source}")
        }
        a4n_media_processing_core::VideoProbeError::MissingVideoStream => {
            format!("{label}缺少视频流")
        }
        a4n_media_processing_core::VideoProbeError::InvalidWidth => format!("{label}缺少有效宽度"),
        a4n_media_processing_core::VideoProbeError::InvalidHeight => format!("{label}缺少有效高度"),
        a4n_media_processing_core::VideoProbeError::InvalidDuration => {
            format!("{label}缺少有效时长")
        }
    }
}
