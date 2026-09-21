use crate::video::prompt_generation::frame_prompts::{
    AppError, FinalVideoPromptContext, FramePromptGenerationSpec, PromptWorkflowMode,
    build_boundary_video_prompt_user, build_chat_video_prompt_user,
    build_first_frame_video_prompt_user, build_storyboard_video_prompt_user,
};

pub(super) async fn build_video_prompt_user_for_spec(
    spec: &FramePromptGenerationSpec,
    context: FinalVideoPromptContext<'_>,
) -> Result<String, AppError> {
    match spec.workflow_mode {
        PromptWorkflowMode::SharedSegmentPlan => Err(AppError::InvalidArgument(
            "共享片段规划不能生成卡片最终提示词".to_string(),
        )),
        PromptWorkflowMode::BoundaryFrames => build_boundary_video_prompt_user(context).await,
        PromptWorkflowMode::FirstFrameChain => build_first_frame_video_prompt_user(context).await,
        PromptWorkflowMode::ChatVideo => build_chat_video_prompt_user(context).await,
        PromptWorkflowMode::Storyboard => build_storyboard_video_prompt_user(context).await,
    }
}
