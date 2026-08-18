use serde::{Deserialize, Serialize};

pub const GENERATION_PROMPT_KIND_IMAGE_PROMPT: &str = "image_prompt";
pub const GENERATION_PROMPT_KIND_VIDEO_PROMPT: &str = "video_prompt";
pub const GENERATION_PROMPT_KIND_CANVAS_PROMPT: &str = "canvas_prompt";
pub const GENERATION_PROMPT_KIND_CANVAS_SHOT_SCRIPT: &str = "shot_script";
pub const GENERATION_PROMPT_KIND_CANVAS_STORYBOARD: &str = "storyboard";
pub const GENERATION_PROMPT_KIND_CANVAS_VISUAL_DESCRIPTION: &str = "visual_description";
pub const GENERATION_PROMPT_KIND_CANVAS_DIALOGUE: &str = "dialogue";
pub const GENERATION_PROMPT_KIND_CANVAS_RHYTHM_PLAN: &str = "rhythm_plan";

pub const GENERATION_PROMPT_KINDS: &[&str] = &[
    GENERATION_PROMPT_KIND_IMAGE_PROMPT,
    GENERATION_PROMPT_KIND_VIDEO_PROMPT,
    "chat_video_prompt",
    "provider_video_prompt",
    "negative_prompt",
    "manga_pipeline_output",
    "manga_page_prompt",
    "video_segment_plan",
    GENERATION_PROMPT_KIND_CANVAS_PROMPT,
    GENERATION_PROMPT_KIND_CANVAS_SHOT_SCRIPT,
    GENERATION_PROMPT_KIND_CANVAS_STORYBOARD,
    GENERATION_PROMPT_KIND_CANVAS_VISUAL_DESCRIPTION,
    GENERATION_PROMPT_KIND_CANVAS_DIALOGUE,
    GENERATION_PROMPT_KIND_CANVAS_RHYTHM_PLAN,
    "background_image_prompt",
    "event_cg_prompt",
];
pub(in crate::generation) const INTERNAL_PROMPT_TOKENS: [&str; 5] = [
    "workflow_mode",
    "contract_version",
    "scene_ref",
    "must_repeat_from_previous",
    "duration_capability",
];
pub const VIDEO_PROMPT_REQUIRED_SECTIONS: [&str; 6] = [
    "[主体定义与素材绑定]",
    "[片段内容以及前因后果]",
    "[镜头分镜]",
    "[画面风格描述]",
    "[连续性与结尾状态]",
    "[限制与避免项]",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArtifactInsertInput {
    pub generation_spec_id: i64,
    pub prompt_kind: String,
    pub prompt_text: String,
    pub renderer_version: String,
    pub edited_by_user: bool,
}
