use std::path::Path;

pub(in crate::actions::video_records) fn probe_video_properties(
    path: &Path,
    label: &str,
) -> Result<a4n_media::VideoProbeProperties, String> {
    a4n_media::probe_video_properties(path).map_err(|error| video_probe_error_message(label, error))
}

pub(in crate::actions::video_records) fn video_probe_error_message(
    label: &str,
    error: a4n_media::VideoProbeError,
) -> String {
    match error {
        a4n_media::VideoProbeError::Command(a4n_media::FfprobeCommandError::Unavailable {
            ..
        }) => format!("未检测到 FFprobe，无法读取{label}元数据"),
        a4n_media::VideoProbeError::Command(a4n_media::FfprobeCommandError::Failed { stderr }) => {
            format!("读取{label}元数据失败：{stderr}")
        }
        a4n_media::VideoProbeError::InvalidJson { source } => {
            format!("{label}元数据不是有效 JSON：{source}")
        }
        a4n_media::VideoProbeError::MissingVideoStream => format!("{label}缺少视频流"),
        a4n_media::VideoProbeError::InvalidWidth => format!("{label}缺少有效宽度"),
        a4n_media::VideoProbeError::InvalidHeight => format!("{label}缺少有效高度"),
        a4n_media::VideoProbeError::InvalidDuration => format!("{label}缺少有效时长"),
    }
}
