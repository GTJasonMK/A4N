use a4n_orchestrator::galgame::{GALGAME_ENTITY_LINE, GALGAME_ENTITY_SCENE};
use a4n_orchestrator::media::{
    MEDIA_BINDING_ROLE_BGM, MEDIA_BINDING_ROLE_SFX, galgame_media_binding_required_media_kind,
};
use base64::Engine as _;
use std::path::Path;

const MAX_IMPORT_AUDIO_BYTES: usize = 256 * 1024 * 1024;

pub(super) struct GalgameRoleRule {
    pub(super) required_media_kind: &'static str,
}

pub(super) struct ImportedAudioSource {
    pub(super) file_name: String,
    pub(super) mime_type: &'static str,
    pub(super) extension: &'static str,
    pub(super) bytes: Vec<u8>,
}

impl ImportedAudioSource {
    pub(super) fn from_upload(
        file_name: &str,
        mime_type: &str,
        data_base64: &str,
    ) -> Result<Self, String> {
        let file_name = required_text(file_name, "音频文件名")?;
        let (mime_type, extension) = supported_audio_mime(mime_type)?;
        let raw = strip_data_url_prefix(data_base64).trim();
        if raw.is_empty() {
            return Err("音频文件数据不能为空".to_string());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw)
            .map_err(|err| format!("音频文件 base64 解码失败：{err}"))?;
        if bytes.is_empty() {
            return Err("音频文件不能为空".to_string());
        }
        if bytes.len() > MAX_IMPORT_AUDIO_BYTES {
            return Err(format!(
                "音频文件超过 {}MB",
                MAX_IMPORT_AUDIO_BYTES / 1024 / 1024
            ));
        }
        if !file_name_matches_audio_extension(&file_name, extension) {
            return Err(format!(
                "音频文件扩展名与 MIME 不一致，需要 .{} 文件",
                extension
            ));
        }
        Ok(Self {
            file_name,
            mime_type,
            extension,
            bytes,
        })
    }
}

pub(super) fn normalize_galgame_media_target_kind(value: &str) -> Result<String, String> {
    match value.trim() {
        GALGAME_ENTITY_SCENE => Ok(GALGAME_ENTITY_SCENE.to_string()),
        GALGAME_ENTITY_LINE => Ok(GALGAME_ENTITY_LINE.to_string()),
        other => Err(format!("Galgame 媒体目标类型无效: {other}")),
    }
}

pub(super) fn normalize_galgame_media_role(
    target_kind: &str,
    role: &str,
) -> Result<String, String> {
    let role = role.trim();
    validate_galgame_media_role(target_kind, role)?;
    Ok(role.to_string())
}

pub(super) fn validate_galgame_media_role(
    target_kind: &str,
    role: &str,
) -> Result<GalgameRoleRule, String> {
    galgame_media_binding_required_media_kind(target_kind.trim(), role.trim())
        .map(|required_media_kind| GalgameRoleRule {
            required_media_kind,
        })
        .ok_or_else(|| format!("Galgame 媒体绑定目标/角色无效: {} / {}", target_kind, role))
}

pub(super) fn normalize_audio_role(role: &str) -> Result<String, String> {
    match role.trim() {
        MEDIA_BINDING_ROLE_BGM => Ok(MEDIA_BINDING_ROLE_BGM.to_string()),
        MEDIA_BINDING_ROLE_SFX => Ok(MEDIA_BINDING_ROLE_SFX.to_string()),
        other => Err(format!("Galgame 音频角色无效: {other}")),
    }
}

fn supported_audio_mime(mime_type: &str) -> Result<(&'static str, &'static str), String> {
    match mime_type.trim().to_ascii_lowercase().as_str() {
        "audio/mpeg" | "audio/mp3" => Ok(("audio/mpeg", "mp3")),
        "audio/wav" | "audio/x-wav" => Ok(("audio/wav", "wav")),
        "audio/ogg" | "application/ogg" => Ok(("audio/ogg", "ogg")),
        other => Err(format!("Galgame 音频格式不支持: {other}")),
    }
}

fn strip_data_url_prefix(value: &str) -> &str {
    value
        .split_once(";base64,")
        .map(|(_, body)| body)
        .unwrap_or(value)
}

fn file_name_matches_audio_extension(file_name: &str, extension: &str) -> bool {
    Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

fn required_text(value: &str, field_name: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field_name}不能为空"));
    }
    Ok(value.to_string())
}
