use crate::actions::dependency_conflict::DownstreamConflictStrategy;
use serde::{Deserialize, Serialize};

pub(super) const CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME: &str =
    "chapter_video_prompt_optimization";
pub(super) const CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME: &str =
    "chapter_video_prompt_optimization_user";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptimizeChapterVideoPromptInput {
    pub instruction: String,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::actions::video_prompts) struct OptimizedVideoPromptFields {
    pub visual_prompt: String,
    pub motion_prompt: Option<String>,
    pub negative_prompt: Option<String>,
}
