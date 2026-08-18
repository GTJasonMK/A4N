//! 章节视频提示词优化类型合同（无 DB、无 Host）。

use a4n_core::{
    error::AppError,
    models::chapter::chapter_video_prompt,
    prompt_language::prompt_type_uses_prompt_output_language,
};

use super::{
    prompt_records::expected_contract_version, prompt_store::PromptWorkflowMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VideoPromptOptimizationKind {
    ProviderVideo,
    ChatVideo,
    Frame,
}

pub(super) fn video_prompt_optimization_kind(
    prompt: &chapter_video_prompt::Model,
) -> Result<VideoPromptOptimizationKind, AppError> {
    let workflow_mode = PromptWorkflowMode::from_row(prompt)?;
    let prompt_type = prompt.prompt_type.as_str();
    let pair_is_valid = expected_contract_version(workflow_mode.as_str(), prompt_type).is_some();
    let uses_prompt_output_language = prompt_type_uses_prompt_output_language(prompt_type);
    match (prompt_type, workflow_mode) {
        (chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN, _) => {
            Err(AppError::InvalidArgument(format!(
                "视频片段规划 #{} 不是可优化的画面或视频提示词",
                prompt.id
            )))
        }
        (chapter_video_prompt::PROMPT_TYPE_VIDEO, _) if !pair_is_valid => {
            Err(AppError::InvalidArgument(format!(
                "视频提示词 #{} 的类型 {} 与 workflow_mode {} 不匹配",
                prompt.id,
                prompt_type,
                workflow_mode.as_str()
            )))
        }
        (chapter_video_prompt::PROMPT_TYPE_VIDEO, PromptWorkflowMode::ChatVideo) => {
            Ok(VideoPromptOptimizationKind::ChatVideo)
        }
        (chapter_video_prompt::PROMPT_TYPE_VIDEO, _) => {
            Ok(VideoPromptOptimizationKind::ProviderVideo)
        }
        _ if uses_prompt_output_language && pair_is_valid => {
            Ok(VideoPromptOptimizationKind::Frame)
        }
        (prompt_type, _) if uses_prompt_output_language => {
            Err(AppError::InvalidArgument(format!(
                "视频提示词 #{} 的类型 {} 与 workflow_mode {} 不匹配",
                prompt.id,
                prompt_type,
                workflow_mode.as_str()
            )))
        }
        (prompt_type, _) => Err(AppError::InvalidArgument(format!(
            "视频提示词 #{} 不支持优化：prompt_type={}，workflow_mode={}",
            prompt.id,
            prompt_type,
            workflow_mode.as_str()
        ))),
    }
}
