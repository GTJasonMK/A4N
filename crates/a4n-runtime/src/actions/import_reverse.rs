use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state, model_request_preview,
};
use crate::actions::template_variables::parse_template_variables_app_error_from_ref_with_message;
use crate::{AppState, actions::access};
use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::state::status::ProjectStatus;
use a4n_engine::llm::provider::ChatRequest;
use a4n_services::import::reverse::{
    ReverseContentRequest, ReverseContextKind, apply_reverse_blueprint_data,
    apply_reverse_outline_data, apply_reverse_part_outline_data, resolve_reverse_content_sample,
};
use a4n_services::source::source_content_sha256;
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use sea_orm::TransactionTrait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

const REVERSE_VARIABLES_ERROR: &str = "variables 必须是 JSON 对象或 null";

#[derive(Debug, Clone, Deserialize)]
pub struct ReverseEngineerInput {
    pub project_id: i64,
    #[serde(default)]
    pub content_sample: String,
    #[serde(default)]
    pub auto_context: bool,
    #[serde(default)]
    pub context_limit: Option<usize>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Copy)]
pub struct ReverseEngineerTaskScope {
    pub project_id: i64,
    pub total: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum ReverseEngineeringTarget {
    Blueprint,
    Outline,
    PartOutline,
}

impl ReverseEngineeringTarget {
    fn context_kind(self) -> ReverseContextKind {
        match self {
            Self::Blueprint => ReverseContextKind::Blueprint,
            Self::Outline => ReverseContextKind::Outline,
            Self::PartOutline => ReverseContextKind::PartOutline,
        }
    }

    pub fn prompt_name(self) -> &'static str {
        match self {
            Self::Blueprint => "reverse_blueprint",
            Self::Outline => "reverse_outline",
            Self::PartOutline => "reverse_part_outline",
        }
    }

    pub(crate) fn user_instruction(self) -> &'static str {
        match self {
            Self::Blueprint => "请从以下内容反推蓝图（世界观/角色/剧情大纲/主题，输出 JSON）：",
            Self::Outline => "请从以下章节内容反推章节大纲（输出 JSON）：",
            Self::PartOutline => "请从以下内容反推分部大纲结构（输出 JSON）：",
        }
    }

    pub(crate) fn max_tokens_profile(self) -> &'static str {
        match self {
            Self::Blueprint => llm_token_profiles::BLUEPRINT,
            Self::Outline | Self::PartOutline => llm_token_profiles::OUTLINE,
        }
    }

    fn preview_title(self) -> &'static str {
        match self {
            Self::Blueprint => "反推蓝图 LLM 请求预览",
            Self::Outline => "反推章节大纲 LLM 请求预览",
            Self::PartOutline => "反推分部大纲 LLM 请求预览",
        }
    }

    fn preview_step_title(self) -> &'static str {
        match self {
            Self::Blueprint => "反推蓝图 LLM 请求",
            Self::Outline => "反推章节大纲 LLM 请求",
            Self::PartOutline => "反推分部大纲 LLM 请求",
        }
    }

    pub fn initial_message(self, apply: bool) -> &'static str {
        match (self, apply) {
            (Self::Blueprint, false) => "正在准备反推蓝图",
            (Self::Blueprint, true) => "正在准备反推蓝图并应用",
            (Self::Outline, false) => "正在准备反推章节大纲",
            (Self::Outline, true) => "正在准备反推章节大纲并应用",
            (Self::PartOutline, false) => "正在准备反推分部大纲",
            (Self::PartOutline, true) => "正在准备反推分部大纲并应用",
        }
    }

    pub fn success_message(self, apply: bool) -> &'static str {
        match (self, apply) {
            (Self::Blueprint, false) => "反推蓝图完成",
            (Self::Blueprint, true) => "反推蓝图结果已应用",
            (Self::Outline, false) => "反推章节大纲完成",
            (Self::Outline, true) => "反推章节大纲结果已应用",
            (Self::PartOutline, false) => "反推分部大纲完成",
            (Self::PartOutline, true) => "反推分部大纲结果已应用",
        }
    }

    pub fn cancelled_message(self) -> &'static str {
        match self {
            Self::Blueprint => "反推蓝图已停止",
            Self::Outline => "反推章节大纲已停止",
            Self::PartOutline => "反推分部大纲已停止",
        }
    }

    pub fn failure_event(self) -> &'static str {
        match self {
            Self::Blueprint => "reverse_blueprint_failed",
            Self::Outline => "reverse_outline_failed",
            Self::PartOutline => "reverse_part_outline_failed",
        }
    }

    fn preparing_context_message(self) -> &'static str {
        match self {
            Self::Blueprint => "正在准备反推蓝图上下文",
            Self::Outline => "正在准备反推章节大纲上下文",
            Self::PartOutline => "正在准备反推分部大纲上下文",
        }
    }

    fn calling_model_message(self) -> &'static str {
        match self {
            Self::Blueprint => "正在调用模型反推蓝图",
            Self::Outline => "正在调用模型反推章节大纲",
            Self::PartOutline => "正在调用模型反推分部大纲",
        }
    }

    fn organizing_message(self) -> &'static str {
        match self {
            Self::Blueprint => "模型已返回，正在整理反推蓝图",
            Self::Outline => "模型已返回，正在整理章节大纲",
            Self::PartOutline => "模型已返回，正在整理分部大纲",
        }
    }

    fn applying_message(self) -> &'static str {
        match self {
            Self::Blueprint => "正在应用反推蓝图结果",
            Self::Outline => "正在应用反推章节大纲结果",
            Self::PartOutline => "正在应用反推分部大纲结果",
        }
    }

    fn applied_message(self) -> &'static str {
        match self {
            Self::Blueprint => "反推蓝图结果已应用",
            Self::Outline => "反推章节大纲结果已应用",
            Self::PartOutline => "反推分部大纲结果已应用",
        }
    }

    fn applied_status(self) -> ProjectStatus {
        match self {
            Self::Blueprint => ProjectStatus::BlueprintReady,
            Self::Outline => ProjectStatus::ChapterOutlinesReady,
            Self::PartOutline => ProjectStatus::PartOutlinesReady,
        }
    }
}

