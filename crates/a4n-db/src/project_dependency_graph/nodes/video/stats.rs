use a4n_core::models::chapter::chapter_video_prompt;

use crate::project_dependency_graph::{
    helpers::prompt_type_in,
    types::{
        CHAPTER_VIDEO_PROMPT_TYPES, VIDEO_FRAME_PROMPT_TYPES, VIDEO_SEGMENT_PLAN_PROMPT_TYPES,
    },
};

pub(super) struct VideoPromptStats {
    pub(super) segment_plan_count: u64,
    pub(super) frame_prompt_count: u64,
    pub(super) chapter_video_prompt_count: u64,
    pub(super) linked_video_count: u64,
}

pub(super) fn video_prompt_stats(
    video_prompts: &[chapter_video_prompt::Model],
) -> VideoPromptStats {
    let mut stats = VideoPromptStats {
        segment_plan_count: 0,
        frame_prompt_count: 0,
        chapter_video_prompt_count: 0,
        linked_video_count: 0,
    };

    for row in video_prompts {
        if prompt_type_in(row, VIDEO_SEGMENT_PLAN_PROMPT_TYPES) {
            stats.segment_plan_count += 1;
        }
        if prompt_type_in(row, VIDEO_FRAME_PROMPT_TYPES) {
            stats.frame_prompt_count += 1;
        }
        if prompt_type_in(row, CHAPTER_VIDEO_PROMPT_TYPES) {
            stats.chapter_video_prompt_count += 1;
        }
        if row.generated_video_id.is_some() {
            stats.linked_video_count += 1;
        }
    }

    stats
}
