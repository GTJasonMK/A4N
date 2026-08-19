use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    structured_llm::{ChatJsonStreamInput, StructuredLlmProgress},
};
use a4n_llm::provider::ChatRequest;

use super::super::super::progress::{SceneProfileSyncProgress, report_scene_profile_sync_progress};
use super::super::super::types::{SceneProfileStageCallResult, SceneProfileStageProgressSpec};

pub(super) async fn chat_scene_profile_stage(
    state: &OrchestratorContext,
    request: ChatRequest,
    prompt_name: &str,
    root_field: &str,
    stage_progress: SceneProfileStageProgressSpec,
    progress: Option<&SceneProfileSyncProgress<'_>>,
    response_chars: &mut Vec<Value>,
) -> Result<SceneProfileStageCallResult, String> {
    let truncated_message = format!(
        "{prompt_name} 输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“角色/档案发现”输出档位，或减少本次章节上下文后重试"
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
    let content = state
        .chat_json_stream_result_with_sink(
            ChatJsonStreamInput {
                request,
                prompt_name,
                truncated_message: &truncated_message,
                structured_progress: Some(StructuredLlmProgress {
                    spec: stage_progress.structured,
                    span: stage_progress.span,
                }),
                cancel_token: progress.and_then(|progress| progress.cancel_token),
                cancelled_message: "场景档案同步已停止",
            },
            |current, message| {
                report_scene_profile_sync_progress(progress, current, message, "场景档案同步已停止")
            },
            stream_sink
                .as_mut()
                .map(|sink| sink as &mut (dyn FnMut(&str) + Send)),
        )
        .await?
        .response_content;
    response_chars.push(json!({
        "prompt_name": prompt_name,
        "response_chars": content.chars().count()}));
    let parsed = super::super::super::parsing::parse_scene_stage_root_array(
        &content,
        prompt_name,
        root_field,
    )?;
    let item_count = parsed.as_array().map_or(0, Vec::len);
    report_scene_profile_sync_progress(
        progress,
        stage_progress.span.end(),
        format!("{prompt_name} 已完成：解析到 {item_count} 条{root_field}"),
        "场景档案同步已停止",
    )?;
    Ok(SceneProfileStageCallResult {
        output: parsed,
        response_content: content,
    })
}
