use crate::AppState;
use a4n_api_contract::{
    agent::AgentScope,
    generation_tasks::{GenerationStartRequestMeta, GenerationTaskHandle, GenerationTaskSummary},
    optimize::{
        ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse,
        CreateOptimizationChatSessionInput, OptimizationAgentPermissions,
        OptimizationPatchSetSummary, OptimizationSessionSummary, OptimizationTurnSummary,
        OptimizeSessionInput, SendOptimizationMessageInput, StartOptimizeChatGenerationInput,
    },
};
use a4n_error::AppError;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EnsureContentOptimizationSessionInput {
    pub project_id: i64,
    pub chapter_id: i64,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SendContentOptimizationMessageInput {
    pub session_id: i64,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GenerationTaskSummaryInput {
    pub task_id: String,
    pub attempt: u32,
}

pub(crate) async fn ensure_session(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: EnsureContentOptimizationSessionInput,
) -> Result<OptimizationSessionSummary, AppError> {
    ensure_positive(input.project_id, "projectId")?;
    ensure_positive(input.chapter_id, "chapterId")?;
    super::super::agent_runtime::ensure_project_scope(scope, input.project_id)?;
    super::super::agent_runtime::ensure_chapter_scope(scope, input.chapter_id)?;

    let chapter = crate::actions::chapter_content::list_chapters(state, user_id, input.project_id)
        .await?
        .into_iter()
        .find(|chapter| chapter.id == input.chapter_id)
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "项目 {} 中不存在章节 {}",
                input.project_id, input.chapter_id
            ))
        })?;
    let sessions = crate::actions::optimize::list_optimize_sessions(
        state,
        user_id,
        a4n_api_contract::optimize::ListOptimizeSessionsInput {
            project_id: input.project_id,
            chapter_id: Some(input.chapter_id),
        },
    )
    .await
    .map_err(AppError::InvalidArgument)?;
    if let Some(session) = sessions
        .into_iter()
        .find(|session| session.status != "cancelled")
    {
        return Ok(session);
    }

    let requested_name = input
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("章节 {} Agent 优化", chapter.chapter_number));
    crate::actions::optimize::create_optimization_chat_session(
        state,
        user_id,
        CreateOptimizationChatSessionInput {
            project_id: input.project_id,
            chapter_number: chapter.chapter_number,
            name: requested_name,
            permissions: default_permissions(),
        },
    )
    .await
    .map_err(AppError::InvalidArgument)
}

pub(crate) async fn send_message(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: SendContentOptimizationMessageInput,
    start_request_id: String,
    max_output_tokens: usize,
) -> Result<GenerationTaskHandle, AppError> {
    ensure_positive(input.session_id, "sessionId")?;
    let message = input.message.trim();
    if message.is_empty() {
        return Err(AppError::InvalidArgument("message 不能为空".into()));
    }
    let session = scoped_session(state, user_id, scope, input.session_id).await?;
    let request = StartOptimizeChatGenerationInput {
        meta: GenerationStartRequestMeta {
            start_request_id,
            retry_of_task_id: None,
        },
        request: SendOptimizationMessageInput {
            session_id: session.id,
            message: message.to_string(),
            permissions: session.agent_permissions,
        },
    };
    crate::actions::optimize::start_optimize_chat_generation_with_output_limit(
        state,
        user_id,
        request,
        Some(max_output_tokens),
    )
    .await
    .map_err(|error| AppError::InvalidArgument(error.to_string()))
}

pub(crate) async fn get_session(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: OptimizeSessionInput,
) -> Result<OptimizationSessionSummary, AppError> {
    scoped_session(state, user_id, scope, input.session_id).await
}

pub(crate) async fn list_turns(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: OptimizeSessionInput,
) -> Result<Vec<OptimizationTurnSummary>, AppError> {
    scoped_session(state, user_id, scope, input.session_id).await?;
    crate::actions::optimize::list_optimize_turns(state, user_id, input)
        .await
        .map_err(AppError::InvalidArgument)
}

