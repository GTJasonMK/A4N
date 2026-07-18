use super::super::{
    MangaPromptSummary, OptimizeMangaPromptInput,
    access::{load_manga_page_prompt, required_trimmed, resolve_manga_prompt_downstream_conflict},
    projections::to_summary,
};
use super::{
    request::build_manga_prompt_optimization_request,
    workflow::finalize_manga_prompt_optimization_workflow,
};
use crate::{
    AppState,
    actions::{
        access, dependency_conflict::delete_downstream_images_if_requested,
        workflow_checkpoint::llm_call_checkpoint_fingerprint_sha256,
    },
};
use a4n_llm::trace::context_aware_chat_request_trace_json;
use a4n_orchestrator::manga::ensure_manga_source_can_write;
use a4n_orchestrator::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

mod call;
mod fingerprint;

pub async fn optimize_manga_prompt(
    state: &AppState,
    user_id: i64,
    input: OptimizeMangaPromptInput,
) -> Result<MangaPromptSummary, String> {
    let instruction = required_trimmed(&input.instruction, "优化要求")?;
    let panel = load_manga_page_prompt(state, input.prompt_id, "整页漫画提示词不存在").await?;
    access::verify_project_or_derived_family_editor(state, user_id, panel.project_id)
        .await
        .map_err(|err| err.to_string())?;
    let source = state
        .orchestrator
        .chapter_manga_source_by_id(panel.chapter_id, panel.project_id, false)
        .await
        .map_err(|err| err.to_string())?;
    ensure_manga_source_can_write(&source, "优化整页漫画提示词").map_err(|err| err.to_string())?;
    let downstream_image_id =
        resolve_manga_prompt_downstream_conflict(&panel, input.downstream_strategy, "优化提示词")?;

    let mut request = build_manga_prompt_optimization_request(state, &panel, instruction)
        .await
        .map_err(|err| err.to_string())?;
    let request_json = context_aware_chat_request_trace_json(&request)
        .map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
    let input_fingerprint_json =
        fingerprint::optimization_input_fingerprint(&panel, instruction, request_json);
    let call_key = format!("optimize_manga_prompt:prompt:{}", panel.id);
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!(
                "manga_prompt_optimization:{}:{}",
                panel.project_id, panel.id
            ),
            workflow_kind: "manga_prompt_optimization".to_string(),
            project_id: Some(panel.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter_manga_prompt".to_string()),
            scope_id: Some(panel.id),
            chapter_id: Some(panel.chapter_id),
            mode: Some(panel.prompt_type.clone()),
            input_fingerprint_json: Some(input_fingerprint_json.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(call_key.clone()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
        .map_err(|err| err.to_string())?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    state
        .orchestrator
        .mark_workflow_current_step(workflow_run_id, &call_key)
        .await
        .map_err(|err| err.to_string())?;

    let result = async {
        let image_prompt = call::run_manga_prompt_optimization_call(
            state,
            request,
            workflow_run_id,
            &call_key,
            &input_fingerprint_json,
        )
        .await?;
        if panel.image_prompt.trim() == image_prompt {
            return Ok(panel.clone());
        }
        let updated = state
            .orchestrator
            .update_manga_page_prompt_image_prompt(panel.clone(), image_prompt.to_string())
            .await
            .map_err(crate::actions::app_error_message)?;
        delete_downstream_images_if_requested(state, user_id, downstream_image_id).await?;
        Ok(updated)
    }
    .await;

    if let Err(status_err) =
        finalize_manga_prompt_optimization_workflow(state, workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!(
                "漫画提示词优化完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result.and_then(|row| to_summary(&row))
}
