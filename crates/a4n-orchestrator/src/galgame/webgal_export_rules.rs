//! WebGAL 导出纯规则（无 DB、无 Host 路径 IO）。

use serde_json::Value;
use std::path::Path;

use crate::media::{MEDIA_BINDING_ROLE_BGM, MEDIA_BINDING_ROLE_SFX};

/// 将行 position 映射为 WebGAL figure 位置。
pub fn webgal_figure_position(
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

/// 从 presentation_json 解析 WebGAL transition 参数串。
pub fn webgal_transition_args(value: &Value, line_id: i64) -> Result<&'static str, String> {
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

/// 解析媒体文件扩展名（小写、仅 alnum/-/_）。
pub fn media_file_extension(file_path: &str) -> Result<String, String> {
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

/// WebGAL 角色约束下的媒体扩展名（音频仅 mp3/wav/ogg）。
pub fn media_file_extension_for_webgal_role(file_path: &str, role: &str) -> Result<String, String> {
    let extension = media_file_extension(file_path)?;
    match role {
        MEDIA_BINDING_ROLE_BGM | MEDIA_BINDING_ROLE_SFX
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

/// 导出包文件名 stem 净化。
pub fn sanitize_export_file_stem(value: &str) -> String {
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

/// WebGAL 说话人参数转义。
pub fn webgal_speaker(value: &str) -> String {
    webgal_arg(value)
}

/// WebGAL 对白文本转义。
pub fn webgal_text(value: &str) -> String {
    value
        .trim()
        .replace(['\r', '\n', '\t'], " ")
        .replace(';', "；")
}

/// WebGAL 旁白文本转义。
pub fn webgal_narration(value: &str) -> String {
    webgal_text(value).replace(':', "：")
}

/// WebGAL 通用参数转义。
pub fn webgal_arg(value: &str) -> String {
    webgal_text(value).replace(':', " ").replace('|', "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn figure_position_and_transition() {
        assert_eq!(
            webgal_figure_position(Some("left"), 1).unwrap(),
            Some("left")
        );
        assert!(webgal_figure_position(Some("top"), 1).is_err());
        assert_eq!(webgal_transition_args(&json!({}), 1).unwrap(), "");
        assert!(
            webgal_transition_args(&json!({"transition": "cut"}), 1)
                .unwrap()
                .contains("duration=0")
        );
    }

    #[test]
    fn media_extension_and_sanitize() {
        assert_eq!(media_file_extension("a/b.PNG").unwrap(), "png");
        assert!(media_file_extension_for_webgal_role("a.mp4", MEDIA_BINDING_ROLE_BGM).is_err());
        assert_eq!(
            media_file_extension_for_webgal_role("a.ogg", MEDIA_BINDING_ROLE_SFX).unwrap(),
            "ogg"
        );
        assert_eq!(sanitize_export_file_stem("  My Script!  "), "My_Script");
        assert_eq!(sanitize_export_file_stem("   "), "galgame");
        // 非 ASCII 字母数字会被替换为 '-'，最终可能回落到默认 stem
        assert_eq!(sanitize_export_file_stem("雨夜"), "galgame");
    }

    #[test]
    fn text_escaping() {
        assert_eq!(webgal_text("a;b\nc"), "a；b c");
        assert_eq!(webgal_narration("a:b"), "a：b");
        assert_eq!(webgal_arg("a:b|c"), "a bc");
    }
}
