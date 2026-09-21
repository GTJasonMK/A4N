pub const GENERATION_ENTITY_KINDS: &[&str] =
    &["character", "character_profile", "object", "scene", "event"];
pub const GENERATION_TEXT_ANCHOR_ROLES: &[&str] = &[
    "context",
    "direct_content",
    "previous_context",
    "next_context",
];
pub const GENERATION_ENTITY_ROLES: &[&str] = &[
    "subject",
    "supporting",
    "location",
    "prop",
    "continuity_reference",
];
pub const GENERATION_MEDIA_REF_ROLES: &[&str] = &[
    "first_frame",
    "end_frame",
    "character_reference",
    "object_reference",
    "scene_reference",
    "style_reference",
    "reference",
];

pub const SPEC_STATUS_DRAFT: &str = "draft";
const SPEC_STATUS_READY: &str = "ready";
const SPEC_STATUS_RUNNING: &str = "running";
const SPEC_STATUS_COMPLETED: &str = "completed";
const SPEC_STATUS_FAILED: &str = "failed";
const SPEC_STATUS_CANCELLED: &str = "cancelled";

pub const GENERATION_SPEC_TARGET_KIND_CANVAS_PROMPT: &str = "canvas_prompt";
pub const GENERATION_SPEC_TARGET_KIND_CANVAS_IMAGE: &str = "canvas_image";
pub const GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO: &str = "canvas_video";
pub const GENERATION_SPEC_TARGET_KIND_CANVAS_ASSEMBLY: &str = "canvas_assembly";
pub const GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE: &str = "canvas_node";

pub const GENERATION_SPEC_TARGET_KINDS: &[&str] = &[
    "character_material_image",
    "portrait_expression",
    "object_material_image",
    "scene_material_image",
    "galgame_background",
    "galgame_cg",
    "video_segment",
    "video_segment_plan",
    "video_frame_prompt",
    "boundary_frame",
    "first_frame",
    "final_assembly",
    GENERATION_SPEC_TARGET_KIND_CANVAS_PROMPT,
    GENERATION_SPEC_TARGET_KIND_CANVAS_IMAGE,
    GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO,
    GENERATION_SPEC_TARGET_KIND_CANVAS_ASSEMBLY,
];
pub const GENERATION_SPEC_TARGET_ENTITY_KINDS: &[&str] = &[
    "character",
    "character_profile",
    "object",
    "scene",
    "chapter",
    "segment",
    "assembly",
    "chapter_video_prompt",
    "galgame_scene",
    "galgame_line",
    GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE,
];
pub const GENERATION_SPEC_STATUSES: &[&str] = &[
    SPEC_STATUS_DRAFT,
    SPEC_STATUS_READY,
    SPEC_STATUS_RUNNING,
    SPEC_STATUS_COMPLETED,
    SPEC_STATUS_FAILED,
    SPEC_STATUS_CANCELLED,
];

pub fn is_valid_generation_spec_target_kind(value: &str) -> bool {
    GENERATION_SPEC_TARGET_KINDS.contains(&value)
}

pub fn is_valid_generation_spec_target_entity_kind(value: &str) -> bool {
    GENERATION_SPEC_TARGET_ENTITY_KINDS.contains(&value)
}

pub fn is_valid_generation_entity_kind(value: &str) -> bool {
    GENERATION_ENTITY_KINDS.contains(&value)
}

pub fn is_valid_generation_spec_status(value: &str) -> bool {
    GENERATION_SPEC_STATUSES.contains(&value)
}

pub fn is_valid_generation_text_anchor_role(value: &str) -> bool {
    GENERATION_TEXT_ANCHOR_ROLES.contains(&value)
}

pub fn is_valid_generation_entity_role(value: &str) -> bool {
    GENERATION_ENTITY_ROLES.contains(&value)
}

pub fn is_valid_generation_media_ref_role(value: &str) -> bool {
    GENERATION_MEDIA_REF_ROLES.contains(&value)
}
