use super::super::super::ProviderGenerationWorkflow;
use a4n_core::models::chapter::chapter_video_prompt;

pub(in crate::actions::video_generation) fn ensure_provider_generation_workflow(
    prompt: &chapter_video_prompt::Model,
) -> Result<(), String> {
    match prompt.workflow_mode.as_str() {
        "boundary_frames" | "first_frame_chain" | "storyboard" => Ok(()),
        "chat_video" => Err(format!(
            "视频提示词 #{} 属于 Chat 生视频模式，不使用应用内视频 provider 生成；请复制提示词到外部 Chat 工具生成后导入视频",
            prompt.id
        )),
        other => Err(format!(
            "视频提示词 #{} workflow_mode 无效: {}",
            prompt.id, other
        )),
    }
}

pub(in crate::actions::video_generation) fn prompt_provider_workflow(
    prompt: &chapter_video_prompt::Model,
) -> Result<ProviderGenerationWorkflow, String> {
    ensure_provider_generation_workflow(prompt)?;
    match prompt.workflow_mode.as_str() {
        "boundary_frames" => Ok(ProviderGenerationWorkflow::BoundaryFrames),
        "first_frame_chain" => Ok(ProviderGenerationWorkflow::FirstFrameChain),
        "storyboard" => Ok(ProviderGenerationWorkflow::Storyboard),
        other => Err(format!(
            "视频提示词 #{} workflow_mode 无效: {}",
            prompt.id, other
        )),
    }
}

pub(in crate::actions::video_generation) fn prompt_is_first_frame_chain(
    prompt: &chapter_video_prompt::Model,
) -> Result<bool, String> {
    Ok(prompt_provider_workflow(prompt)?.uses_first_frame_chain())
}