pub type ReverseProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

async fn start_reverse_workflow_run(
    state: &AppState,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
    apply: bool,
    content_sample: &str,
    variables: &HashMap<String, Value>,
) -> Result<i64, AppError> {
    let mode = if apply { "apply" } else { "reverse" };
    let run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!(
                "import_reverse_engineer:project:{}:{}:{mode}",
                input.project_id,
                target.prompt_name()
            ),
            workflow_kind: "import_reverse_engineer".to_string(),
            project_id: Some(input.project_id),
            root_project_id: Some(input.project_id),
            source_snapshot_id: None,
            scope_table: Some("project".to_string()),
            scope_id: Some(input.project_id),
            chapter_id: None,
            mode: Some(mode.to_string()),
            input_fingerprint_json: Some(json!({
                "schema_version": 1,
                "workflow": "import_reverse_engineer",
                "target": target.prompt_name(),
                "mode": mode,
                "project_id": input.project_id,
                "content_sample_sha256": source_content_sha256(content_sample),
                "auto_context": input.auto_context,
                "context_limit": input.context_limit,
                "variables": variables,
            })),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(target.prompt_name().to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    Ok(run.id)
}

async fn complete_reverse_workflow_run(
    state: &AppState,
    workflow_run_id: i64,
) -> Result<(), AppError> {
    state
        .workflow_run
        .mark_status(workflow_run_id, WORKFLOW_STATUS_COMPLETED, false, None)
        .await?;
    Ok(())
}

async fn fail_reverse_workflow_run(state: &AppState, workflow_run_id: i64, err: &AppError) {
    let _ = state
        .workflow_run
        .mark_status(
            workflow_run_id,
            WORKFLOW_STATUS_FAILED,
            true,
            Some(&err.to_string()),
        )
        .await;
}

pub async fn reverse_engineer_task_scope(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
    apply: bool,
) -> Result<ReverseEngineerTaskScope, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    Ok(ReverseEngineerTaskScope {
        project_id: input.project_id,
        total: if apply { 4 } else { 3 },
    })
}

async fn build_reverse_request_for_user(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
) -> Result<ChatRequest, AppError> {
    reverse_engineer_task_scope(state, user_id, input, false).await?;
    let variables = parse_template_variables_app_error_from_ref_with_message(
        input.variables.as_ref(),
        REVERSE_VARIABLES_ERROR,
    )?;
    build_reverse_request(state, input, target, &variables).await
}

pub async fn preview_reverse_engineer_request(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
) -> Result<ModelRequestPreview, AppError> {
    let request = build_reverse_request_for_user(state, user_id, input, target).await?;
    let step = chat_request_step_for_state(
        state,
        ChatRequestStepInput {
            key: target.prompt_name().to_string(),
            title: target.preview_step_title().to_string(),
            prompt_name: Some(target.prompt_name().to_string()),
            state: "ready".to_string(),
            note: None,
            request,
        },
    )
    .await
    .map_err(AppError::InvalidArgument)?;
    Ok(model_request_preview(
        target.prompt_name(),
        target.preview_title(),
        vec![step],
    ))
}

async fn build_reverse_request(
    state: &AppState,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
    variables: &HashMap<String, Value>,
) -> Result<ChatRequest, AppError> {
    let content_sample = reverse_content_sample(state, input, target).await?;
    build_reverse_request_from_sample(state, input.project_id, target, &content_sample, variables)
}

