//! 画布音频导入：文件名 / 扩展名 / MIME 纯规则（无 DB、无 Host）。

use std::path::Path;

/// 校验上传音频文件名（非空、无路径分量）。
pub fn required_canvas_audio_file_name(file_name: &str) -> Result<String, String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err("上传音频文件名不能为空".to_string());
    }
    let name = Path::new(trimmed)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "上传音频文件名无效".to_string())?;
    if name != trimmed {
        return Err("上传音频文件名不能包含路径".to_string());
    }
    Ok(name.to_string())
}

/// 由路径解析支持的画布音频扩展名。
pub fn canvas_audio_extension(path: &Path) -> Result<&'static str, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("导入音频缺少文件扩展名：{}", path.display()))?;
    match extension.to_ascii_lowercase().as_str() {
        "mp3" => Ok("mp3"),
        "wav" => Ok("wav"),
        "ogg" => Ok("ogg"),
        "m4a" => Ok("m4a"),
        "aac" => Ok("aac"),
        "flac" => Ok("flac"),
        _ => Err(format!("导入音频格式不支持：{extension}")),
    }
}

/// 校验 MIME 与扩展名是否匹配（允许通用 audio/* 与 octet-stream）。
pub fn validate_canvas_audio_mime(mime_type: &str, extension: &str) -> Result<(), String> {
    let mime = mime_type.trim().to_ascii_lowercase();
    if mime.is_empty() || mime == "application/octet-stream" || mime.starts_with("audio/") {
        return Ok(());
    }
    let valid = match extension {
        "m4a" => mime == "video/mp4" || mime == "audio/mp4",
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err("上传音频 MIME 与文件扩展名不匹配".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn file_name_rejects_path_components() {
        assert!(required_canvas_audio_file_name("a.mp3").is_ok());
        assert!(required_canvas_audio_file_name("dir/a.mp3").is_err());
        assert!(required_canvas_audio_file_name("  ").is_err());
    }

    #[test]
    fn extension_maps_supported() {
        assert_eq!(canvas_audio_extension(Path::new("x.WAV")).unwrap(), "wav");
        assert!(canvas_audio_extension(Path::new("x.txt")).is_err());
    }

    #[test]
    fn mime_allows_audio_star_and_m4a_video_mp4() {
        assert!(validate_canvas_audio_mime("audio/mpeg", "mp3").is_ok());
        assert!(validate_canvas_audio_mime("video/mp4", "m4a").is_ok());
        assert!(validate_canvas_audio_mime("image/png", "mp3").is_err());
    }
}
