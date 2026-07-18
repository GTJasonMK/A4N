use std::path::Path;

pub(super) fn required_canvas_audio_file_name(file_name: &str) -> Result<String, String> {
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

pub(super) fn canvas_audio_extension(path: &Path) -> Result<&'static str, String> {
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

pub(super) fn validate_canvas_audio_mime(mime_type: &str, extension: &str) -> Result<(), String> {
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
