//! 画布本地音频路径的扩展名 / MIME 纯规则（无 DB、无 Host）。

use std::path::Path;

use a4n_core::media::{canonical_audio_mime, media_mime_from_path};

/// 由路径解析支持的画布音频规范 MIME 与扩展名。
pub(super) fn canvas_audio_format(
    path: &Path,
) -> Result<(&'static str, &'static str), String> {
    let mime_type = media_mime_from_path(path)
        .map_err(|_| format!("导入音频格式不支持：{}", path.display()))?;
    canonical_audio_mime(mime_type)
        .map(|(_, extension)| (mime_type, extension))
        .map_err(|_| format!("导入音频格式不支持：{}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_maps_supported() {
        assert_eq!(
            canvas_audio_format(Path::new("x.WAV")).unwrap(),
            ("audio/wav", "wav")
        );
        assert!(canvas_audio_format(Path::new("x.txt")).is_err());
    }
}
