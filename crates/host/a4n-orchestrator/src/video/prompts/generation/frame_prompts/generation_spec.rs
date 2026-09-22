use a4n_db_models::models::chapter::chapter_video_prompt;

use super::PromptWorkflowMode;

pub(super) struct FramePromptGenerationSpec {
    pub(super) workflow_mode: PromptWorkflowMode,
    pub(super) frame_prompt_type: &'static str,
    pub(super) frame_source_prompt_type: &'static str,
    pub(super) frame_check_progress: &'static str,
    pub(super) reused_frame_label: &'static str,
    pub(super) frame_generate_progress: &'static str,
    pub(super) frame_regenerate_progress: &'static str,
    pub(super) frame_prompt_name: &'static str,
    pub(super) frame_saved_progress: &'static str,
    pub(super) final_progress: &'static str,
    pub(super) video_prompt_label: &'static str,
    pub(super) video_prompt_name: &'static str,
    pub(super) read_video_inputs_progress: &'static str,
    pub(super) incomplete_frame_label: &'static str,
    pub(super) generate_video_progress: &'static str,
    pub(super) layout: FramePromptLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FramePromptLayout {
    BoundaryFrames,
    InitialFirstFrame,
    PerSegmentStoryboard,
}

impl FramePromptGenerationSpec {
    pub(super) fn is_chat_video(&self) -> bool {
        matches!(self.workflow_mode, PromptWorkflowMode::ChatVideo)
    }

    pub(super) fn is_first_frame_chain(&self) -> bool {
        matches!(self.layout, FramePromptLayout::InitialFirstFrame)
    }

    pub(super) fn is_storyboard(&self) -> bool {
        matches!(self.layout, FramePromptLayout::PerSegmentStoryboard)
    }

    pub(super) fn segment_storage_prompt_type(&self) -> &'static str {
        chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
    }

    pub(super) fn frame_source_prompt_type(&self) -> &'static str {
        self.frame_source_prompt_type
    }
}

pub(super) const BOUNDARY_FRAME_PROMPT_GENERATION: FramePromptGenerationSpec =
    FramePromptGenerationSpec {
        workflow_mode: PromptWorkflowMode::BoundaryFrames,
        frame_prompt_type: chapter_video_prompt::PROMPT_TYPE_FRAME,
        frame_source_prompt_type: chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
        frame_check_progress: "视频片段已规划，正在检查边界帧断点",
        reused_frame_label: "边界帧提示词",
        frame_generate_progress: "正在生成边界帧提示词",
        frame_regenerate_progress: "正在重新生成边界帧提示词",
        frame_prompt_name: "chapter_video_frame_prompts",
        frame_saved_progress: "边界帧提示词已保存，正在写入记录",
        final_progress: "边界帧提示词记录已更新",
        video_prompt_label: "视频提示词",
        video_prompt_name: "chapter_video_prompt",
        read_video_inputs_progress: "正在读取边界帧提示词",
        incomplete_frame_label: "边界帧提示词",
        generate_video_progress: "正在逐段生成视频提示词",
        layout: FramePromptLayout::BoundaryFrames,
    };

pub(super) const FIRST_FRAME_PROMPT_GENERATION: FramePromptGenerationSpec =
    FramePromptGenerationSpec {
        workflow_mode: PromptWorkflowMode::FirstFrameChain,
        frame_prompt_type: chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME,
        frame_source_prompt_type: chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
        frame_check_progress: "视频片段已规划，正在检查初始首帧提示词",
        reused_frame_label: "初始首帧提示词",
        frame_generate_progress: "正在生成初始首帧提示词",
        frame_regenerate_progress: "正在重新生成初始首帧提示词",
        frame_prompt_name: "chapter_video_first_frame_prompts",
        frame_saved_progress: "初始首帧提示词已保存，正在写入记录",
        final_progress: "初始首帧提示词记录已更新",
        video_prompt_label: "首帧串联视频提示词",
        video_prompt_name: "chapter_video_first_frame_video_prompt",
        read_video_inputs_progress: "正在读取共享视频片段规划",
        incomplete_frame_label: "首帧提示词",
        generate_video_progress: "正在逐段生成首帧串联视频提示词",
        layout: FramePromptLayout::InitialFirstFrame,
    };

pub(super) const CHAT_VIDEO_PROMPT_GENERATION: FramePromptGenerationSpec =
    FramePromptGenerationSpec {
        workflow_mode: PromptWorkflowMode::ChatVideo,
        frame_prompt_type: chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME,
        frame_source_prompt_type: chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
        frame_check_progress: "Chat 视频片段已规划，正在检查首尾帧断点",
        reused_frame_label: "Chat 首尾帧提示词",
        frame_generate_progress: "正在生成 Chat 首尾帧提示词",
        frame_regenerate_progress: "正在重新生成 Chat 首尾帧提示词",
        frame_prompt_name: "chapter_video_frame_prompts",
        frame_saved_progress: "Chat 首尾帧提示词已保存，正在写入记录",
        final_progress: "Chat 首尾帧提示词记录已更新",
        video_prompt_label: "Chat 视频提示词",
        video_prompt_name: "chapter_video_chat_prompt",
        read_video_inputs_progress: "正在读取共享视频片段规划",
        incomplete_frame_label: "Chat 首尾帧提示词",
        generate_video_progress: "正在逐段生成 Chat 视频提示词",
        layout: FramePromptLayout::BoundaryFrames,
    };

pub(super) const STORYBOARD_PROMPT_GENERATION: FramePromptGenerationSpec =
    FramePromptGenerationSpec {
        workflow_mode: PromptWorkflowMode::Storyboard,
        frame_prompt_type: chapter_video_prompt::PROMPT_TYPE_STORYBOARD,
        frame_source_prompt_type: chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
        frame_check_progress: "视频片段已规划，正在检查分镜提示词",
        reused_frame_label: "分镜提示词",
        frame_generate_progress: "正在生成分镜提示词",
        frame_regenerate_progress: "正在重新生成分镜提示词",
        frame_prompt_name: "chapter_video_storyboard_prompts",
        frame_saved_progress: "分镜提示词已保存，正在写入记录",
        final_progress: "分镜提示词记录已更新",
        video_prompt_label: "分镜视频提示词",
        video_prompt_name: "chapter_video_storyboard_video_prompt",
        read_video_inputs_progress: "正在读取共享视频片段规划和分镜提示词",
        incomplete_frame_label: "分镜提示词",
        generate_video_progress: "正在逐段生成分镜视频提示词",
        layout: FramePromptLayout::PerSegmentStoryboard,
    };
