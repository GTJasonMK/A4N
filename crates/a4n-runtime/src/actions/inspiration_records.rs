use crate::AppState;
use crate::actions::access;
use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state, model_request_preview,
};
use crate::actions::template_variables::parse_template_variables_app_error_from_ref_with_message;
use a4n_core::error::AppError;
use a4n_core::models::user::message;
use a4n_engine::llm::provider::ChatRequest;
use a4n_services::generation::{
    context_aware_chat_request_trace_json, generation_checkpoint_fingerprint_sha256,
};
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const INSPIRATION_VARIABLES_ERROR: &str = "variables 必须是 JSON 对象";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSummary {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Value>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub chapter_number: Option<i32>,
    pub is_important: bool,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConverseInput {
    pub project_id: i64,
    pub user_input: String,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewInspirationRequestInput {
    pub project_id: i64,
    pub user_input: String,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConverseStreamInput {
    pub project_id: i64,
    pub message: String,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

fn message_summary(m: &message::Model) -> MessageSummary {
    MessageSummary {
        id: m.id,
        conversation_id: m.conversation_id,
        role: m.role.clone(),
        content: m.content.clone(),
        tool_calls: m.tool_calls.clone(),
        prompt_tokens: m.prompt_tokens,
        completion_tokens: m.completion_tokens,
        chapter_number: m.chapter_number,
        is_important: m.is_important,
        created_at: Some(m.created_at.to_string()),
    }
}

pub async fn get_inspiration_history(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    limit: u64,
) -> Result<Vec<MessageSummary>, AppError> {
    validate_history_limit(limit)?;
    access::verify_project_editor(state, user_id, project_id).await?;
    let messages = state.inspiration.history(project_id, limit).await?;
    Ok(messages.iter().map(message_summary).collect())
}

pub async fn converse_inspiration(
    state: &AppState,
    user_id: i64,
    input: ConverseInput,
) -> Result<String, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables_app_error_from_ref_with_message(
        input.variables.as_ref(),
        INSPIRATION_VARIABLES_ERROR,
    )?;
    let request = state
        .inspiration
        .build_inspiration_request(
            input.project_id,
            &input.user_input,
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    let workflow_fingerprint = inspiration_workflow_fingerprint(
        input.project_id,
        "non_stream",
        input.variables.as_ref(),
        &request,
    )?;
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: inspiration_business_key(&workflow_fingerprint)?,
            workflow_kind: "inspiration_conversation".to_string(),
            project_id: Some(input.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("conversation".to_string()),
            scope_id: None,
            chapter_id: None,
            mode: Some("non_stream".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(format!(
                "inspiration_conversation.project_{}.turn",
                input.project_id
            )),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    let result = state
        .inspiration
        .converse(
            input.project_id,
            &input.user_input,
            state.llm.as_ref(),
            state.engine.as_ref(),
            &variables,
            Some(workflow_run.id),
        )
        .await;
    finalize_inspiration_workflow(state, workflow_run.id, &result).await?;
    result
}

pub async fn converse_inspiration_stream<F>(
    state: &AppState,
    user_id: i64,
    input: ConverseStreamInput,
    on_chunk: F,
) -> Result<String, AppError>
where
    F: FnMut(&str, u32),
{
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables_app_error_from_ref_with_message(
        input.variables.as_ref(),
        INSPIRATION_VARIABLES_ERROR,
    )?;
    let request = state
        .inspiration
        .build_inspiration_request(
            input.project_id,
            &input.message,
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    let workflow_fingerprint = inspiration_workflow_fingerprint(
        input.project_id,
        "stream",
        input.variables.as_ref(),
        &request,
    )?;
    let active_task_id = input
        .task_id
        .as_ref()
        .map(|task_id| inspiration_stream_task_id(input.project_id, Some(task_id.clone())))
        .transpose()?;
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: inspiration_business_key(&workflow_fingerprint)?,
            workflow_kind: "inspiration_conversation".to_string(),
            project_id: Some(input.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("conversation".to_string()),
            scope_id: None,
            chapter_id: None,
            mode: Some("stream".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(format!(
                "inspiration_conversation.project_{}.turn",
                input.project_id
            )),
            active_task_id,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    let result = state
        .inspiration
        .converse_stream(a4n_services::inspiration::InspirationConverseStreamInput {
            project_id: input.project_id,
            user_input: &input.message,
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
            workflow_run_id: Some(workflow_run.id),
            on_chunk,
        })
        .await;
    finalize_inspiration_workflow(state, workflow_run.id, &result).await?;
    result
}

fn inspiration_workflow_fingerprint(
    project_id: i64,
    mode: &str,
    variables: Option<&Value>,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "workflow": "inspiration_conversation",
        "schema_version": "inspiration_conversation_workflow_v1",
        "project_id": project_id,
        "mode": mode,
        "variables": variables.cloned().unwrap_or_else(|| json!({})),
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

fn inspiration_business_key(fingerprint: &Value) -> Result<String, AppError> {
    Ok(format!(
        "inspiration_conversation:{}",
        generation_checkpoint_fingerprint_sha256(fingerprint)?
    ))
}

async fn finalize_inspiration_workflow<T>(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<T, AppError>,
) -> Result<(), AppError> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(err) => (WORKFLOW_STATUS_FAILED, true, Some(err.to_string())),
    };
    state
        .workflow_run
        .mark_status(
            workflow_run_id,
            status,
            can_resume,
            error_message.as_deref(),
        )
        .await?;
    Ok(())
}

pub async fn preview_inspiration_request(
    state: &AppState,
    user_id: i64,
    input: PreviewInspirationRequestInput,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables_app_error_from_ref_with_message(
        input.variables.as_ref(),
        INSPIRATION_VARIABLES_ERROR,
    )?;
    let request = state
        .inspiration
        .build_inspiration_request(
            input.project_id,
            &input.user_input,
            state.engine.as_ref(),
            &variables,
        )
        .await?;
    Ok(model_request_preview(
        "inspiration_conversation",
        "灵感对话请求预览",
        vec![
            chat_request_step_for_state(
                state,
                ChatRequestStepInput {
                    key: "inspiration".to_string(),
                    title: "本次灵感对话".to_string(),
                    prompt_name: Some("inspiration".to_string()),
                    state: "ready".to_string(),
                    note: None,
                    request,
                },
            )
            .await
            .map_err(AppError::InvalidArgument)?,
        ],
    ))
}

pub fn inspiration_stream_task_id(
    project_id: i64,
    provided: Option<String>,
) -> Result<String, AppError> {
    let Some(candidate) = provided.map(|id| id.trim().to_string()) else {
        return Ok(format!(
            "inspiration-{project_id}-{}",
            chrono::Utc::now().timestamp_millis()
        ));
    };
    if candidate.is_empty() {
        return Err(AppError::InvalidArgument("task_id 不能为空".to_string()));
    }
    if candidate.len() > 160 {
        return Err(AppError::InvalidArgument(
            "task_id 长度不能超过 160 个字符".to_string(),
        ));
    }
    if !candidate
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'))
    {
        return Err(AppError::InvalidArgument(
            "task_id 只能包含 ASCII 字母、数字、-、_、:、.".to_string(),
        ));
    }
    Ok(candidate)
}

fn validate_history_limit(limit: u64) -> Result<(), AppError> {
    if limit == 0 {
        Err(AppError::InvalidArgument("limit 必须大于 0".to_string()))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn inspiration_stream_task_id_generates_id_when_absent() {
        let task_id = inspiration_stream_task_id(9, None).expect("generated task id");

        assert!(task_id.starts_with("inspiration-9-"));
    }

    #[test]
    fn inspiration_stream_task_id_accepts_trimmed_safe_id() {
        let task_id =
            inspiration_stream_task_id(9, Some("  inspiration:9_alpha-1.test  ".to_string()))
                .expect("provided task id");

        assert_eq!(task_id, "inspiration:9_alpha-1.test");
    }

    #[test]
    fn inspiration_stream_task_id_rejects_invalid_provided_id() {
        assert!(inspiration_stream_task_id(9, Some("   ".to_string())).is_err());
        assert!(inspiration_stream_task_id(9, Some("灵感-9".to_string())).is_err());
        assert!(inspiration_stream_task_id(9, Some("a".repeat(161))).is_err());
    }

    #[test]
    fn history_limit_must_be_positive() {
        assert!(validate_history_limit(1).is_ok());
        let err = validate_history_limit(0).expect_err("zero limit should fail");
        assert!(err.to_string().contains("limit 必须大于 0"));
    }

    #[test]
    fn variables_must_be_object_when_present() {
        let variables = Some(json!(["tone"]));
        let err = parse_template_variables_app_error_from_ref_with_message(
            variables.as_ref(),
            INSPIRATION_VARIABLES_ERROR,
        )
        .expect_err("array variables should fail");
        assert!(err.to_string().contains("variables 必须是 JSON 对象"));
    }

    #[test]
    fn converse_input_rejects_unknown_fields() {
        let err = serde_json::from_value::<ConverseInput>(json!({
            "project_id": 7,
            "user_input": "继续",
            "projectId": 7
        }))
        .expect_err("unknown fields must fail instead of being ignored");

        assert!(err.to_string().contains("unknown field"));
    }
}
