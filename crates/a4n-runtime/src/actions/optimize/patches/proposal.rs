use super::{
    super::{
        ProposeOptimizationPatchesInput, ProposeOptimizationPatchesResponse,
        access::verify_optimize_session_access,
        preview::chat_step,
        workflow::{
            OptimizeCommandError, finalize_optimize_workflow_run, record_optimize_command_result,
            start_optimize_workflow_run,
        },
    },
    scope::propose_optimization_patches_task_scope,
};
use crate::{
    AppState,
    actions::request_preview::{ModelRequestPreview, chat_status_preview_for_state},
    prompt_renderer::RuntimePromptRenderer,
};
use a4n_orchestrator::optimize::{OptimizeLlmStream, OptimizeProgress};

pub async fn propose_optimization_patches(
    state: &AppState,
    user_id: i64,
    input: ProposeOptimizationPatchesInput,
    mut progress: Option<OptimizeProgress<'_>>,
    stream: Option<OptimizeLlmStream<'_>>,
) -> Result<ProposeOptimizationPatchesResponse, String> {
    let session_id = input.session_id;
    propose_optimization_patches_task_scope(state, user_id, &input).await?;
    if let Some(progress) = progress.as_mut() {
        progress(0, 4, "正在准备局部修复补丁").map_err(|err| err.to_string())?;
    }
    let workflow_run_id =
        start_optimize_workflow_run(state, session_id, "content_optimization_patch", "patch")
            .await?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let result = state
        .propose_patches(a4n_orchestrator::optimize::OptimizeProposePatchesInput {
            session_id: input.session_id,
            issue_ids: input.issue_ids,
            prompt_renderer: &prompt_renderer,
            agent_instruction_override: input.agent_instruction.as_deref(),
            workflow_run_id,
            progress,
            stream,
        })
        .await
        .map(|proposal| ProposeOptimizationPatchesResponse {
            session_id: proposal.session_id,
            project_id: proposal.project_id,
            chapter_id: proposal.chapter_id,
            base_content_hash: proposal.base_content_hash,
            summary: proposal.summary,
            patches: proposal.patches,
        })
        .map_err(OptimizeCommandError::from_app);
    let result =
        finalize_optimize_workflow_run(state, workflow_run_id, "局部补丁生成", result).await;
    record_optimize_command_result(state, session_id, "局部补丁生成", result).await
}

pub async fn preview_propose_optimization_patches_request(
    state: &AppState,
    user_id: i64,
    input: ProposeOptimizationPatchesInput,
) -> Result<ModelRequestPreview, String> {
    verify_optimize_session_access(state, user_id, input.session_id).await?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let request = state
        .build_propose_patches_preview_request(
            input.session_id,
            input.issue_ids,
            &prompt_renderer,
            input.agent_instruction.as_deref(),
        )
        .await
        .map_err(|err| err.to_string())?;
    chat_status_preview_for_state(
        state,
        "optimization_patch_generation",
        "内容优化局部补丁请求预览",
        vec![chat_step(
            "optimization_patch_generation",
            "局部补丁生成 LLM 请求",
            Some("content_optimization_patch"),
            "ready",
            None,
            request,
        )],
        "llm_chat",
    )
    .await
}
