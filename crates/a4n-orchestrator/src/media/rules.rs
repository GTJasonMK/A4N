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

pub(in crate::media) const MEDIA_KINDS: &[&str] =
    &[MEDIA_KIND_IMAGE, MEDIA_KIND_VIDEO, MEDIA_KIND_AUDIO];
pub(in crate::media) const MEDIA_ORIGINS: &[&str] = &[
    MEDIA_ORIGIN_GENERATED,
    MEDIA_ORIGIN_IMPORTED,
    MEDIA_ORIGIN_EXTRACTED_FRAME,
    MEDIA_ORIGIN_ASSEMBLED_PREVIEW,
];
pub(in crate::media) const MEDIA_STATUSES: &[&str] = &[
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
    "chapter_manga_prompt",
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
                galgame_media_binding_required_media_kind(MEDIA_BINDING_TARGET_GALGAME_LINE, role,),
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
}
