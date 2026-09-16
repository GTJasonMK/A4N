use serde_json::Value;
use std::path::Path;

use chrono::NaiveDateTime;

/// Facts inspected from a generated media file before publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedMediaArtifactFile {
    pub content_type: String,
    pub sha256: String,
    pub byte_size: u64,
}

/// Decoded image facts passed from media parsing to the image publication
/// owner. The parser owns validation; this pure value carries no image crate
/// or file-system implementation dependency.
#[derive(Debug)]
pub struct DecodedUploadedImage {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
    pub mime_type: &'static str,
    pub width: i32,
    pub height: i32,
    pub source_file_name: String,
}

/// Decoded audio facts passed from upload validation to the owning workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedUploadedAudio {
    pub file_name: String,
    pub mime_type: &'static str,
    pub extension: &'static str,
    pub bytes: Vec<u8>,
}

/// Pure input for creating a generated-image record. Database validation,
/// project ownership, locking and persistence stay in `a4n-db`.
#[derive(Debug, Clone)]
pub struct CreateGeneratedImageRecordDraftInput {
    pub project_id: i64,
    pub image_type: String,
    pub file_path: String,
    pub prompt: String,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub file_size: Option<i64>,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    pub metadata_json: Option<Value>,
    pub created_at: Option<NaiveDateTime>,
}

/// Durable facts needed to recover an image publication without exposing ORM
/// rows to the Runtime worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePublicationRecoveryRecord {
    pub publication_id: String,
    pub project_id: i64,
    pub requested_by_user_id: i64,
    pub staging_path: String,
    pub final_path: String,
    pub image_type: String,
    pub expected_file_size: i64,
    pub expected_sha256: String,
    pub file_size: Option<i64>,
    pub status: String,
    pub version: i64,
    pub image_id: Option<i64>,
}

pub const MEDIA_KIND_IMAGE: &str = "image";
pub const MEDIA_KIND_VIDEO: &str = "video";
pub const MEDIA_KIND_AUDIO: &str = "audio";

pub const MEDIA_ORIGIN_GENERATED: &str = "generated";
pub const MEDIA_ORIGIN_IMPORTED: &str = "imported";
pub const MEDIA_ORIGIN_EXTRACTED_FRAME: &str = "extracted_frame";
pub const MEDIA_ORIGIN_ASSEMBLED_PREVIEW: &str = "assembled_preview";

pub const MEDIA_STATUS_PENDING: &str = "pending";
pub const MEDIA_STATUS_RUNNING: &str = "running";
pub const MEDIA_STATUS_COMPLETED: &str = "completed";
pub const MEDIA_STATUS_FAILED: &str = "failed";
pub const MEDIA_STATUS_CANCELLED: &str = "cancelled";

pub const IMAGE_PUBLICATION_STATUS_PENDING: &str = "pending";
pub const IMAGE_PUBLICATION_STATUS_PUBLISHED: &str = "published";
pub const IMAGE_PUBLICATION_STATUS_RECLAIMING: &str = "reclaiming";
pub const IMAGE_PUBLICATION_STATUS_RECLAIMED: &str = "reclaimed";

const MEDIA_KINDS: &[&str] = &[MEDIA_KIND_IMAGE, MEDIA_KIND_VIDEO, MEDIA_KIND_AUDIO];
const MEDIA_ORIGINS: &[&str] = &[
    MEDIA_ORIGIN_GENERATED,
    MEDIA_ORIGIN_IMPORTED,
    MEDIA_ORIGIN_EXTRACTED_FRAME,
    MEDIA_ORIGIN_ASSEMBLED_PREVIEW,
];
const MEDIA_STATUSES: &[&str] = &[
    MEDIA_STATUS_PENDING,
    MEDIA_STATUS_RUNNING,
    MEDIA_STATUS_COMPLETED,
    MEDIA_STATUS_FAILED,
    MEDIA_STATUS_CANCELLED,
];

pub const MEDIA_BINDING_TARGET_KINDS: &[&str] = &[
    "character_profile",
    "object_profile",
    "scene_profile",
    "galgame_scene",
    "galgame_line",
    "video_segment",
    "chapter_video_prompt",
    "generated_video",
    "chapter",
    "assembly_timeline",
];
pub const MEDIA_BINDING_ROLES: &[&str] = &[
    "portrait",
    "object_material",
    "scene_material",
    "background",
    "cg",
    "bgm",
    "sfx",
    "first_frame",
    "end_frame",
    "reference",
    "generated_result",
    "inserted_bridge",
    "video_tail_frame",
];

