use a4n_domain::source_spans::{ChapterSourceContext, source_metadata_with_flags};
use serde_json::Value;

use super::super::ChapterVideoSourceContext;

mod builders;
mod ledger;
mod project_context;
mod public_context;
mod validation;

pub(in crate::video) use self::builders::{
    FinalVideoPromptContext, FramePromptContext, VideoPlanningPromptContext,
    build_boundary_video_prompt_user, build_chat_video_prompt_user, build_first_frame_prompt_user,
    build_first_frame_video_prompt_user, build_segment_plan_for_shot_user,
    build_single_frame_prompt_user, build_storyboard_prompt_user,
    build_storyboard_video_prompt_user, reference_asset_summary,
};
pub(in crate::video) use self::ledger::{
    VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN, VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT,
    VideoContextManifestInput, VideoContextPreviousOutput, build_final_video_context_manifest,
    build_video_context_manifest,
};
#[cfg(test)]
pub(in crate::video) use self::project_context::{
    video_context_character_names, video_profile_context,
};
pub(in crate::video) use self::public_context::final_video_prompt_public_context;
pub(in crate::video) use self::validation::validate_prompt_options;
pub(in crate::video) use self::validation::validate_prompt_options_with_reference_limit;

pub(in crate::video) fn video_source_metadata(source: &ChapterVideoSourceContext<'_>) -> Value {
    source_metadata_with_flags(
        ChapterSourceContext {
            content: source.content,
        },
        [
            (
                "summary_present",
                source.summary.map(str::trim).is_some_and(|s| !s.is_empty()),
            ),
            ("analysis_present", source.analysis.is_some()),
        ],
    )
}
