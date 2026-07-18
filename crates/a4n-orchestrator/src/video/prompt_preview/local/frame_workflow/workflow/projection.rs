use crate::video::prompt_preview::preview_steps::{
    preview_boundary_from_steps, preview_chat_boundary_from_steps, preview_first_frame_from_steps,
    preview_storyboard_from_steps,
};
use crate::video::prompt_preview::{VideoPromptRequestPreview, VideoPromptRequestPreviewStep};

use super::super::super::spec::LocalVideoPromptPreviewMode;

pub(super) fn preview_from_frame_steps(
    mode: LocalVideoPromptPreviewMode,
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    match mode {
        LocalVideoPromptPreviewMode::BoundaryFrames => preview_boundary_from_steps(steps),
        LocalVideoPromptPreviewMode::FirstFrameChain => preview_first_frame_from_steps(steps),
        LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames => {
            preview_chat_boundary_from_steps(steps)
        }
        LocalVideoPromptPreviewMode::Storyboard => preview_storyboard_from_steps(steps),
    }
}

#[cfg(test)]
mod tests {
    use super::preview_from_frame_steps;
    use crate::video::prompt_preview::local::spec::LocalVideoPromptPreviewMode;

    #[test]
    fn projects_frame_preview_modes_to_stable_workflows() {
        let cases = [
            (
                LocalVideoPromptPreviewMode::BoundaryFrames,
                "chapter_video_boundary_frame_generation",
                "边界帧完整提示词与模型请求",
            ),
            (
                LocalVideoPromptPreviewMode::FirstFrameChain,
                "chapter_video_first_frame_generation",
                "首帧完整提示词与模型请求",
            ),
            (
                LocalVideoPromptPreviewMode::ChatVideoBoundaryFrames,
                "chapter_video_chat_boundary_frame_generation",
                "Chat 首尾帧完整提示词与模型请求",
            ),
            (
                LocalVideoPromptPreviewMode::Storyboard,
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