pub const MEDIA_BINDING_TARGET_GALGAME_SCENE: &str = "galgame_scene";
pub const MEDIA_BINDING_TARGET_GALGAME_LINE: &str = "galgame_line";
pub const MEDIA_BINDING_ROLE_BACKGROUND: &str = "background";
pub const MEDIA_BINDING_ROLE_CG: &str = "cg";
pub const MEDIA_BINDING_ROLE_BGM: &str = "bgm";
pub const MEDIA_BINDING_ROLE_SFX: &str = "sfx";

pub fn is_valid_media_kind(value: &str) -> bool {
    MEDIA_KINDS.contains(&value)
}

pub fn is_valid_media_origin(value: &str) -> bool {
    MEDIA_ORIGINS.contains(&value)
}

pub fn is_valid_media_status(value: &str) -> bool {
    MEDIA_STATUSES.contains(&value)
}

pub fn is_valid_media_binding_target_kind(value: &str) -> bool {
    MEDIA_BINDING_TARGET_KINDS.contains(&value)
}

pub fn is_valid_media_binding_role(value: &str) -> bool {
    MEDIA_BINDING_ROLES.contains(&value)
}

pub fn galgame_media_binding_required_media_kind(
    target_kind: &str,
    role: &str,
) -> Option<&'static str> {
    match (target_kind, role) {
        (MEDIA_BINDING_TARGET_GALGAME_SCENE, MEDIA_BINDING_ROLE_BACKGROUND) => {
            Some(MEDIA_KIND_IMAGE)
        }
        (MEDIA_BINDING_TARGET_GALGAME_LINE, MEDIA_BINDING_ROLE_CG) => Some(MEDIA_KIND_IMAGE),
        (MEDIA_BINDING_TARGET_GALGAME_LINE, MEDIA_BINDING_ROLE_BGM | MEDIA_BINDING_ROLE_SFX) => {
            Some(MEDIA_KIND_AUDIO)
        }
        _ => None,
    }
}

pub fn is_valid_galgame_media_binding_pair(target_kind: &str, role: &str) -> bool {
    match target_kind {
        MEDIA_BINDING_TARGET_GALGAME_SCENE | MEDIA_BINDING_TARGET_GALGAME_LINE => {
            galgame_media_binding_required_media_kind(target_kind, role).is_some()
        }
        _ => true,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaProvenanceRecordInput {
    pub generation_spec_id: Option<i64>,
    pub prompt_artifact_id: Option<i64>,
    pub provider_job_id: Option<String>,
    pub source_media_asset_id: Option<i64>,
    pub source_external_path: Option<String>,
    pub imported_from_user: bool,
    pub request_snapshot_json: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateMediaAssetRecordInput {
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub media_kind: String,
    pub origin: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_secs: Option<f32>,
    pub fps: Option<f32>,
    pub file_size: Option<i64>,
    pub checksum: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub provenance: MediaProvenanceRecordInput,
}

pub fn media_mime_from_path(path: &Path) -> Result<&'static str, String> {
    let extension = media_file_extension(path, "媒体文件")?;
    media_mime_from_extension(&extension)
        .ok_or_else(|| format!("媒体文件扩展名不支持：{extension}"))
}

pub fn image_mime_from_path(path: &Path) -> Result<&'static str, String> {
    let extension = media_file_extension(path, "图片")?;
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
        return Err(format!("图片扩展名不支持：{extension}"));
    }
    media_mime_from_extension(&extension).ok_or_else(|| format!("图片扩展名不支持：{extension}"))
}

pub fn video_mime_from_path(path: &Path) -> Result<&'static str, String> {
    let extension = media_file_extension(path, "视频文件")?;
    if !matches!(extension.as_str(), "mp4" | "m4v" | "webm" | "mov") {
        return Err(format!("视频文件扩展名不支持：{extension}"));
    }
    media_mime_from_extension(&extension)
        .ok_or_else(|| format!("视频文件扩展名不支持：{extension}"))
}

pub fn canonical_audio_mime(value: &str) -> Result<(&'static str, &'static str), String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "audio/mpeg" | "audio/mp3" => Ok(("audio/mpeg", "mp3")),
        "audio/wav" | "audio/x-wav" => Ok(("audio/wav", "wav")),
        "audio/ogg" | "application/ogg" => Ok(("audio/ogg", "ogg")),
        "audio/mp4" | "audio/x-m4a" => Ok(("audio/mp4", "m4a")),
        "audio/aac" => Ok(("audio/aac", "aac")),
        "audio/flac" | "audio/x-flac" => Ok(("audio/flac", "flac")),
        other => Err(format!("音频 MIME 不支持：{other}")),
    }
}