pub async fn reverse_engineer(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
    progress: ReverseProgressReporter<'_>,
) -> Result<Value, AppError> {
    run_reverse_engineer(state, user_id, input, target, false, progress).await
}

pub async fn reverse_engineer_and_apply(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
    progress: ReverseProgressReporter<'_>,
) -> Result<Value, AppError> {
    run_reverse_engineer(state, user_id, input, target, true, progress).await
}

async fn run_reverse_engineer(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
    apply: bool,
    progress: ReverseProgressReporter<'_>,
) -> Result<Value, AppError> {
    let scope = reverse_engineer_task_scope(state, user_id, input, apply).await?;
    progress(0, scope.total, target.preparing_context_message())?;
    let variables = parse_template_variables_app_error_from_ref_with_message(
        input.variables.as_ref(),
        REVERSE_VARIABLES_ERROR,
    )?;
    let content_sample = reverse_content_sample(state, input, target).await?;
    let workflow_run_id =
        start_reverse_workflow_run(state, input, target, apply, &content_sample, &variables)
            .await?;

    progress(1, scope.total, target.calling_model_message())?;
    let result = match reverse_engineer_with_target(
        state,
        input.project_id,
        target,
        &content_sample,
        &variables,
        Some(workflow_run_id),
    )
    .await
    {
        Ok(result) => result,
        Err(err) => {
            fail_reverse_workflow_run(state, workflow_run_id, &err).await;
            return Err(err);
        }
    };
    progress(2, scope.total, target.organizing_message())?;
    if !apply {
        complete_reverse_workflow_run(state, workflow_run_id).await?;
        return Ok(result);
    }

    progress(3, scope.total, target.applying_message())?;
    let applied =
        match apply_reverse_engineering_result(state, input.project_id, target, &result).await {
            Ok(applied) => applied,
            Err(err) => {
                fail_reverse_workflow_run(state, workflow_run_id, &err).await;
                return Err(err);
            }
        };
    progress(4, scope.total, target.applied_message())?;
    complete_reverse_workflow_run(state, workflow_run_id).await?;
    Ok(json!({
        "result": result,
        "applied": applied,
    }))
}

async fn reverse_engineer_with_target(
    state: &AppState,
    project_id: i64,
    target: ReverseEngineeringTarget,
    content_sample: &str,
    variables: &HashMap<String, Value>,
    workflow_run_id: Option<i64>,
) -> Result<Value, AppError> {
    let request =
        build_reverse_request_from_sample(state, project_id, target, content_sample, variables)?;
    state
        .import
        .reverse_engineer_request(
            target.prompt_name(),
            request,
            state.llm.as_ref(),
            workflow_run_id,
        )
        .await
}

pub(crate) fn build_reverse_request_from_sample(
    state: &AppState,
    project_id: i64,
    target: ReverseEngineeringTarget,
    content_sample: &str,
    variables: &HashMap<String, Value>,
) -> Result<ChatRequest, AppError> {
    state
        .import
        .build_reverse_engineer_request(a4n_services::import::ReverseEngineerRequestInput {
            project_id,
            prompt_name: target.prompt_name(),
            user_prefix: target.user_instruction(),
            content_sample,
            prompt_engine: state.engine.as_ref(),
            variables,
            max_tokens_profile: target.max_tokens_profile(),
        })
}

async fn apply_reverse_engineering_result(
    state: &AppState,
    project_id: i64,
    target: ReverseEngineeringTarget,
    result: &Value,
) -> Result<Value, AppError> {
    let txn = state.db.begin().await.map_err(AppError::Database)?;
    let applied = match target {
        ReverseEngineeringTarget::Blueprint => {
            apply_reverse_blueprint_data(&txn, project_id, result)
                .await
                .map_err(AppError::InvalidArgument)?
        }
        ReverseEngineeringTarget::Outline => apply_reverse_outline_data(&txn, project_id, result)
            .await
            .map_err(AppError::InvalidArgument)?,
        ReverseEngineeringTarget::PartOutline => {
            apply_reverse_part_outline_data(&txn, project_id, result)
                .await
                .map_err(AppError::InvalidArgument)?
        }
    };
    txn.commit().await.map_err(AppError::Database)?;
    crate::workflow::advance_project_status_if_allowed(state, project_id, target.applied_status())
        .await?;
    Ok(applied)
}

async fn reverse_content_sample(
    state: &AppState,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
) -> Result<String, AppError> {
    resolve_reverse_content_sample(
        state.db.as_ref(),
        ReverseContentRequest {
            project_id: input.project_id,
            manual_sample: &input.content_sample,
            auto_context: input.auto_context,
            context_limit: input.context_limit,
        },
        target.context_kind(),
    )
    .await
    .map_err(AppError::InvalidArgument)
}
