mod files;

pub use files::*;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use chrono::NaiveDateTime;

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

/// Pure input for creating a generated-video record. Database validation,
/// source ownership checks, locking and persistence stay in `a4n-db`.
#[derive(Debug, Clone)]
pub struct CreateGeneratedVideoRecordDraftInput {
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: Option<String>,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
    pub source_video_id: Option<i64>,
    pub source_image_id: Option<i64>,
    pub provider_job_id: Option<String>,
    pub metadata_json: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedImageRecord {
    pub id: i64,
    pub project_id: i64,
    pub image_type: String,
    pub file_path: String,
    pub prompt: String,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub file_size: Option<i64>,
    pub ai_score: Option<f32>,
    pub is_favorite: bool,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    pub metadata_json: Option<Value>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaProvenanceRecord {
    pub media_asset_id: i64,
    pub generation_spec_id: Option<i64>,
    pub prompt_artifact_id: Option<i64>,
    pub provider_job_id: Option<String>,
    pub source_media_asset_id: Option<i64>,
    pub source_external_path: Option<String>,
    pub imported_from_user: bool,
    pub request_snapshot_json: Option<Value>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedImageReferenceRecord {
    pub id: i64,
    pub image_type: String,
    pub metadata_json: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedVideoRecord {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: Option<String>,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
    pub file_size: Option<i64>,
    pub source_video_id: Option<i64>,
    pub source_image_id: Option<i64>,
    pub status: String,
    pub provider_job_id: Option<String>,
    pub error_message: Option<String>,
    pub timeline_item_id: Option<i64>,
    pub metadata_json: Option<Value>,
    pub is_favorite: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Pure patch input for a generated-image record. Database ownership checks,
/// normalization and persistence stay in `a4n-db`.
#[derive(Debug, Clone, Default)]
pub struct UpdateGeneratedImageRecordInput {
    pub prompt: Option<String>,
    pub image_type: Option<String>,
    pub chapter_id: Option<i64>,
    pub clear_chapter_id: bool,
    pub panel_id: Option<String>,
    pub clear_panel_id: bool,
    pub ai_score: Option<f32>,
    pub clear_ai_score: bool,
    pub is_favorite: Option<bool>,
}

/// Pure patch input for a generated-video record. Database ownership checks,
/// normalization and persistence stay in `a4n-db`.
#[derive(Debug, Clone, Default)]
pub struct UpdateGeneratedVideoRecordInput {
    pub prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub clear_negative_prompt: bool,
    pub metadata_json: Option<Value>,
    pub clear_metadata_json: bool,
    pub is_favorite: Option<bool>,
}

/// Input for recording a pending image publication. The database owns the
/// transaction and state machine; this value only carries the request facts.
#[derive(Debug, Clone)]
pub struct CreateImagePublicationIntentInput {
    pub publication_id: String,
    pub project_id: i64,
    pub requested_by_user_id: i64,
    pub staging_path: String,
    pub final_path: String,
    pub image: CreateGeneratedImageRecordDraftInput,
    pub expected_file_size: i64,
    pub expected_sha256: String,
    /// 生成重试可以替换仍处于 pending 的完整输出意图；导入和普通发布意图必须保持严格幂等。
    pub allow_pending_replacement: bool,
}

/// File facts recorded by the media adapter during image publication.
#[derive(Debug, Clone)]
pub struct ImagePublicationFileFacts {
    pub file_size: i64,
    pub sha256: String,
}

/// One video transition in a batch terminal-status update. The database owns
/// status validation, prompt linkage and the transaction.
#[derive(Debug, Clone)]
pub struct GeneratedVideoErrorStatusBatchInput {
    pub video_id: i64,
    pub prompt_id: Option<i64>,
    pub status: String,
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
    pub model: String,
    pub expected_file_size: i64,
    pub expected_sha256: String,
    pub file_size: Option<i64>,
    pub status: String,
    pub version: i64,
    pub image_id: Option<i64>,
}

/// Complete image-publication projection used by orchestration and recovery.
/// The database adapter owns conversion from the persistence row.
#[derive(Debug, Clone, PartialEq)]
pub struct ImagePublicationRecord {
    pub publication_id: String,
    pub project_id: i64,
    pub requested_by_user_id: i64,
    pub staging_path: String,
    pub final_path: String,
    pub image_type: String,
    pub prompt: String,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub expected_file_size: i64,
    pub expected_sha256: String,
    pub file_size: Option<i64>,
    pub file_sha256: Option<String>,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    pub metadata_json: Option<Value>,
    pub status: String,
    pub version: i64,
    pub image_id: Option<i64>,
    pub last_error: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaAssetRecord {
    pub id: i64,
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaBindingRecord {
    pub id: i64,
    pub media_asset_id: i64,
    pub target_kind: String,
    pub target_id: i64,
    pub role: String,
    pub created_at: NaiveDateTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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
        assert_eq!(
            image_mime_from_path(Path::new("image.JPEG")),
            Ok("image/jpeg")
        );
        assert!(image_mime_from_path(Path::new("image.gif")).is_err());
        for format in IMAGE_UPLOAD_MEDIA_TYPES {
            for extension in format.accepted_extensions {
                assert_eq!(
                    image_upload_format_from_extension(extension).map(|media| media.mime_type),
                    Some(format.mime_type)
                );
            }
            for mime_type in format.accepted_upload_mime_types {
                assert_eq!(
                    image_upload_format_from_mime(mime_type).map(|media| media.mime_type),
                    Some(format.mime_type)
                );
            }
        }
        for (input, expected) in [
            ("MP4", "mp4"),
            ("m4v", "m4v"),
            ("WebM", "webm"),
            ("mov", "mov"),
        ] {
            assert_eq!(canonical_video_extension(input), Some(expected));
        }
        assert_eq!(canonical_video_extension("avi"), None);
        for (mime_type, expected) in [
            ("video/mp4", ("video/mp4", "mp4")),
            ("VIDEO/WEBM", ("video/webm", "webm")),
            ("video/quicktime", ("video/quicktime", "mov")),
        ] {
            assert_eq!(canonical_video_mime(mime_type), Some(expected));
        }
        assert_eq!(canonical_video_mime("video/x-m4v"), None);
        for (extension, mime_type) in [
            ("mp4", "video/mp4"),
            ("mp4", "application/mp4"),
            ("webm", "video/webm"),
            ("mov", "video/quicktime"),
            ("mov", "video/mov"),
            ("m4v", "video/x-m4v"),
            ("m4v", "video/mp4"),
            ("m4v", "application/mp4"),
        ] {
            assert!(video_upload_mime_matches(extension, mime_type));
        }
        assert!(video_upload_mime_matches("mp4", " APPLICATION/MP4 "));
        assert!(!video_upload_mime_matches("MP4", "video/mp4"));
        assert!(!video_upload_mime_matches("mp4", "video/x-m4v"));
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
        for format in AUDIO_MEDIA_TYPES {
            assert_eq!(
                audio_media_type_from_extension(format.extension).map(|media| media.mime_type),
                Some(format.mime_type)
            );
            for mime_type in format.accepted_upload_mime_types {
                assert_eq!(
                    canonical_audio_mime(mime_type),
                    Ok((format.mime_type, format.extension))
                );
            }
        }
        assert_eq!(
            audio_media_type_from_path(Path::new("voice.M4A"))
                .map(|format| (format.mime_type, format.extension)),
            Ok(("audio/mp4", "m4a"))
        );
        assert!(canonical_audio_mime("application/octet-stream").is_err());
    }
}