fn media_file_extension(path: &Path, label: &str) -> Result<String, String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| {
            format!(
                "{label}缺少文件扩展名，无法确定 MIME 类型：{}",
                path.display()
            )
        })
}

fn media_mime_from_extension(extension: &str) -> Option<&'static str> {
    match extension {
        "apng" => Some("image/apng"),
        "avif" => Some("image/avif"),
        "gif" => Some("image/gif"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "svg" => Some("image/svg+xml"),
        "webp" => Some("image/webp"),
        "mp4" | "m4v" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "mov" => Some("video/quicktime"),
        "mp3" => Some("audio/mpeg"),
        "m4a" => Some("audio/mp4"),
        "wav" => Some("audio/wav"),
        "ogg" => Some("audio/ogg"),
        "aac" => Some("audio/aac"),
        "flac" => Some("audio/flac"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_contract_accepts_only_registered_values() {
        for value in MEDIA_KINDS {
            assert!(is_valid_media_kind(value));
        }
        for value in MEDIA_ORIGINS {
            assert!(is_valid_media_origin(value));
        }
        for value in MEDIA_STATUSES {
            assert!(is_valid_media_status(value));
        }
        for value in MEDIA_BINDING_TARGET_KINDS {
            assert!(is_valid_media_binding_target_kind(value));
        }
        for value in MEDIA_BINDING_ROLES {
            assert!(is_valid_media_binding_role(value));
        }

        assert!(!is_valid_media_kind("document"));
        assert!(!is_valid_media_origin("uploaded"));
        assert!(!is_valid_media_status("done"));
        assert!(!is_valid_media_binding_target_kind("unknown_target"));
        assert!(!is_valid_media_binding_role("unknown_role"));
    }

    #[test]
    fn galgame_binding_pairs_require_the_expected_media_kind() {
        assert_eq!(
            galgame_media_binding_required_media_kind(
                MEDIA_BINDING_TARGET_GALGAME_SCENE,
                MEDIA_BINDING_ROLE_BACKGROUND,
            ),
            Some(MEDIA_KIND_IMAGE),
        );
        assert_eq!(
            galgame_media_binding_required_media_kind(
                MEDIA_BINDING_TARGET_GALGAME_LINE,
                MEDIA_BINDING_ROLE_CG,
            ),
            Some(MEDIA_KIND_IMAGE),
        );
        for role in [MEDIA_BINDING_ROLE_BGM, MEDIA_BINDING_ROLE_SFX] {
            assert_eq!(
                galgame_media_binding_required_media_kind(MEDIA_BINDING_TARGET_GALGAME_LINE, role),
                Some(MEDIA_KIND_AUDIO),
            );
        }
        assert!(!is_valid_galgame_media_binding_pair(
            MEDIA_BINDING_TARGET_GALGAME_SCENE,
            MEDIA_BINDING_ROLE_BGM,
        ));
        assert!(is_valid_galgame_media_binding_pair(
            "chapter_video_prompt",
            "reference",
        ));
    }

    #[test]
    fn strict_image_and_video_mime_require_supported_extensions() {
        assert_eq!(
            image_mime_from_path(Path::new("image.PNG")),
            Ok("image/png")
        );
        assert!(image_mime_from_path(Path::new("image.gif")).is_err());
        assert_eq!(video_mime_from_path(Path::new("clip.M4V")), Ok("video/mp4"));
        assert!(video_mime_from_path(Path::new("clip.avi")).is_err());
    }

    #[test]
    fn generic_media_mime_rejects_unknown_extensions() {
        assert_eq!(
            media_mime_from_path(Path::new("image.AVIF")),
            Ok("image/avif")
        );
        assert_eq!(
            media_mime_from_path(Path::new("voice.flac")),
            Ok("audio/flac")
        );
        assert!(media_mime_from_path(Path::new("archive.bin")).is_err());
    }

    #[test]
    fn audio_mime_is_canonical_and_extension_bound() {
        assert_eq!(
            canonical_audio_mime("audio/x-wav"),
            Ok(("audio/wav", "wav"))
        );
        assert_eq!(
            canonical_audio_mime("audio/x-flac"),
            Ok(("audio/flac", "flac"))
        );
        assert!(canonical_audio_mime("application/octet-stream").is_err());
    }
}
