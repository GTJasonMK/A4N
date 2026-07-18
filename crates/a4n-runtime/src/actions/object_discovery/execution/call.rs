use super::super::parsing::parse_and_validate_object_discovery_output;
use super::super::progress::{
    OBJECT_DISCOVERY_CALL_KEY, OBJECT_DISCOVERY_FIELDS, OBJECT_DISCOVERY_LLM_SPAN,
    ObjectDiscoveryProgress, report_object_discovery_progress,
};
use super::super::types::ParsedObjectProfileDraft;
use super::super::workflow::{
    load_completed_object_discovery_call_checkpoint, object_discovery_call_fingerprint_json,
    save_completed_object_discovery_call_checkpoint,
};
use crate::actions::llm_request::chat_request_with_workflow_run_id;
use crate::actions::workflow_checkpoint::llm_call_checkpoint_fingerprint_sha256;
use crate::state::AppState;
use a4n_core::models::{chapter::chapter, project::text_anchor};
use a4n_core::structured_progress::StructuredJsonProgressSpec;
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::structured_llm::{ChatJsonStreamInput, StructuredLlmProgress};
use std::collections::BTreeSet;

pub(super) struct ObjectDiscoveryCallResult {
    pub(super) parsed_drafts: Vec<ParsedObjectProfileDraft>,
    pub(super) response_content: String,
    pub(super) llm_request: ChatRequest,
    pub(super) analyzed_chapters: BTreeSet<i32>,
}

pub(super) async fn run_object_discovery_call(
    state: &AppState,
    workflow_run_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    request: &ChatRequest,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    progress: Option<&ObjectDiscoveryProgress<'_>>,
) -> Result<ObjectDiscoveryCallResult, String> {
    report_object_discovery_progress(
        progress,
        OBJECT_DISCOVERY_LLM_SPAN.start(),
        "正在调用模型识别关键物体",
        "物体档案发现已停止",
    )?;
    let mut llm_request = chat_request_with_workflow_run_id(request.clone(), workflow_run_id);
    let analyzed_chapters = chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .collect::<BTreeSet<_>>();
    let call_fingerprint_json =
        object_discovery_call_fingerprint_json(project_id, chapter_number, &llm_request)?;
    let call_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&call_fingerprint_json)
        .map_err(|err| err.to_string())?;
    if let Some(trace_context) = llm_request.trace_context.as_mut() {
        trace_context.call_key = Some(OBJECT_DISCOVERY_CALL_KEY.to_string());
        trace_context.input_fingerprint_sha256 = Some(call_fingerprint_sha256);
    }

    let (parsed_drafts, response_content) = if let Some(checkpoint) =
        load_completed_object_discovery_call_checkpoint(
            state,
            workflow_run_id,
            &call_fingerprint_json,
        )
        .await?
    {
        report_object_discovery_progress(
            progress,
            80,
            "物体档案发现已从断点恢复，正在写入物体档案",
            "物体档案发现已停止",
        )?;
        let (parsed_drafts, parsed_output) = parse_and_validate_object_discovery_output(
            &checkpoint.response_content,
            &analyzed_chapters,
            chapters,
            text_anchors,
        )?;
        if parsed_output != checkpoint.parsed_output {
            return Err(format!(
                "{} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致",
                OBJECT_DISCOVERY_CALL_KEY
            ));
        }
        (parsed_drafts, checkpoint.response_content)
    } else {
        let response_content = state
            .orchestrator
            .chat_json_stream_with_progress(
                ChatJsonStreamInput {
                    request: llm_request.clone(),
                    prompt_name: "object_discovery",
                    truncated_message: "物体档案发现输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“角色/档案发现”输出档位，或减少本次章节上下文后重试",
                    structured_progress: Some(StructuredLlmProgress {
                        spec: StructuredJsonProgressSpec {
                            root_array_key: "objects",
                            item_label: "物体",
                            expected_items: None,
                            milestones: OBJECT_DISCOVERY_FIELDS,
                        },
                        span: OBJECT_DISCOVERY_LLM_SPAN,
                    }),
                    cancel_token: progress.and_then(|progress| progress.cancel_token),
                    cancelled_message: "物体档案发现已停止",
                },
                |current, message| {
                    report_object_discovery_progress(
                        progress,
                        current,
                        message,
                        "物体档案发现已停止",
                    )
                },
            )
            .await?;

        report_object_discovery_progress(
            progress,
            80,
            "模型已返回，正在校验物体发现结果",
            "物体档案发现已停止",
        )?;
        let (parsed_drafts, parsed_output) = parse_and_validate_object_discovery_output(
            &response_content,
            &analyzed_chapters,
            chapters,
            text_anchors,
        )?;
        save_completed_object_discovery_call_checkpoint(
            state,
            workflow_run_id,
            call_fingerprint_json,
            &response_content,
            parsed_output,
        )
        .await?;
        (parsed_drafts, response_content)
    };

    Ok(ObjectDiscoveryCallResult {
        parsed_drafts,
        response_content,
        llm_request,
        analyzed_chapters,
    })
}
