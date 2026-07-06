use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use crate::{AppState, actions::access};
use a4n_core::models::ai::{
    optimization_patch, optimization_patch_set, optimization_session, optimization_turn,
    optimization_turn_event,
};
use a4n_core::models::chapter::chapter;
use a4n_services::optimize::{
    OptimizationPatch, OptimizeAgentPermissions as OptimizationAgentPermissions, OptimizeLlmStream,
    OptimizeProgress, optimization_content_hash,
};
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSessionSummary {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub name: Option<String>,
    pub status: String,
    pub issues: Option<serde_json::Value>,
    pub agent_permissions: Option<serde_json::Value>,
    pub agent_instruction: Option<String>,
    pub fixes_applied: i32,
    pub summary: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationTurnEventSummary {
    pub id: i64,
    pub turn_id: i64,
    pub seq: i32,
    pub kind: String,
    pub status: String,
    pub public_text: String,
    pub payload: Option<serde_json::Value>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationTurnSummary {
    pub id: i64,
    pub session_id: i64,
    pub user_text: String,
    pub status: String,
    pub permissions_snapshot: Option<serde_json::Value>,
    pub final_reply: Option<String>,
    pub error: Option<String>,
    pub events: Vec<OptimizationTurnEventSummary>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationPatchSetSummary {
    pub id: i64,
    pub turn_id: i64,
    pub session_id: i64,
    pub chapter_id: i64,
    pub base_content_hash: String,
    pub status: String,
    pub summary: String,
    pub patches: Vec<OptimizationPatch>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzeChapterInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub name: String,
    pub mode: String,
    pub scope: Option<String>,
    pub permissions: Option<OptimizationAgentPermissions>,
    pub agent_instruction: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateOptimizationChatSessionInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub name: String,
    pub permissions: Option<OptimizationAgentPermissions>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PrepareOptimizationAnalysisSessionInput {
    pub session_id: i64,
    pub mode: String,
    pub scope: Option<String>,
    pub permissions: Option<OptimizationAgentPermissions>,
    pub agent_instruction: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProposeOptimizationPatchesInput {
    pub session_id: i64,
    pub issue_ids: Option<Vec<String>>,
    pub agent_instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposeOptimizationPatchesResponse {
    pub session_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub base_content_hash: String,
    pub summary: String,
    pub patches: Vec<OptimizationPatch>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplyOptimizationPatchesInput {
    pub session_id: i64,
    pub base_content_hash: String,
    pub patches: Vec<OptimizationPatch>,
    pub confirmed_apply: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyOptimizationPatchesResponse {
    pub session_id: i64,
    pub chapter_id: i64,
    pub applied_count: usize,
    pub word_count: i32,
    pub content_hash: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SendOptimizationMessageInput {
    pub session_id: i64,
    pub message: String,
    pub permissions: Option<OptimizationAgentPermissions>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SendOptimizationMessageResponse {
    pub session_id: i64,
    pub turn: OptimizationTurnSummary,
}

#[derive(Debug, Clone, Copy)]
pub struct OptimizationTaskScope {
    pub project_id: i64,
    pub total: u32,
}

pub async fn execute_optimization_analysis_session_task_scope(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<OptimizationTaskScope, String> {
    optimize_session_task_scope(state, user_id, session_id, 5).await
}

pub async fn propose_optimization_patches_task_scope(
    state: &AppState,
    user_id: i64,
    input: &ProposeOptimizationPatchesInput,
) -> Result<OptimizationTaskScope, String> {
    optimize_session_task_scope(state, user_id, input.session_id, 4).await
}

pub async fn apply_optimization_patches_task_scope(
    state: &AppState,
    user_id: i64,
    input: &ApplyOptimizationPatchesInput,
) -> Result<OptimizationTaskScope, String> {
    optimize_session_task_scope(state, user_id, input.session_id, 3).await
}

pub async fn send_optimization_message_task_scope(
    state: &AppState,
    user_id: i64,
    input: &SendOptimizationMessageInput,
) -> Result<OptimizationTaskScope, String> {
    optimize_session_task_scope(state, user_id, input.session_id, 4).await
}

pub async fn list_optimize_sessions(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: Option<i64>,
) -> Result<Vec<OptimizationSessionSummary>, String> {
    verify_optimize_project_access(state, user_id, project_id).await?;
    let sessions = state
        .optimize
        .list_sessions(project_id, chapter_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(sessions.iter().map(to_session_summary).collect())
}

pub async fn get_optimize_session(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<OptimizationSessionSummary, String> {
    verify_optimize_session_access(state, user_id, session_id).await?;
    let session = state
        .optimize
        .get_session(session_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(to_session_summary(&session))
}

pub async fn list_optimize_turns(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<Vec<OptimizationTurnSummary>, String> {
    verify_optimize_session_access(state, user_id, session_id).await?;
    let turns = state
        .optimize
        .list_turns(session_id)
        .await
        .map_err(|e| e.to_string())?;
    let mut summaries = Vec::with_capacity(turns.len());
    for turn in turns {
        let events = state
            .optimize
            .list_turn_events(turn.id)
            .await
            .map_err(|e| e.to_string())?;
        summaries.push(to_turn_summary(&turn, events));
    }
    Ok(summaries)
}

pub async fn list_optimize_patch_sets(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<Vec<OptimizationPatchSetSummary>, String> {
    verify_optimize_session_access(state, user_id, session_id).await?;
    let patch_sets = state
        .optimize
        .list_patch_sets(session_id)
        .await
        .map_err(|e| e.to_string())?;
    let mut summaries = Vec::with_capacity(patch_sets.len());
    for patch_set in patch_sets {
        let patches = state
            .optimize
            .list_patch_set_patches(patch_set.id)
            .await
            .map_err(|e| e.to_string())?;
        summaries.push(to_patch_set_summary(&patch_set, patches));
    }
    Ok(summaries)
}

pub async fn reject_pending_optimize_patch_sets(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<(), String> {
    verify_optimize_session_access(state, user_id, session_id).await?;
    state
        .optimize
        .reject_pending_patch_sets(session_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn delete_optimize_session(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<(), String> {
    verify_optimize_session_access(state, user_id, session_id).await?;
    state
        .optimize
        .delete_session(session_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn record_optimize_agent_event(
    state: &AppState,
    user_id: i64,
    session_id: i64,
    content: String,
) -> Result<(), String> {
    verify_optimize_session_access(state, user_id, session_id).await?;
    let content = content.trim();
    if content.is_empty() {
        return Err("Agent 事件内容不能为空".to_string());
    }
    state
        .optimize
        .record_agent_event(session_id, content)
        .await
        .map_err(|e| e.to_string())
}

pub async fn create_optimization_chat_session(
    state: &AppState,
    user_id: i64,
    input: CreateOptimizationChatSessionInput,
) -> Result<OptimizationSessionSummary, String> {
    verify_optimize_project_access(state, user_id, input.project_id).await?;
    let chapter = find_chapter_for_optimize(state, input.project_id, input.chapter_number).await?;
    let permissions = input.permissions.unwrap_or_default();
    let mut variables = std::collections::HashMap::new();
    variables.insert("agent_permissions".to_string(), permissions.as_value());

    let session = state
        .optimize
        .start_chat_session(input.project_id, chapter.id, &input.name, &variables)
        .await
        .map_err(|e| e.to_string())?;
    Ok(to_session_summary(&session))
}

pub async fn create_optimization_analysis_session(
    state: &AppState,
    user_id: i64,
    input: AnalyzeChapterInput,
) -> Result<OptimizationSessionSummary, String> {
    verify_optimize_project_access(state, user_id, input.project_id).await?;
    let chapter = find_chapter_for_optimize(state, input.project_id, input.chapter_number).await?;
    let variables = build_analysis_variables(
        &input.mode,
        input.scope.as_deref(),
        input.permissions.as_ref(),
        input.agent_instruction.as_deref(),
    )?;
    let session = state
        .optimize
        .create_analysis_session(
            input.project_id,
            chapter.id,
            &input.name,
            &chapter.content,
            state.engine.as_ref(),
            &variables,
        )
        .await
        .map_err(|e| e.to_string())?;
    Ok(to_session_summary(&session))
}

pub async fn prepare_optimization_analysis_session(
    state: &AppState,
    user_id: i64,
    input: PrepareOptimizationAnalysisSessionInput,
) -> Result<OptimizationSessionSummary, String> {
    let session_id = input.session_id;
    let result = async {
        verify_optimize_session_access(state, user_id, input.session_id).await?;
        let current_session = state
            .optimize
            .get_session(input.session_id)
            .await
            .map_err(|e| e.to_string())?;
        let chapter = find_chapter_by_id(state, current_session.chapter_id).await?;
        let variables = build_analysis_variables(
            &input.mode,
            input.scope.as_deref(),
            input.permissions.as_ref(),
            input.agent_instruction.as_deref(),
        )?;
        let session = state
            .optimize
            .prepare_analysis_session(
                session_id,
                &chapter.content,
                state.engine.as_ref(),
                &variables,
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(to_session_summary(&session))
    }
    .await;
    record_optimize_command_result(state, session_id, "准备内容优化分析", result).await
}

pub async fn execute_optimization_analysis_session(
    state: &AppState,
    user_id: i64,
    session_id: i64,
    progress: Option<OptimizeProgress<'_>>,
    stream: Option<OptimizeLlmStream<'_>>,
) -> Result<OptimizationSessionSummary, String> {
    execute_optimization_analysis_session_task_scope(state, user_id, session_id).await?;
    let workflow_run_id = start_optimize_workflow_run(
        state,
        session_id,
        "content_optimization_analysis",
        "analysis",
    )
    .await?;
    let result = state
        .optimize
        .run_analysis_session(
            session_id,
            state.llm.as_ref(),
            workflow_run_id,
            progress,
            stream,
        )
        .await
        .map(|session| to_session_summary(&session))
        .map_err(|e| e.to_string());
    let result =
        finalize_optimize_workflow_run(state, workflow_run_id, "内容优化分析", result).await;
    record_optimize_command_result(state, session_id, "内容优化分析", result).await
}

pub async fn preview_create_optimization_analysis_session_request(
    state: &AppState,
    user_id: i64,
    input: AnalyzeChapterInput,
) -> Result<ModelRequestPreview, String> {
    verify_optimize_project_access(state, user_id, input.project_id).await?;
    let chapter = find_chapter_for_optimize(state, input.project_id, input.chapter_number).await?;
    let variables = build_analysis_variables(
        &input.mode,
        input.scope.as_deref(),
        input.permissions.as_ref(),
        input.agent_instruction.as_deref(),
    )?;
    let request = state
        .optimize
        .build_analysis_preview_request(
            input.project_id,
            chapter.id,
            &chapter.content,
            state.engine.as_ref(),
            &variables,
        )
        .await
        .map_err(|e| e.to_string())?;
    optimization_analysis_request_preview(state, request).await
}

pub async fn preview_prepare_optimization_analysis_session_request(
    state: &AppState,
    user_id: i64,
    input: PrepareOptimizationAnalysisSessionInput,
) -> Result<ModelRequestPreview, String> {
    verify_optimize_session_access(state, user_id, input.session_id).await?;
    let current_session = state
        .optimize
        .get_session(input.session_id)
        .await
        .map_err(|e| e.to_string())?;
    let chapter = find_chapter_by_id(state, current_session.chapter_id).await?;
    let variables = build_analysis_variables(
        &input.mode,
        input.scope.as_deref(),
        input.permissions.as_ref(),
        input.agent_instruction.as_deref(),
    )?;
    let request = state
        .optimize
        .build_analysis_preview_request(
            current_session.project_id,
            current_session.chapter_id,
            &chapter.content,
            state.engine.as_ref(),
            &variables,
        )
        .await
        .map_err(|e| e.to_string())?;
    optimization_analysis_request_preview(state, request).await
}

pub async fn propose_optimization_patches(
    state: &AppState,
    user_id: i64,
    input: ProposeOptimizationPatchesInput,
    mut progress: Option<OptimizeProgress<'_>>,
    stream: Option<OptimizeLlmStream<'_>>,
) -> Result<ProposeOptimizationPatchesResponse, String> {
    let session_id = input.session_id;
    propose_optimization_patches_task_scope(state, user_id, &input).await?;
    if let Some(progress) = progress.as_mut() {
        progress(0, 4, "正在准备局部修复补丁").map_err(|e| e.to_string())?;
    }
    let workflow_run_id =
        start_optimize_workflow_run(state, session_id, "content_optimization_patch", "patch")
            .await?;
    let result = state
        .optimize
        .propose_patches(a4n_services::optimize::OptimizeProposePatchesInput {
            session_id: input.session_id,
            issue_ids: input.issue_ids,
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            agent_instruction_override: input.agent_instruction.as_deref(),
            workflow_run_id: workflow_run_id,
            progress: progress,
            stream: stream,
        })
        .await
        .map(|proposal| ProposeOptimizationPatchesResponse {
            session_id: proposal.session_id,
            project_id: proposal.project_id,
            chapter_id: proposal.chapter_id,
            base_content_hash: proposal.base_content_hash,
            summary: proposal.summary,
            patches: proposal.patches,
        })
        .map_err(|e| e.to_string());
    let result =
        finalize_optimize_workflow_run(state, workflow_run_id, "局部补丁生成", result).await;
    record_optimize_command_result(state, session_id, "局部补丁生成", result).await
}

pub async fn preview_propose_optimization_patches_request(
    state: &AppState,
    user_id: i64,
    input: ProposeOptimizationPatchesInput,
) -> Result<ModelRequestPreview, String> {
    verify_optimize_session_access(state, user_id, input.session_id).await?;
    let request = state
        .optimize
        .build_propose_patches_preview_request(
            input.session_id,
            input.issue_ids,
            state.engine.as_ref(),
            input.agent_instruction.as_deref(),
        )
        .await
        .map_err(|e| e.to_string())?;
    chat_status_preview_for_state(
        state,
        "optimization_patch_generation",
        "内容优化局部补丁请求预览",
        vec![chat_step(
            "optimization_patch_generation",
            "局部补丁生成 LLM 请求",
            Some("content_optimization_patch"),
            "ready",
            None,
            request,
        )],
        "llm_chat",
    )
    .await
}

pub async fn apply_optimization_patches(
    state: &AppState,
    user_id: i64,
    input: ApplyOptimizationPatchesInput,
    mut progress: Option<OptimizeProgress<'_>>,
) -> Result<ApplyOptimizationPatchesResponse, String> {
    let session_id = input.session_id;
    apply_optimization_patches_task_scope(state, user_id, &input).await?;
    let result = async {
        if let Some(progress) = progress.as_mut() {
            progress(0, 3, "正在校验局部补丁").map_err(|e| e.to_string())?;
        }
        if !input.confirmed_apply {
            return Err("应用局部补丁必须经过用户确认".to_string());
        }
        if input.patches.is_empty() {
            return Err("应用补丁列表不能为空".to_string());
        }
        if let Some(progress) = progress.as_mut() {
            progress(1, 3, "正在写入局部补丁结果").map_err(|e| e.to_string())?;
        }
        let written = state
            .optimize
            .write_patches_to_session_chapter(
                input.session_id,
                &input.base_content_hash,
                &input.patches,
            )
            .await
            .map_err(|e| e.to_string())?;
        crate::workflow::postprocess_chapter_after_content_change(state, written.chapter_id)
            .await
            .map_err(|e| e.to_string())?;
        let updated = state
            .chapter
            .get(written.chapter_id)
            .await
            .map_err(|e| format!("应用局部补丁后刷新章节失败: {e}"))?;
        let content_hash = optimization_content_hash(&updated.content);
        state
            .optimize
            .record_patches_applied(
                input.session_id,
                updated.id,
                updated.word_count,
                &content_hash,
                &input.patches,
            )
            .await
            .map_err(|e| e.to_string())?;
        state
            .optimize
            .mark_matching_patch_sets_applied(input.session_id, &input.base_content_hash)
            .await
            .map_err(|e| e.to_string())?;
        if let Some(progress) = progress.as_mut() {
            progress(3, 3, "局部补丁已应用").map_err(|e| e.to_string())?;
        }

        Ok(ApplyOptimizationPatchesResponse {
            session_id: input.session_id,
            chapter_id: updated.id,
            applied_count: input.patches.len(),
            word_count: updated.word_count,
            content_hash,
        })
    }
    .await;
    record_optimize_command_result(state, session_id, "应用局部补丁", result).await
}

pub async fn send_optimization_message(
    state: &AppState,
    user_id: i64,
    input: SendOptimizationMessageInput,
    mut progress: Option<OptimizeProgress<'_>>,
    stream: Option<OptimizeLlmStream<'_>>,
) -> Result<SendOptimizationMessageResponse, String> {
    let session_id = input.session_id;
    send_optimization_message_task_scope(state, user_id, &input).await?;
    if let Some(progress) = progress.as_mut() {
        progress(0, 4, "正在准备内容优化 Agent 对话").map_err(|e| e.to_string())?;
    }
    let workflow_run_id =
        start_optimize_workflow_run(state, session_id, "content_optimization_chat", "chat").await?;
    let result = match state
        .optimize
        .send_user_message_with_permissions(a4n_services::optimize::OptimizeSendUserMessageInput {
            session_id: input.session_id,
            message: &input.message,
            permissions_override: input.permissions.as_ref(),
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            workflow_run_id: workflow_run_id,
            progress: progress,
            stream: stream,
        })
        .await
    {
        Ok(turn) => {
            let events = state
                .optimize
                .list_turn_events(turn.id)
                .await
                .map_err(|err| err.to_string());
            events.map(|events| SendOptimizationMessageResponse {
                session_id: input.session_id,
                turn: to_turn_summary(&turn, events),
            })
        }
        Err(err) => Err(err.to_string()),
    };
    let result =
        finalize_optimize_workflow_run(state, workflow_run_id, "内容优化对话", result).await;
    record_optimize_command_result(state, session_id, "内容优化对话", result).await
}

pub async fn preview_send_optimization_message_request(
    state: &AppState,
    user_id: i64,
    input: SendOptimizationMessageInput,
) -> Result<ModelRequestPreview, String> {
    verify_optimize_session_access(state, user_id, input.session_id).await?;
    let request = state
        .optimize
        .build_send_user_message_preview_request(
            input.session_id,
            &input.message,
            input.permissions.as_ref(),
            state.engine.as_ref(),
        )
        .await
        .map_err(|e| e.to_string())?;
    chat_status_preview_for_state(
        state,
        "optimization_chat_agent_loop",
        "内容优化对话 Agent 请求预览",
        vec![
            chat_step(
                "optimization_chat_step_1",
                "内容优化对话 Agent Loop 第 1 步",
                Some("content_optimization_chat"),
                "ready",
                None,
                request,
            ),
            blocked_step(
                "optimization_chat_followup",
                "内容优化对话 Agent Loop 后续步骤",
                Some("content_optimization_chat"),
                "后续请求依赖上一轮模型决策、工具观察和补丁结果",
            ),
        ],
        "llm_chat",
    )
    .await
}

async fn session_project_id(state: &AppState, session_id: i64) -> Result<i64, String> {
    Ok(state
        .optimize
        .get_session(session_id)
        .await
        .map_err(|e| e.to_string())?
        .project_id)
}

async fn verify_optimize_session_access(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<i64, String> {
    let project_id = session_project_id(state, session_id).await?;
    verify_optimize_project_access(state, user_id, project_id).await?;
    Ok(project_id)
}

async fn optimize_session_task_scope(
    state: &AppState,
    user_id: i64,
    session_id: i64,
    total: u32,
) -> Result<OptimizationTaskScope, String> {
    let project_id = verify_optimize_session_access(state, user_id, session_id).await?;
    Ok(OptimizationTaskScope { project_id, total })
}

async fn start_optimize_workflow_run(
    state: &AppState,
    session_id: i64,
    workflow_kind: &str,
    mode: &str,
) -> Result<i64, String> {
    let session = state
        .optimize
        .get_session(session_id)
        .await
        .map_err(|e| e.to_string())?;
    let current_step_key = format!("optimization_session:{session_id}:{mode}");
    let run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!("{workflow_kind}:{session_id}:{mode}"),
            workflow_kind: workflow_kind.to_string(),
            project_id: Some(session.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("optimization_session".to_string()),
            scope_id: Some(session_id),
            chapter_id: Some(session.chapter_id),
            mode: Some(mode.to_string()),
            input_fingerprint_json: Some(json!({
                "workflow": workflow_kind,
                "mode": mode,
                "session_id": session_id,
                "project_id": session.project_id,
                "chapter_id": session.chapter_id,
            })),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(current_step_key),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|e| e.to_string())?;
    Ok(run.id)
}

async fn finalize_optimize_workflow_run<T>(
    state: &AppState,
    workflow_run_id: i64,
    operation: &str,
    result: Result<T, String>,
) -> Result<T, String> {
    let (status, can_resume, error_message) = match &result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(message) => (WORKFLOW_STATUS_FAILED, true, Some(message.as_str())),
    };
    match state
        .workflow_run
        .mark_status(workflow_run_id, status, can_resume, error_message)
        .await
    {
        Ok(_) => result,
        Err(status_err) => match result {
            Ok(_) => Err(format!(
                "{operation}完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        },
    }
}

async fn verify_optimize_project_access(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())
}

async fn find_chapter_for_optimize(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<chapter::Model, String> {
    chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!("章节不存在: project_id={project_id}, chapter_number={chapter_number}")
        })
}

async fn find_chapter_by_id(state: &AppState, chapter_id: i64) -> Result<chapter::Model, String> {
    chapter::Entity::find_by_id(chapter_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("章节不存在: {chapter_id}"))
}

fn build_analysis_variables(
    mode: &str,
    scope: Option<&str>,
    permissions: Option<&OptimizationAgentPermissions>,
    agent_instruction: Option<&str>,
) -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
    let mut variables = std::collections::HashMap::new();
    variables.insert(
        "mode".to_string(),
        serde_json::Value::String(mode.to_string()),
    );
    if let Some(scope) = scope {
        variables.insert(
            "scope".to_string(),
            serde_json::Value::String(scope.to_string()),
        );
    }
    let permissions = permissions.cloned().unwrap_or_default();
    variables.insert("agent_permissions".to_string(), permissions.as_value());
    if let Some(instruction) = agent_instruction
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        variables.insert(
            "agent_instruction".to_string(),
            serde_json::Value::String(instruction.to_string()),
        );
    }
    Ok(variables)
}

async fn optimization_analysis_request_preview(
    state: &AppState,
    request: a4n_engine::llm::provider::ChatRequest,
) -> Result<ModelRequestPreview, String> {
    chat_status_preview_for_state(
        state,
        "optimization_analysis_agent_loop",
        "内容优化分析 Agent 请求预览",
        vec![
            chat_step(
                "optimization_analysis_step_1",
                "内容优化分析 Agent Loop 第 1 步",
                Some("content_optimization"),
                "ready",
                None,
                request,
            ),
            blocked_step(
                "optimization_analysis_followup",
                "内容优化分析 Agent Loop 后续步骤",
                Some("content_optimization"),
                "后续请求依赖上一轮模型决策、工具观察和补丁结果",
            ),
        ],
        "llm_chat",
    )
    .await
}

fn chat_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: Option<&str>,
    state: impl Into<String>,
    note: Option<&str>,
    request: a4n_engine::llm::provider::ChatRequest,
) -> ChatStatusPreviewStep {
    ChatStatusPreviewStep::llm(
        key,
        title,
        prompt_name.map(ToString::to_string),
        state,
        note.map(ToString::to_string),
        Some(request),
    )
}

fn blocked_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: Option<&str>,
    note: impl Into<String>,
) -> ChatStatusPreviewStep {
    ChatStatusPreviewStep::llm(
        key,
        title,
        prompt_name.map(ToString::to_string),
        "blocked",
        Some(note.into()),
        None,
    )
}

async fn record_optimize_command_error(
    state: &AppState,
    session_id: i64,
    phase: &str,
    err: &str,
) -> Result<(), String> {
    state
        .optimize
        .record_command_error(session_id, phase, err)
        .await
        .map_err(|save_err| format!("记录 {phase} 失败事件失败：{save_err}"))
}

async fn record_optimize_command_result<T>(
    state: &AppState,
    session_id: i64,
    phase: &str,
    result: Result<T, String>,
) -> Result<T, String> {
    match result {
        Ok(value) => Ok(value),
        Err(err) => {
            if let Err(save_err) =
                record_optimize_command_error(state, session_id, phase, &err).await
            {
                return Err(format!("{err}；{save_err}"));
            }
            Err(err)
        }
    }
}

fn to_session_summary(m: &optimization_session::Model) -> OptimizationSessionSummary {
    OptimizationSessionSummary {
        id: m.id,
        project_id: m.project_id,
        chapter_id: m.chapter_id,
        name: m.name.clone(),
        status: m.status.clone(),
        issues: m.issues.clone(),
        agent_permissions: m.agent_permissions.clone(),
        agent_instruction: m.agent_instruction.clone(),
        fixes_applied: m.fixes_applied,
        summary: m.summary.clone(),
        created_at: Some(format_utc_timestamp(m.created_at)),
        updated_at: Some(format_utc_timestamp(m.updated_at)),
    }
}

fn to_turn_summary(
    turn: &optimization_turn::Model,
    events: Vec<optimization_turn_event::Model>,
) -> OptimizationTurnSummary {
    OptimizationTurnSummary {
        id: turn.id,
        session_id: turn.session_id,
        user_text: turn.user_text.clone(),
        status: turn.status.clone(),
        permissions_snapshot: turn.permissions_snapshot.clone(),
        final_reply: turn.final_reply.clone(),
        error: turn.error.clone(),
        events: events.iter().map(to_turn_event_summary).collect(),
        created_at: Some(format_utc_timestamp(turn.created_at)),
        updated_at: Some(format_utc_timestamp(turn.updated_at)),
    }
}

fn to_turn_event_summary(m: &optimization_turn_event::Model) -> OptimizationTurnEventSummary {
    OptimizationTurnEventSummary {
        id: m.id,
        turn_id: m.turn_id,
        seq: m.seq,
        kind: m.kind.clone(),
        status: m.status.clone(),
        public_text: m.public_text.clone(),
        payload: m.payload.clone(),
        created_at: Some(format_utc_timestamp(m.created_at)),
    }
}

fn to_patch_set_summary(
    patch_set: &optimization_patch_set::Model,
    patches: Vec<optimization_patch::Model>,
) -> OptimizationPatchSetSummary {
    OptimizationPatchSetSummary {
        id: patch_set.id,
        turn_id: patch_set.turn_id,
        session_id: patch_set.session_id,
        chapter_id: patch_set.chapter_id,
        base_content_hash: patch_set.base_content_hash.clone(),
        status: patch_set.status.clone(),
        summary: patch_set.summary.clone(),
        patches: patches
            .iter()
            .map(|patch| OptimizationPatch {
                patch_id: patch.patch_id.clone(),
                issue_id: patch.issue_id.clone(),
                operation: patch.operation.clone(),
                old_text: patch.old_text.clone(),
                new_text: patch.new_text.clone(),
                reason: patch.reason.clone(),
                risk: patch.risk.clone(),
            })
            .collect(),
        created_at: Some(format_utc_timestamp(patch_set.created_at)),
        updated_at: Some(format_utc_timestamp(patch_set.updated_at)),
    }
}

fn format_utc_timestamp(value: chrono::NaiveDateTime) -> String {
    value.and_utc().to_rfc3339()
}
