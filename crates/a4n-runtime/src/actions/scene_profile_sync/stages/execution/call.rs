use serde_json::{Value, json};

use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::structured_llm::{ChatJsonStreamInput, StructuredLlmProgress};

use crate::state::AppState;

use super::super::super::progress::{SceneProfileSyncProgress, report_scene_profile_sync_progress};
use super::super::super::types::{SceneProfileStageCallResult, SceneProfileStageProgressSpec};

pub(super) async fn chat_scene_profile_stage(
    state: &AppState,
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
    let content = state
        .orchestrator
        .chat_json_stream_with_progress(
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
        )
        .await?;
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
