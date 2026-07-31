use super::super::{VideoPromptRequestPreview, VideoPromptRequestPreviewStep};

pub(in crate::video::prompt_preview) fn preview_boundary_from_steps(
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    preview_from_steps_named(
        "chapter_video_boundary_frame_generation",
        "边界帧完整提示词与模型请求",
        steps,
    )
}

pub(in crate::video::prompt_preview) fn preview_first_frame_from_steps(
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    preview_from_steps_named(
        "chapter_video_first_frame_generation",
        "首帧完整提示词与模型请求",
        steps,
    )
}

pub(in crate::video::prompt_preview) fn preview_storyboard_from_steps(
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    preview_from_steps_named(
        "chapter_video_storyboard_generation",
        "分镜完整提示词与模型请求",
        steps,
    )
}

pub(in crate::video::prompt_preview) fn preview_video_prompt_from_steps(
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    preview_from_steps_named(
        "chapter_video_prompt_generation",
        "视频完整提示词与模型请求",
        steps,
    )
}

pub(in crate::video::prompt_preview) fn preview_storyboard_video_prompt_from_steps(
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    preview_from_steps_named(
        "chapter_video_storyboard_prompt_generation",
        "分镜视频完整提示词与模型请求",
        steps,
    )
}

pub(in crate::video::prompt_preview) fn preview_chat_video_prompt_from_steps(
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    preview_from_steps_named(
        "chapter_chat_video_prompt_generation",
        "Chat 视频完整提示词与模型请求",
        steps,
    )
}

fn preview_from_steps_named(
    workflow: &str,
    title: &str,
    steps: Vec<VideoPromptRequestPreviewStep>,
) -> VideoPromptRequestPreview {
    VideoPromptRequestPreview {
        workflow: workflow.to_string(),
        title: title.to_string(),
        steps,
    }
}
