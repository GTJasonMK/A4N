//! 首帧链提示词与尾帧图片匹配规则（无 DB、无 Host）。

use a4n_core::models::chapter::chapter_video_prompt;

use super::{prompt_chain_matches, prompt_source_type_matches};

pub fn first_frame_prompt_matches_video_prompt(
    video_prompt: &chapter_video_prompt::Model,
    first_frame_prompt: &chapter_video_prompt::Model,
) -> bool {
    prompt_chain_matches(
        chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN,
        video_prompt,
        first_frame_prompt,
    ) && prompt_uses_shared_segment_source(video_prompt)
        && prompt_uses_shared_segment_source(first_frame_prompt)
}

fn prompt_uses_shared_segment_source(prompt: &chapter_video_prompt::Model) -> bool {
    prompt_source_type_matches(prompt, chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN)
}
