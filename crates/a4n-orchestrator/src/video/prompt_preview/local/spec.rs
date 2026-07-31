use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use a4n_core::error::AppError;

use super::{PromptWorkflowMode, VideoSegmentPlan};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LocalVideoPromptPreviewMode {
    BoundaryFrames,
    FirstFrameChain,
    ChatVideo,
    Storyboard,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LocalFramePromptPreviewMode {
    BoundaryFrames,
    FirstFrameChain,
    Storyboard,
}

#[derive(Clone, Copy)]
pub(super) struct LocalVideoPromptPreviewSpec {
    pub(super) mode: LocalVideoPromptPreviewMode,
    pub(super) reuse_sequence: VideoPromptReuseSequence,
    pub(super) segment_prompt_type: &'static str,
    pub(super) segment_title: &'static str,
    pub(super) segment_reused_note: &'static str,
    pub(super) frame_key: &'static str,
    pub(super) frame_title: &'static str,
    pub(super) frame_prompt_type: &'static str,
    pub(super) frame_source_prompt_type: &'static str,
    pub(super) frame_prompt_name: &'static str,
    pub(super) video_title: &'static str,
    pub(super) video_prompt_name: &'static str,
    pub(super) video_placeholder_note: &'static str,
}

impl LocalVideoPromptPreviewSpec {
    pub(super) fn prompt_workflow_mode(self) -> PromptWorkflowMode {
        match self.mode {
            LocalVideoPromptPreviewMode::BoundaryFrames => PromptWorkflowMode::BoundaryFrames,
            LocalVideoPromptPreviewMode::FirstFrameChain => PromptWorkflowMode::FirstFrameChain,
            LocalVideoPromptPreviewMode::ChatVideo => PromptWorkflowMode::ChatVideo,
            LocalVideoPromptPreviewMode::Storyboard => PromptWorkflowMode::Storyboard,
        }
    }

    pub(super) fn workflow_mode(self) -> &'static str {
        self.prompt_workflow_mode().as_str()
    }

    pub(super) fn frame_mode(self) -> Option<LocalFramePromptPreviewMode> {
        match self.mode {
            LocalVideoPromptPreviewMode::BoundaryFrames => {
                Some(LocalFramePromptPreviewMode::BoundaryFrames)
            }
            LocalVideoPromptPreviewMode::FirstFrameChain => {
                Some(LocalFramePromptPreviewMode::FirstFrameChain)
            }
            LocalVideoPromptPreviewMode::Storyboard => {
                Some(LocalFramePromptPreviewMode::Storyboard)
            }
            LocalVideoPromptPreviewMode::ChatVideo => None,
        }
    }

    pub(super) fn boundary_frames() -> Self {
        Self {
            mode: LocalVideoPromptPreviewMode::BoundaryFrames,
            reuse_sequence: VideoPromptReuseSequence::NearestPrevious,
            segment_prompt_type: "frame_segment_plan",
            segment_title: "视频片段规划",
            segment_reused_note: "已存在当前章节片段规划，视频提示词请求会直接使用该前置结果",
            frame_key: "frame_prompts",
            frame_title: "边界帧提示词",
            frame_prompt_type: "frame_prompt",
            frame_source_prompt_type: "frame_segment_plan",
            frame_prompt_name: "chapter_video_frame_prompts",
            video_title: "视频提示词",
            video_prompt_name: "chapter_video_prompt",
            video_placeholder_note: "结构预览：真实生成时 previous_steps.chapter_video_segment_plan 和 chapter_video_frame_prompts 会替换为前序模型输出；这里用占位前序结果展示单段视频提示词请求格式。",
        }
    }

    pub(super) fn first_frame_chain() -> Self {
        Self {
            mode: LocalVideoPromptPreviewMode::FirstFrameChain,
            reuse_sequence: VideoPromptReuseSequence::ContiguousPrefix,
            segment_prompt_type: "frame_segment_plan",
            segment_title: "视频片段规划",
            segment_reused_note: "已存在当前章节共享片段规划，视频提示词请求会直接使用该前置结果",
            frame_key: "first_frame_prompts",
            frame_title: "初始首帧提示词",
            frame_prompt_type: "first_frame_prompt",
            frame_source_prompt_type: "frame_segment_plan",
            frame_prompt_name: "chapter_video_first_frame_prompts",
            video_title: "首帧串联视频提示词",
            video_prompt_name: "chapter_video_first_frame_video_prompt",
            video_placeholder_note: "结构预览：真实生成时 previous_steps.chapter_video_segment_plan 和 chapter_video_first_frame_prompts 会替换为前序模型输出；这里用占位前序结果展示首帧串联视频提示词请求格式。",
        }
    }

    pub(super) fn chat_video() -> Self {
        Self {
            mode: LocalVideoPromptPreviewMode::ChatVideo,
            reuse_sequence: VideoPromptReuseSequence::NearestPrevious,
            segment_prompt_type: "frame_segment_plan",
            segment_title: "视频片段规划",
            segment_reused_note: "已存在当前章节共享片段规划，Chat 视频提示词请求会直接使用该前置结果",
            frame_key: "frames",
            frame_title: "",
            frame_prompt_type: "",
            frame_source_prompt_type: "frame_segment_plan",
            frame_prompt_name: "",
            video_title: "Chat 视频提示词",
            video_prompt_name: "chapter_video_chat_prompt",
            video_placeholder_note: "结构预览：真实生成时 previous_steps.chapter_video_segment_plan 会替换为共享片段规划模型输出；Chat 方案不生成中间帧提示词。",
        }
    }

    pub(super) fn storyboard() -> Self {
        Self {
            mode: LocalVideoPromptPreviewMode::Storyboard,
            reuse_sequence: VideoPromptReuseSequence::ContiguousPrefix,
            segment_prompt_type: "frame_segment_plan",
            segment_title: "视频片段规划",
            segment_reused_note: "已存在当前章节共享片段规划，分镜视频提示词请求会直接使用该前置结果",
            frame_key: "storyboard_prompts",
            frame_title: "分镜提示词",
            frame_prompt_type: "storyboard_prompt",
            frame_source_prompt_type: "frame_segment_plan",
            frame_prompt_name: "chapter_video_storyboard_prompts",
            video_title: "分镜视频提示词",
            video_prompt_name: "chapter_video_storyboard_video_prompt",
            video_placeholder_note: "结构预览：真实生成时 previous_steps.chapter_video_segment_plan 和 chapter_video_storyboard_prompts 会替换为前序模型输出；这里用占位前序结果展示单段分镜视频提示词请求格式。",
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum VideoPromptReuseSequence {
    NearestPrevious,
    ContiguousPrefix,
}

pub(super) enum VideoPromptPreviewTarget<'a, T> {
    Empty,
    Complete,
    Pending {
        current_segment: &'a VideoSegmentPlan,
        previous_shot: Option<&'a T>,
    },
}

pub(super) fn video_prompt_target<'a, T>(
    segment_plan: &'a [VideoSegmentPlan],
    reusable_video_prompts: &'a BTreeMap<i32, T>,
    regenerate_existing: bool,
    sequence: VideoPromptReuseSequence,
) -> VideoPromptPreviewTarget<'a, T> {
    if segment_plan.is_empty() {
        return VideoPromptPreviewTarget::Empty;
    }
    if !regenerate_existing
        && segment_plan
            .iter()
            .all(|segment| reusable_video_prompts.contains_key(&segment.segment_number))
    {
        return VideoPromptPreviewTarget::Complete;
    }

    match sequence {
        VideoPromptReuseSequence::NearestPrevious => {
            let current_segment = if regenerate_existing {
                &segment_plan[0]
            } else {
                let Some(segment) = segment_plan
                    .iter()
                    .find(|segment| !reusable_video_prompts.contains_key(&segment.segment_number))
                else {
                    return VideoPromptPreviewTarget::Complete;
                };
                segment
            };
            let previous_shot = reusable_video_prompts
                .range(..current_segment.segment_number)
                .next_back()
                .map(|(_, prompt)| prompt);
            VideoPromptPreviewTarget::Pending {
                current_segment,
                previous_shot,
            }
        }
        VideoPromptReuseSequence::ContiguousPrefix => {
            if regenerate_existing {
                return VideoPromptPreviewTarget::Pending {
                    current_segment: &segment_plan[0],
                    previous_shot: None,
                };
            }
            let mut previous_shot = None;
            for segment in segment_plan {
                if let Some(reusable) = reusable_video_prompts.get(&segment.segment_number) {
                    previous_shot = Some(reusable);
                    continue;
                }
                return VideoPromptPreviewTarget::Pending {
                    current_segment: segment,
                    previous_shot,
                };
            }
            VideoPromptPreviewTarget::Complete
        }
    }
}

pub(super) fn prompt_values_by_shot<T: Serialize>(
    prompts: BTreeMap<i32, T>,
) -> Result<BTreeMap<i32, Value>, AppError> {
    prompts
        .into_iter()
        .map(|(shot_number, prompt)| Ok((shot_number, serde_json::to_value(prompt)?)))
        .collect()
}
