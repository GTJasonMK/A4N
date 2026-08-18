use crate::models::chapter::chapter_video_prompt;
use crate::video_execution::VideoSegmentWorkflowModeV1;

pub const WORKFLOW_MODE_MIXED: &str = "mixed";
pub const WORKFLOW_MODE_CANVAS_VIDEO: &str = "canvas_video";
pub const WORKFLOW_MODE_CANVAS_TRIM: &str = "canvas_trim";
pub const WORKFLOW_MODE_CANVAS_ASSEMBLY: &str = "canvas_assembly";
pub const CANVAS_WORKFLOW_MODES: [&str; 3] = [
    WORKFLOW_MODE_CANVAS_VIDEO,
    WORKFLOW_MODE_CANVAS_TRIM,
    WORKFLOW_MODE_CANVAS_ASSEMBLY,
];

pub fn is_prompt_video_workflow_mode(mode: &str) -> bool {
    chapter_video_prompt::WORKFLOW_MODES.contains(&mode)
}

pub fn is_assembly_video_workflow_mode(mode: &str) -> bool {
    mode == VideoSegmentWorkflowModeV1::ReferenceMedia.as_str()
        || matches!(
            mode,
            chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES
                | chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN
                | chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO
                | chapter_video_prompt::WORKFLOW_MODE_STORYBOARD
                | WORKFLOW_MODE_MIXED
        )
}

pub fn is_generated_video_workflow_mode(mode: &str) -> bool {
    is_assembly_video_workflow_mode(mode) || CANVAS_WORKFLOW_MODES.contains(&mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_video_modes_include_assembly_and_canvas_modes() {
        assert!(is_assembly_video_workflow_mode(WORKFLOW_MODE_MIXED));
        assert!(is_generated_video_workflow_mode(WORKFLOW_MODE_MIXED));
        assert!(is_generated_video_workflow_mode(
            WORKFLOW_MODE_CANVAS_ASSEMBLY
        ));
        assert!(!is_assembly_video_workflow_mode(
            WORKFLOW_MODE_CANVAS_ASSEMBLY
        ));
        assert!(!is_generated_video_workflow_mode("legacy"));
    }

    #[test]
    fn prompt_video_modes_are_distinct_from_storage_and_assembly_modes() {
        for mode in [
            "shared_segment_plan",
            "boundary_frames",
            "first_frame_chain",
            "chat_video",
            "storyboard",
        ] {
            assert!(is_prompt_video_workflow_mode(mode));
        }
        assert!(!is_prompt_video_workflow_mode("reference_media"));
        assert!(!is_prompt_video_workflow_mode(WORKFLOW_MODE_MIXED));
        assert!(!is_prompt_video_workflow_mode("legacy"));
    }
}
