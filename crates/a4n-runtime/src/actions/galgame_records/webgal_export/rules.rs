use serde_json::Value;
use std::path::Path;

pub(super) fn webgal_figure_position(
    value: Option<&str>,
    line_id: i64,
) -> Result<Option<&'static str>, String> {
    match value {
        Some("left") => Ok(Some("left")),
        Some("center") => Ok(Some("center")),
        Some("right") => Ok(Some("right")),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => Err(format!(
            "Galgame 行 {line_id} position 无效，无法导出 WebGAL: {value}"
        )),
        None => Ok(None),
    }
}

pub(super) fn webgal_transition_args(value: &Value, line_id: i64) -> Result<&'static str, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("Galgame 行 {line_id} presentation_json 必须是 JSON 对象"))?;
    let Some(value) = object.get("transition") else {
        return Ok("");
    };
    let transition = value
        .as_str()
        .ok_or_else(|| format!("Galgame 行 {line_id} transition 必须是字符串"))?
        .trim();
    match transition {
        "" | "fade" | "dissolve" => Ok(""),
        "cut" => Ok(" -duration=0 -enterDuration=0 -exitDuration=0"),
        other => Err(format!("Galgame 行 {line_id} transition 不支持: {other}")),
    }
}

pub(super) fn media_file_extension(file_path: &str) -> Result<String, String> {
    let extension = Path::new(file_path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("媒体文件缺少扩展名: {file_path}"))?;
    if extension
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        Ok(extension)
    } else {
        Err(format!("媒体文件扩展名非法: {extension}"))
    }
}

pub(super) fn media_file_extension_for_webgal_role(
    file_path: &str,
    role: &str,
) -> Result<String, String> {
    let extension = media_file_extension(file_path)?;
    match role {
        a4n_orchestrator::media::MEDIA_BINDING_ROLE_BGM
        | a4n_orchestrator::media::MEDIA_BINDING_ROLE_SFX
            if !matches!(extension.as_str(), "mp3" | "wav" | "ogg") =>
        {
            Err(format!(
                "WebGAL 音频媒体扩展名不支持: role={}, extension={}",
                role, extension
            ))
        }
        _ => Ok(extension),
    }
}

pub(super) fn sanitize_export_file_stem(value: &str) -> String {
    let stem = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else if ch.is_whitespace() {
                '_'
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .trim_matches('_')
        .to_string();
    if stem.is_empty() {
        "galgame".to_string()
    } else {
        stem
    }
}

pub(super) fn webgal_speaker(value: &str) -> String {
    webgal_arg(value)
}

pub(super) fn webgal_text(value: &str) -> String {
    value
        .trim()
        .replace(['\r', '\n', '\t'], " ")
        .replace(';', "；")
}

pub(super) fn webgal_narration(value: &str) -> String {
    webgal_text(value).replace(':', "：")
}

pub(super) fn webgal_arg(value: &str) -> String {
    webgal_text(value).replace(':', " ").replace('|', "")
}
