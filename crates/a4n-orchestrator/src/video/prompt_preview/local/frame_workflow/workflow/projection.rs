use crate::video::prompt_preview::preview_steps::{
    preview_boundary_from_steps, preview_first_frame_from_steps, preview_storyboard_from_steps,
};
use crate::video::prompt_preview::{VideoPromptRequestPreview, VideoPromptRequestPreviewStep};

use super::super::super::spec::LocalFramePromptPreviewMode;

pub(super) fn preview_from_frame_steps(
    mode: LocalFramePromptPreviewMode,
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    match mode {
        LocalFramePromptPreviewMode::BoundaryFrames => preview_boundary_from_steps(steps),
        LocalFramePromptPreviewMode::FirstFrameChain => preview_first_frame_from_steps(steps),
        LocalFramePromptPreviewMode::Storyboard => preview_storyboard_from_steps(steps),
    }
}

#[cfg(test)]
mod tests {
    use super::preview_from_frame_steps;
    use crate::video::prompt_preview::local::spec::LocalFramePromptPreviewMode;

    #[test]
    fn projects_frame_preview_modes_to_stable_workflows() {
        let cases = [
            (
                LocalFramePromptPreviewMode::BoundaryFrames,
                "chapter_video_boundary_frame_generation",
                "边界帧完整提示词与模型请求",
            ),
            (
                LocalFramePromptPreviewMode::FirstFrameChain,
                "chapter_video_first_frame_generation",
                "首帧完整提示词与模型请求",
            ),
            (
                LocalFramePromptPreviewMode::Storyboard,
                "chapter_video_storyboard_generation",
                "分镜完整提示词与模型请求",
            ),
        ];

        for (mode, expected_workflow, expected_title) in cases {
            let preview = preview_from_frame_steps(mode, Vec::new());
            assert_eq!(preview.workflow, expected_workflow);
            assert_eq!(preview.title, expected_title);
            assert!(preview.steps.is_empty());
        }
    }
}
