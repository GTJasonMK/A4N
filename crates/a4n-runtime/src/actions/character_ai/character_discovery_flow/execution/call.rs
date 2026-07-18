use super::super::{
    contracts::{
        CHARACTER_DISCOVERY_CALL_KEY, CHARACTER_DISCOVERY_FIELDS, CHARACTER_DISCOVERY_LLM_SPAN,
        CHARACTER_DISCOVERY_PROGRESS_TOTAL,
    },
    rules::{parse_and_validate_character_discovery_content, report_character_discovery_progress},
    workflow::{
        character_discovery_call_fingerprint_json, character_discovery_parsed_output_json,
        load_completed_character_discovery_call_checkpoint,
        save_completed_character_discovery_call_checkpoint,
    },
};
use crate::{
    AppState,
    actions::{
        character_ai::CharacterAiProgress, llm_request::chat_request_with_workflow_run_id,
        workflow_checkpoint::llm_call_checkpoint_fingerprint_sha256,
    },
};
use a4n_core::models::{chapter::chapter, project::text_anchor};
use a4n_core::structured_progress::StructuredJsonProgressSpec;
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::structured_llm::{ChatJsonStreamInput, StructuredLlmProgress};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) struct CharacterDiscoveryCallResult {
    pub(super) discovered: Vec<Value>,
    pub(super) response_content: String,
    pub(super) llm_request: ChatRequest,
    pub(super) analyzed_chapters: BTreeSet<i32>,
}

pub(super) async fn run_character_discovery_call(
    state: &AppState,
    workflow_run_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    request: &ChatRequest,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<CharacterDiscoveryCallResult, String> {
    report_character_discovery_progress(
        progress,
        10,
        CHARACTER_DISCOVERY_PROGRESS_TOTAL,
        "正在构造角色发现提示词",
        "角色发现已停止",
    )?;
    report_character_discovery_progress(
        progress,
        CHARACTER_DISCOVERY_LLM_SPAN.start(),
        CHARACTER_DISCOVERY_PROGRESS_TOTAL,
        "正在调用模型发现角色",
        "角色发现已停止",
    )?;
    let mut llm_request = chat_request_with_workflow_run_id(request.clone(), workflow_run_id);
    let analyzed_chapters = chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .collect::<BTreeSet<_>>();
    let call_fingerprint_json =
        character_discovery_call_fingerprint_json(project_id, chapter_number, &llm_request)?;
    let call_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&call_fingerprint_json)
        .map_err(|err| err.to_string())?;
    if let Some(trace_context) = llm_request.trace_context.as_mut() {
        trace_context.call_key = Some(CHARACTER_DISCOVERY_CALL_KEY.to_string());
        trace_context.input_fingerprint_sha256 = Some(call_fingerprint_sha256);
    }

    let (discovered, response_content) = if let Some(checkpoint) =
        load_completed_character_discovery_call_checkpoint(
            state,
            workflow_run_id,
            &call_fingerprint_json,
        )
        .await?
    {
        report_character_discovery_progress(
            progress,
            80,
            CHARACTER_DISCOVERY_PROGRESS_TOTAL,
            "角色发现已从断点恢复，正在写入发现结果",
            "角色发现已停止",
        )?;
        let discovered = parse_and_validate_character_discovery_content(
            &checkpoint.response_content,
            &analyzed_chapters,
            chapters,
            text_anchors,
        )?;
        let parsed_output = character_discovery_parsed_output_json(&discovered);
        if parsed_output != checkpoint.parsed_output {
            return Err(format!(
                "{} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致",
                CHARACTER_DISCOVERY_CALL_KEY
            ));
        }
        (discovered, checkpoint.response_content)
    } else {
        let response_content = state
            .orchestrator
            .chat_json_stream_with_progress(
                ChatJsonStreamInput {
                    request: llm_request.clone(),
                    prompt_name: "character_discovery",
                    truncated_message: "角色发现输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“角色发现”输出档位，或减少本次章节上下文后重试",
                    structured_progress: Some(StructuredLlmProgress {
                        spec: StructuredJsonProgressSpec {
                            root_array_key: "characters",
                            item_label: "角色",
                            expected_items: None,
                            milestones: CHARACTER_DISCOVERY_FIELDS,
                        },
                        span: CHARACTER_DISCOVERY_LLM_SPAN,
                    }),
                    cancel_token: progress.and_then(|progress| progress.cancel_token),
                    cancelled_message: "角色发现已停止",
                },
                |current, message| {
                    report_character_discovery_progress(
                        progress,
                        current,
                        CHARACTER_DISCOVERY_PROGRESS_TOTAL,
                        message,
                        "角色发现已停止",
                    )
                },
            )
            .await?;

        report_character_discovery_progress(
            progress,
            80,
            CHARACTER_DISCOVERY_PROGRESS_TOTAL,
            "模型已返回，正在解析发现结果",
            "角色发现已停止",
        )?;
        let discovered = parse_and_validate_character_discovery_content(
            &response_content,
            &analyzed_chapters,
            chapters,
            text_anchors,
        )?;
        let parsed_output = character_discovery_parsed_output_json(&discovered);
        save_completed_character_discovery_call_checkpoint(
            state,
            workflow_run_id,
            call_fingerprint_json,
            &response_content,
            parsed_output,
        )
        .await?;
        (discovered, response_content)
    };

    Ok(CharacterDiscoveryCallResult {
        discovered,
        response_content,
        llm_request,
        analyzed_chapters,
    })
}
