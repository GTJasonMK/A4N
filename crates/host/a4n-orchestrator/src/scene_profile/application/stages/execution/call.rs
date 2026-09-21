use a4n_error::AppError;

use crate::{
    OrchestratorContext,
    structured_llm::{
        ChatJsonStreamInput, StructuredLlmProgress, StructuredLlmStreamResult,
        stream_error_or_cancelled,
    },
};
use a4n_llm_provider_contract::provider::ChatRequest;

use super::super::super::progress::{SceneProfileSyncProgress, report_scene_profile_sync_progress};

pub(super) async fn chat_scene_profile_stage(
    state: &OrchestratorContext,
    request: ChatRequest,
    prompt_name: &str,
    stage_progress: StructuredLlmProgress,
    progress: Option<&SceneProfileSyncProgress<'_>>,
) -> Result<StructuredLlmStreamResult, AppError> {
    let truncated_message = format!(
        "{prompt_name} 输出达到当前 max_tokens 上限，返回 JSON 被截断；请在模型配置中提高 max_output_tokens，或减少本次章节上下文后重试"
    );
    // 活体信号与字段级里程碑取自同一个 token 流：前者证明"还在输出"，
    // 后者说明"进行到哪了"，二者互补而非替代。
    let mut streamed = String::new();
    let mut stream_sink = progress
        .filter(|progress| progress.has_stream_reporter())
        .map(|progress| {
            move |delta: &str| {
                streamed.push_str(delta);
                progress.stream_text(&streamed);
            }
        });
    let cancel_token = progress.and_then(SceneProfileSyncProgress::cancel_token);
    state
        .chat_json_stream_result_with_sink(
            ChatJsonStreamInput {
                request,
                prompt_name,
                truncated_message: &truncated_message,
                structured_progress: Some(stage_progress),
                cancel_token,
                cancelled_message: "场景档案同步已停止",
            },
            |current, message| {
                report_scene_profile_sync_progress(progress, current, message)
                    .map_err(|error| error.to_string())
            },
            stream_sink
                .as_mut()
                .map(|sink| sink as &mut (dyn FnMut(&str) + Send)),
        )
        .await
        .map_err(|error| stream_error_or_cancelled(error, cancel_token))
}