pub(crate) async fn list_patch_sets(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: OptimizeSessionInput,
) -> Result<Vec<OptimizationPatchSetSummary>, AppError> {
    scoped_session(state, user_id, scope, input.session_id).await?;
    crate::actions::optimize::list_optimize_patch_sets(state, user_id, input)
        .await
        .map_err(AppError::InvalidArgument)
}

pub(crate) async fn apply_patches(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: ApplyOptimizationPatchesInput,
) -> Result<ApplyOptimizationPatchesResponse, AppError> {
    scoped_session(state, user_id, scope, input.session_id).await?;
    crate::actions::optimize::apply_optimization_patches(state, user_id, input)
        .await
        .map_err(AppError::InvalidArgument)
}

pub(crate) async fn generation_task_summary(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: GenerationTaskSummaryInput,
) -> Result<GenerationTaskSummary, AppError> {
    if input.task_id.trim().is_empty() || input.attempt == 0 {
        return Err(AppError::InvalidArgument(
            "taskId 不能为空且 attempt 必须大于 0".into(),
        ));
    }
    let summary = crate::actions::generation_tasks::generation_task_summary(
        state,
        user_id,
        input.task_id.trim(),
        input.attempt,
    )
    .await
    .map_err(|error| AppError::InvalidArgument(error.to_string()))?;

    // The generic generation-task authorization only checks the user's
    // project access. A chapter-bound optimization Agent must also prove that
    // this task belongs to its exact optimization session/chapter.
    if scope.project_id.is_none() && scope.chapter_id.is_none() && scope.resource_kind.is_none() {
        return Ok(summary);
    }
    let project_id = summary.started.project_id.ok_or_else(|| {
        AppError::Forbidden("Agent scope 受限时，generation task 缺少可验证的项目归属".into())
    })?;
    super::super::agent_runtime::ensure_project_scope(scope, project_id)?;

    if scope.chapter_id.is_none() {
        return Ok(summary);
    }
    let target = summary
        .started
        .target
        .as_ref()
        .filter(|target| {
            matches!(
                target.kind.as_str(),
                "optimization_chat" | "optimization_analysis" | "optimization_patch"
            )
        })
        .ok_or_else(|| {
            AppError::Forbidden("章节绑定的 Agent 只能读取可验证的内容优化 generation task".into())
        })?;
    let session_id = target
        .id
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| {
            AppError::InvalidArgument("optimization generation task 的 sessionId 无效".into())
        })?;
    let session = crate::actions::optimize::get_optimize_session(
        state,
        user_id,
        OptimizeSessionInput { session_id },
    )
    .await
    .map_err(AppError::InvalidArgument)?;
    if session.project_id != project_id {
        return Err(AppError::InvalidState {
            current: format!(
                "generation task {} 的项目归属与 optimization session 不一致",
                summary.handle.task_id
            ),
            operation: "校验 Agent generation task scope".into(),
        });
    }
    super::super::agent_runtime::ensure_chapter_scope(scope, session.chapter_id)?;
    Ok(summary)
}

async fn scoped_session(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    session_id: i64,
) -> Result<OptimizationSessionSummary, AppError> {
    ensure_positive(session_id, "sessionId")?;
    let session = crate::actions::optimize::get_optimize_session(
        state,
        user_id,
        OptimizeSessionInput { session_id },
    )
    .await
    .map_err(AppError::InvalidArgument)?;
    super::super::agent_runtime::ensure_project_scope(scope, session.project_id)?;
    super::super::agent_runtime::ensure_chapter_scope(scope, session.chapter_id)?;
    Ok(session)
}

fn default_permissions() -> OptimizationAgentPermissions {
    OptimizationAgentPermissions {
        read_project_detail: true,
        read_blueprint: true,
        read_outlines: true,
        read_character_profiles: true,
        read_foreshadowing: true,
        automatic_writeback: false,
    }
}

fn ensure_positive(value: i64, field: &str) -> Result<(), AppError> {
    if value > 0 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!("{field} 无效")))
    }
}
