use super::super::{
    CheckImplicitUpdateInput, IMPLICIT_UPDATE_PROMPT_NAME, ImplicitUpdateDecisionSummary,
    checkpoint::finalize_implicit_update_review_workflow,
    request::build_check_implicit_update_request,
};
use crate::{AppState, actions::workflow_checkpoint::llm_call_checkpoint_fingerprint_sha256};
use a4n_llm::trace::context_aware_chat_request_trace_json;
use a4n_orchestrator::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use serde_json::json;

mod call;

pub async fn check_implicit_update(
    state: &AppState,
    user_id: i64,
    input: CheckImplicitUpdateInput,
) -> Result<ImplicitUpdateDecisionSummary, String> {
    let attribute_key = input.attribute_key.trim();
    if attribute_key.is_empty() {
        return Err("隐性属性 key 不能为空".to_string());
    }
    let mut request_context = build_check_implicit_update_request(state, user_id, &input).await?;
    let request_json = context_aware_chat_request_trace_json(&request_context.request)
        .map_err(|error| format!("LLM 请求序列化失败: {error}"))?;
    let call_key = format!(
        "implicit_update_review:{}:{attribute_key}",
        input.profile_id
    );
    let input_fingerprint_json = json!({
        "prompt_names": [IMPLICIT_UPDATE_PROMPT_NAME],
        "request": request_json,
        "input_variables": {
            "profile_id": input.profile_id,
            "attribute_key": attribute_key,
            "threshold": input.threshold,
            "window_chapters": input.window_chapters,
            "evidence_limit": input.evidence_limit}});
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!(
                "implicit_update_review:{}:{attribute_key}",
                input.profile_id
            ),
            workflow_kind: "implicit_update_review".to_string(),
            project_id: Some(request_context.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: Some(input.profile_id),
            chapter_id: None,
            mode: Some(attribute_key.to_string()),
            input_fingerprint_json: Some(input_fingerprint_json.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(call_key.clone()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|error| error.to_string())?;
    let workflow_run_id = workflow_run.id;
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
        .map_err(|error| error.to_string())?;
    if let Some(trace_context) = request_context.request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    state
        .orchestrator
        .mark_workflow_current_step(workflow_run_id, &call_key)
        .await
        .map_err(|error| error.to_string())?;

    let result = call::run_manual_implicit_review_call(
        state,
        request_context,
        call::ManualImplicitReviewCallInput {
            profile_id: input.profile_id,
            attribute_key: attribute_key.to_string(),
            workflow_run_id,
            call_key,
            input_fingerprint_json,
        },
    )
    .await;

    if let Err(status_error) =
        finalize_implicit_update_review_workflow(state, workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!(
                "隐性属性复审完成，但工作流状态写入失败: {status_error}"
            )),
            Err(error) => Err(format!("{error}；工作流失败状态写入失败: {status_error}")),
        };
    }
    result
}
