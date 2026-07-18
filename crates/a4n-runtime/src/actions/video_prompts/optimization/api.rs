mod call;
mod persistence;
mod preparation;

use self::{
    call::run_video_prompt_optimization_call,
    persistence::{
        VideoPromptOptimizationPersistenceInput, persist_video_prompt_optimization_result,
    },
    preparation::prepare_video_prompt_optimization,
};
use super::super::{ChapterVideoPromptSummary, prompt_to_summary};
use super::{
    checkpoint::finalize_video_prompt_optimization_workflow,
    contracts::OptimizeChapterVideoPromptInput,
};
use crate::AppState;

pub async fn optimize_chapter_video_prompt(
    state: &AppState,
    user_id: i64,
    prompt_id: i64,
    input: OptimizeChapterVideoPromptInput,
) -> Result<ChapterVideoPromptSummary, String> {
    let prepared = prepare_video_prompt_optimization(state, user_id, prompt_id, input).await?;
    let preparation::PreparedVideoPromptOptimization {
        prompt,
        mutation_plan,
        prompt_output_language,
        request,
        workflow_run_id,
        call_key,
        generation_input_fingerprint_sha256,
        input_fingerprint_json,
        messages_json,
        input_variables_json,
        source_refs_json,
        media_refs_json,
        request_audit_json,
    } = prepared;

    let result = async {
        let call_result = run_video_prompt_optimization_call(
            state,
            &prompt,
            prompt_output_language,
            request,
            workflow_run_id,
            &call_key,
            &input_fingerprint_json,
        )
        .await?;
        persist_video_prompt_optimization_result(
            state,
            user_id,
            VideoPromptOptimizationPersistenceInput {
                prompt,
                mutation_plan,
                prompt_output_language,
                workflow_run_id,
                call_key,
                generation_input_fingerprint_sha256,
                input_fingerprint_json,
                messages_json,
                input_variables_json,
                source_refs_json,
                media_refs_json,
                request_audit_json,
            },
            call_result,
        )
        .await
    }
    .await;

    if let Err(status_err) =
        finalize_video_prompt_optimization_workflow(state, workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!(
                "视频提示词优化完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result.map(|row| prompt_to_summary(&row))
}
