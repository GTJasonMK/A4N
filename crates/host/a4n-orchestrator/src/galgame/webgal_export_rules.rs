//! WebGAL 导出纯规则（无 DB、无 Host 路径 IO）。

use std::path::Path;

use a4n_domain::galgame::{GalgameMediaRole, GalgameTransition};

/// 将强类型转场映射为 WebGAL 参数串。
pub fn webgal_transition_args(value: Option<GalgameTransition>) -> Result<&'static str, String> {
    match value {
        Some(GalgameTransition::Cut) | None => Ok(" -duration=0 -enterDuration=0 -exitDuration=0"),
        Some(GalgameTransition::Fade) => Ok(""),
        Some(GalgameTransition::Dissolve) => Err("WebGAL 4.6.5 不支持叠化转场".to_string()),
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
pub fn media_file_extension_for_webgal_role(
    file_path: &str,
    role: GalgameMediaRole,
) -> Result<String, String> {
    let extension = media_file_extension(file_path)?;
    match role {
        GalgameMediaRole::Bgm | GalgameMediaRole::Sfx
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
pub fn sanitize_export_file_stem(value: &str) -> Result<String, String> {
    let stem = value
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || matches!(ch, '-' | '_') {
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
        Err("Galgame 标题无法生成有效导出文件名".to_string())
    } else {
        Ok(stem)
    }
}

/// Native setVar expressions carry literal text past the script parser's command delimiters.
pub fn webgal_string_literal(value: &str) -> String {
    let json = serde_json::Value::String(value.to_owned()).to_string();
    let mut output = String::with_capacity(json.len());
    for ch in json.chars() {
        match ch {
            ';' | '=' | '{' | '}' | '+' | '-' | '*' | '/' | '(' | ')' => {
                output.push_str(&format!("\\u{:04x}", ch as u32));
            }
            _ => output.push(ch),
        }
    }
    output
}

pub fn webgal_choice_text(value: &str) -> Result<String, String> {
    if value.contains("->") {
        return Err("WebGAL 选择项不支持包含 -> 的文案".to_string());
    }
    Ok(value
        .trim()
        .replace(['\r', '\n', '\t'], " ")
        .replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(':', "\\:")
        .replace('|', "\\|"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_mapping() {
        assert!(webgal_transition_args(None).unwrap().contains("duration=0"));
        assert!(
            webgal_transition_args(Some(GalgameTransition::Cut))
                .unwrap()
                .contains("duration=0")
        );
        assert!(webgal_transition_args(Some(GalgameTransition::Dissolve)).is_err());
        assert!(webgal_choice_text("A -> B").is_err());
    }

    #[test]
    fn media_extension_and_sanitize() {
        assert_eq!(media_file_extension("a/b.PNG").unwrap(), "png");
        assert!(media_file_extension_for_webgal_role("a.mp4", GalgameMediaRole::Bgm).is_err());
        assert_eq!(
            media_file_extension_for_webgal_role("a.ogg", GalgameMediaRole::Sfx).unwrap(),
            "ogg"
        );
        assert_eq!(
            sanitize_export_file_stem("  My Script!  ").unwrap(),
            "My_Script"
        );
        assert!(sanitize_export_file_stem("   ").is_err());
        assert_eq!(sanitize_export_file_stem("雨夜").unwrap(), "雨夜");
    }

    #[test]
    fn text_literals_preserve_content_without_script_delimiters() {
        for text in [
            "A: B; C - D",
            "true+false={value}",
            "引号\"和反斜杠\\;\n下一行/(1)",
        ] {
            let literal = webgal_string_literal(text);
            assert_eq!(serde_json::from_str::<String>(&literal).unwrap(), text);
            assert!(!literal.contains([';', '=', '{', '}', '+', '-', '*', '/', '(', ')']));
        }
    }

    #[test]
    fn choice_literals_escape_webgal_delimiters_without_rewriting_text() {
        assert_eq!(
            webgal_choice_text("A: B; C | D\\E").unwrap(),
            "A\\: B\\; C \\| D\\\\E"
        );
        assert!(webgal_choice_text("A -> B").is_err());
    }
}
