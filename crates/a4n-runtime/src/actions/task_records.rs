use crate::{
    AppState,
    actions::{
        access,
        admin_records::{self, AdminPermission},
        canvas_workflow, video_generation,
    },
};
use a4n_core::error::AppError;
use a4n_core::event::channel::{TaskProgressEvent, TaskSnapshot};
use a4n_core::event::sse::ProgressMeta;
use a4n_core::models::generation::{
    generation_run, workflow_checkpoint, workflow_progress_event, workflow_run,
};
use a4n_core::models::task::task_queue;
use a4n_engine::queue::priority::{
    QueueSlotStats, QueueStats, TaskInfo, TaskProgress as QueueTaskProgress, TaskQueue, TaskStatus,
};
use a4n_services::workflow::{
    ListWorkflowRunsInput as ServiceListWorkflowRunsInput, UpsertWorkflowCheckpointInput,
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_RUNNING, WorkflowProgressEventInput,
};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressSummary {
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub percent: f32,
    pub meta: Option<ProgressMeta>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressEventSummary {
    pub id: String,
    pub task_id: Option<String>,
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub percent: f32,
    pub meta: Option<ProgressMeta>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfoSummary {
    pub id: String,
    pub task_type: String,
    pub priority: String,
    pub status: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub progress: Option<TaskProgressSummary>,
    pub progress_events: Vec<TaskProgressEventSummary>,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSlotStatsSummary {
    pub max_concurrent: usize,
    pub running: usize,
    pub waiting: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatsSummary {
    pub llm_queue: QueueSlotStatsSummary,
    pub image_queue: QueueSlotStatsSummary,
    pub video_queue: QueueSlotStatsSummary,
    pub embedding_queue: QueueSlotStatsSummary,
    pub background_queue: QueueSlotStatsSummary,
    pub high_priority_waiting: usize,
    pub medium_priority_waiting: usize,
    pub low_priority_waiting: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProgressSummary {
    pub task_id: String,
    pub stage: String,
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub percent: f32,
    pub meta: Option<ProgressMeta>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStateSummary {
    pub task_id: String,
    pub status: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub stage: Option<String>,
    pub progress: Option<WorkflowProgressSummary>,
    pub progress_events: Vec<TaskProgressEventSummary>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunProgressSummary {
    pub current: u32,
    pub total: u32,
    pub message: String,
    pub percent: f32,
    pub meta: Option<ProgressMeta>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunSummary {
    pub id: i64,
    pub business_key: String,
    pub workflow_kind: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub scope_table: Option<String>,
    pub scope_id: Option<i64>,
    pub chapter_id: Option<i64>,
    pub mode: Option<String>,
    pub status: String,
    pub current_step_key: Option<String>,
    pub progress: Option<WorkflowRunProgressSummary>,
    pub active_task_id: Option<String>,
    pub can_resume: bool,
    pub resume_block_reason: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCheckpointSummary {
    pub id: i64,
    pub workflow_run_id: i64,
    pub step_key: String,
    pub step_order: i32,
    pub status: String,
    pub input_fingerprint_sha256: Option<String>,
    pub input_fingerprint_json: Option<Value>,
    pub checkpoint_json: Option<Value>,
    pub artifacts_json: Option<Value>,
    pub generation_run_ids_json: Option<Value>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRunSummary {
    pub id: i64,
    pub workflow_run_id: Option<i64>,
    pub workflow: String,
    pub step_key: String,
    pub call_key: Option<String>,
    pub title: String,
    pub model_kind: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub input_fingerprint_sha256: Option<String>,
    pub input_fingerprint_json: Option<Value>,
    pub raw_response_json: Option<Value>,
    pub parsed_output_json: Option<Value>,
    pub validation_json: Option<Value>,
    pub artifact_refs_json: Option<Value>,
    pub attempt: i32,
    pub response_summary_json: Option<Value>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunDetailSummary {
    pub run: WorkflowRunSummary,
    pub checkpoints: Vec<WorkflowCheckpointSummary>,
    pub generation_runs: Vec<GenerationRunSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelWorkflowInput {
    pub task_id: Option<String>,
    pub workflow_run_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowProgressEventsInput {
    pub task_id: Option<String>,
    pub workflow_run_id: Option<i64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ListWorkflowRunsInput {
    pub project_id: Option<i64>,
    pub workflow_kind: Option<String>,
    pub status: Option<String>,
    pub resumable_only: Option<bool>,
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetWorkflowRunInput {
    pub workflow_run_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResumeWorkflowRunInput {
    pub workflow_run_id: i64,
}

pub async fn cancel_task(state: &AppState, user_id: i64, task_id: &str) -> Result<(), String> {
    verify_task_access(state, user_id, task_id).await?;
    state
        .task_queue
        .cancel(task_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_queue_stats(state: &AppState, user_id: i64) -> Result<QueueStatsSummary, String> {
    require_admin_for_unscoped_task(state, user_id).await?;
    Ok(queue_stats_summary(&state.task_queue.queue_stats()))
}

pub async fn get_task_history(
    state: &AppState,
    user_id: i64,
    project_id: Option<i64>,
    offset: u64,
    limit: u64,
) -> Result<Vec<TaskInfoSummary>, String> {
    validate_task_history_limit(limit)?;
    validate_optional_project_id(project_id)?;
    verify_task_history_access(state, user_id, project_id).await?;
    let history = state
        .task_queue
        .history(project_id, offset, limit)
        .await
        .map_err(|e| e.to_string())?;
    Ok(history.iter().map(task_info_summary).collect())
}

pub async fn list_active_workflows(state: &AppState) -> Result<Vec<WorkflowStateSummary>, String> {
    let tasks = state
        .task_queue
        .history(None, 0, 100)
        .await
        .map_err(|e| e.to_string())?;

    let mut active: Vec<WorkflowStateSummary> = tasks
        .iter()
        .filter(|task| matches!(task.status, TaskStatus::Running))
        .map(task_info_to_workflow)
        .collect();

    let mut seen: HashSet<String> = active.iter().map(|item| item.task_id.clone()).collect();
    for snapshot in state.task_manager.list_active_snapshots() {
        if seen.insert(snapshot.task_id.clone()) {
            active.push(task_manager_snapshot_to_workflow(snapshot));
        }
    }

    Ok(active)
}

pub async fn list_workflow_runs_for_user(
    state: &AppState,
    user_id: i64,
    input: ListWorkflowRunsInput,
) -> Result<Vec<WorkflowRunSummary>, String> {
    validate_optional_project_id(input.project_id)?;
    if let Some(project_id) = input.project_id {
        access::verify_project_editor_string(state, user_id, project_id).await?;
    }
    let runs = state
        .workflow_run
        .list_runs(ServiceListWorkflowRunsInput {
            project_id: input.project_id,
            workflow_kind: input.workflow_kind,
            status: input.status,
            resumable_only: input.resumable_only,
            limit: input.limit,
        })
        .await
        .map_err(|err| err.to_string())?;
    let mut visible = Vec::new();
    for run in runs {
        if workflow_run_visible_to_user(state, user_id, &run).await? {
            visible.push(workflow_run_summary(run));
        }
    }
    Ok(visible)
}

pub async fn get_workflow_run_detail_for_user(
    state: &AppState,
    user_id: i64,
    input: GetWorkflowRunInput,
) -> Result<WorkflowRunDetailSummary, String> {
    let run = state
        .workflow_run
        .get_run(input.workflow_run_id)
        .await
        .map_err(|err| err.to_string())?;
    verify_workflow_run_access(state, user_id, &run).await?;
    let checkpoints = state
        .workflow_run
        .checkpoints(input.workflow_run_id)
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(workflow_checkpoint_summary)
        .collect();
    let generation_runs = state
        .generation_run
        .runs_for_workflow(input.workflow_run_id, 300)
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(generation_run_summary)
        .collect();
    Ok(WorkflowRunDetailSummary {
        run: workflow_run_summary(run),
        checkpoints,
        generation_runs,
    })
}

pub async fn resume_workflow_run_for_user(
    state: &AppState,
    user_id: i64,
    input: ResumeWorkflowRunInput,
) -> Result<WorkflowRunSummary, String> {
    let run = state
        .workflow_run
        .get_run(input.workflow_run_id)
        .await
        .map_err(|err| err.to_string())?;
    verify_workflow_run_access(state, user_id, &run).await?;
    if !run.can_resume {
        return Err(run
            .resume_block_reason
            .unwrap_or_else(|| "该工作流当前不可恢复".to_string()));
    }

    match run.workflow_kind.as_str() {
        "video_provider_job" => {
            let video_id = run
                .scope_id
                .filter(|_| run.scope_table.as_deref() == Some("generated_video"))
                .ok_or_else(|| "视频 provider 工作流缺少 generated_video 目标".to_string())?;
            video_generation::poll_video_job(state, user_id, video_id).await?;
            let updated = state
                .workflow_run
                .get_run(input.workflow_run_id)
                .await
                .map_err(|err| err.to_string())?;
            Ok(workflow_run_summary(updated))
        }
        "video_canvas_graph" => {
            let summary =
                canvas_workflow::resume_canvas_graph_run(state, user_id, input.workflow_run_id)
                    .await?;
            let updated = state
                .workflow_run
                .get_run(summary.workflow_run_id)
                .await
                .map_err(|err| err.to_string())?;
            Ok(workflow_run_summary(updated))
        }
        "chapter_analysis" => {
            crate::workflow::resume_chapter_analysis_workflow_run(state, user_id, &run).await?;
            let updated = state
                .workflow_run
                .get_run(input.workflow_run_id)
                .await
                .map_err(|err| err.to_string())?;
            Ok(workflow_run_summary(updated))
        }
        "manga_pipeline" => Err("漫画流水线请在漫画章节页使用“续传流水线”入口恢复".to_string()),
        other => Err(format!("工作流类型 {other} 暂未接入统一恢复入口")),
    }
}

pub async fn list_active_workflows_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<WorkflowStateSummary>, String> {
    let active = list_active_workflows(state).await?;
    let mut visible = Vec::new();
    for workflow in active {
        if workflow_visible_to_user(state, user_id, &workflow).await? {
            visible.push(workflow);
        }
    }
    Ok(visible)
}

pub async fn cancel_workflow(
    state: &AppState,
    user_id: i64,
    input: CancelWorkflowInput,
) -> Result<(), String> {
    let has_task_id = input
        .task_id
        .as_deref()
        .is_some_and(|task_id| !task_id.trim().is_empty());
    let has_workflow_run_id = input.workflow_run_id.is_some();
    if !has_task_id && !has_workflow_run_id {
        return Err("必须提供 task_id 或 workflow_run_id".to_string());
    }
    if let Some(task_id) = input
        .task_id
        .as_deref()
        .map(str::trim)
        .filter(|task_id| !task_id.is_empty())
    {
        verify_task_access(state, user_id, task_id).await?;
        if state.task_manager.cancel(task_id) {
            return Ok(());
        }
        state
            .task_queue
            .cancel(task_id)
            .await
            .map_err(|err| err.to_string())?;
    }
    if let Some(workflow_run_id) = input.workflow_run_id {
        cancel_workflow_run_for_user(state, user_id, workflow_run_id).await?;
    }
    Ok(())
}

async fn cancel_workflow_run_for_user(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
) -> Result<(), String> {
    let run = state
        .workflow_run
        .get_run(workflow_run_id)
        .await
        .map_err(|err| err.to_string())?;
    verify_workflow_run_access(state, user_id, &run).await?;
    if let Some(task_id) = run.active_task_id.as_deref() {
        let _ = state.task_manager.cancel(task_id);
        let _ = state.task_queue.cancel(task_id).await;
    }
    let checkpoints = state
        .workflow_run
        .checkpoints(workflow_run_id)
        .await
        .map_err(|err| err.to_string())?;
    interrupt_canvas_generated_videos_for_cancelled_workflow(state, &checkpoints).await?;
    if matches!(
        run.status.as_str(),
        "completed" | "failed" | "cancelled" | "superseded"
    ) {
        return Ok(());
    }
    for checkpoint in checkpoints
        .into_iter()
        .filter(|checkpoint| checkpoint.status == WORKFLOW_STATUS_RUNNING)
    {
        state
            .workflow_run
            .upsert_checkpoint(UpsertWorkflowCheckpointInput {
                workflow_run_id,
                step_key: checkpoint.step_key,
                step_order: checkpoint.step_order,
                status: WORKFLOW_STATUS_CANCELLED.to_string(),
                input_fingerprint_sha256: checkpoint.input_fingerprint_sha256,
                input_fingerprint_json: checkpoint.input_fingerprint_json,
                checkpoint_json: checkpoint.checkpoint_json,
                artifacts_json: checkpoint.artifacts_json,
                generation_run_ids_json: checkpoint.generation_run_ids_json,
                error_message: Some("用户取消画布运行".to_string()),
                completed: true,
            })
            .await
            .map_err(|err| err.to_string())?;
    }
    state
        .workflow_run
        .mark_status(
            workflow_run_id,
            WORKFLOW_STATUS_CANCELLED,
            false,
            Some("用户取消画布运行"),
        )
        .await
        .map_err(|err| err.to_string())?;
    state
        .workflow_run
        .record_progress_event(WorkflowProgressEventInput {
            workflow_run_id,
            task_id: run.active_task_id.as_deref(),
            current: 1,
            total: 1,
            message: "画布运行已取消",
            meta: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

async fn interrupt_canvas_generated_videos_for_cancelled_workflow(
    state: &AppState,
    checkpoints: &[workflow_checkpoint::Model],
) -> Result<(), String> {
    let mut video_ids = HashSet::new();
    for checkpoint in checkpoints {
        if checkpoint.status != WORKFLOW_STATUS_RUNNING {
            continue;
        }
        if let Some(video_id) = checkpoint
            .artifacts_json
            .as_ref()
            .and_then(|value| value.get("generated_video_id"))
            .and_then(Value::as_i64)
        {
            video_ids.insert(video_id);
        }
        if let Some(video_id) = checkpoint
            .input_fingerprint_json
            .as_ref()
            .and_then(|value| value.get("generated_video_id"))
            .and_then(Value::as_i64)
        {
            video_ids.insert(video_id);
        }
    }
    for video_id in video_ids {
        let video = match state.video.get_video(video_id).await {
            Ok(video) => video,
            Err(err) => {
                tracing::warn!(video_id, error = %err, "取消画布运行时读取关联视频失败");
                continue;
            }
        };
        if matches!(
            video.status.as_str(),
            "completed" | "failed" | "cancelled" | "interrupted"
        ) {
            continue;
        }
        state
            .video
            .mark_interrupted_and_update_prompt(video.id, None, "用户取消画布运行")
            .await
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

pub async fn get_workflow_progress_events(
    state: &AppState,
    user_id: i64,
    input: WorkflowProgressEventsInput,
) -> Result<Vec<TaskProgressEventSummary>, String> {
    let limit = input.limit.unwrap_or(50).clamp(1, 200);
    if let Some(workflow_run_id) = input.workflow_run_id {
        let run = state
            .workflow_run
            .get_run(workflow_run_id)
            .await
            .map_err(|err| err.to_string())?;
        verify_workflow_run_access(state, user_id, &run).await?;
        return state
            .workflow_run
            .progress_events(workflow_run_id, limit)
            .await
            .map_err(|err| err.to_string())
            .map(|events| {
                events
                    .into_iter()
                    .map(workflow_progress_event_summary)
                    .collect()
            });
    }

    let Some(task_id) = input.task_id else {
        return Err("必须提供 task_id 或 workflow_run_id".to_string());
    };
    verify_task_access(state, user_id, &task_id).await?;

    if let Some(snapshot) = state
        .task_manager
        .list_active_snapshots()
        .into_iter()
        .find(|snapshot| snapshot.task_id == task_id)
    {
        return Ok(snapshot
            .progress_events
            .into_iter()
            .rev()
            .take(limit)
            .map(task_progress_event_summary)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect());
    }

    if let Some(task) = task_queue::Entity::find_by_id(task_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        && let Some(workflow_run_id) = task.workflow_run_id
    {
        let run = state
            .workflow_run
            .get_run(workflow_run_id)
            .await
            .map_err(|err| err.to_string())?;
        verify_workflow_run_access(state, user_id, &run).await?;
        return state
            .workflow_run
            .progress_events(workflow_run_id, limit)
            .await
            .map_err(|err| err.to_string())
            .map(|events| {
                events
                    .into_iter()
                    .map(workflow_progress_event_summary)
                    .collect()
            });
    }

    Ok(Vec::new())
}

async fn workflow_visible_to_user(
    state: &AppState,
    user_id: i64,
    workflow: &WorkflowStateSummary,
) -> Result<bool, String> {
    if let Some(project_id) = workflow.project_id {
        return project_visible_to_user(state, user_id, project_id).await;
    }
    if let Some(root_project_id) = workflow.root_project_id {
        return project_visible_to_user(state, user_id, root_project_id).await;
    }
    admin_records::is_active_admin_user(state, user_id)
        .await
        .map_err(|err| err.to_string())
}

async fn workflow_run_visible_to_user(
    state: &AppState,
    user_id: i64,
    run: &workflow_run::Model,
) -> Result<bool, String> {
    if let Some(project_id) = run.project_id.or(run.root_project_id) {
        return project_visible_to_user(state, user_id, project_id).await;
    }
    admin_records::is_active_admin_user(state, user_id)
        .await
        .map_err(|err| err.to_string())
}

async fn verify_workflow_run_access(
    state: &AppState,
    user_id: i64,
    run: &workflow_run::Model,
) -> Result<(), String> {
    verify_task_project_access(state, user_id, run.project_id, run.root_project_id).await
}

async fn project_visible_to_user(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<bool, String> {
    match access::verify_project_editor(state, user_id, project_id).await {
        Ok(()) => Ok(true),
        Err(AppError::Forbidden(_)) | Err(AppError::NotFound(_)) => Ok(false),
        Err(err) => Err(err.to_string()),
    }
}

async fn verify_task_history_access(
    state: &AppState,
    user_id: i64,
    project_id: Option<i64>,
) -> Result<(), String> {
    if let Some(project_id) = project_id {
        return access::verify_project_editor_string(state, user_id, project_id).await;
    }
    require_admin_for_unscoped_task(state, user_id).await
}

async fn verify_task_access(state: &AppState, user_id: i64, task_id: &str) -> Result<(), String> {
    if let Some(task) = task_queue::Entity::find_by_id(task_id.to_string())
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
    {
        return verify_task_project_access(state, user_id, task.project_id, task.root_project_id)
            .await;
    }

    let snapshot = state
        .task_manager
        .list_active_snapshots()
        .into_iter()
        .find(|snapshot| snapshot.task_id == task_id)
        .ok_or_else(|| format!("任务不存在: {task_id}"))?;
    verify_task_project_access(
        state,
        user_id,
        snapshot.project_id,
        snapshot.root_project_id,
    )
    .await
}

async fn verify_task_project_access(
    state: &AppState,
    user_id: i64,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<(), String> {
    if let Some(project_id) = project_id.or(root_project_id) {
        return access::verify_project_editor_string(state, user_id, project_id).await;
    }
    require_admin_for_unscoped_task(state, user_id).await
}

async fn require_admin_for_unscoped_task(state: &AppState, user_id: i64) -> Result<(), String> {
    admin_records::require_admin_permission(state, user_id, AdminPermission::ManageSettings)
        .await
        .map_err(|err| err.to_string())
}

fn task_info_summary(info: &TaskInfo) -> TaskInfoSummary {
    TaskInfoSummary {
        id: info.id.clone(),
        task_type: info.task_type.as_str().to_string(),
        priority: info.priority.as_str().to_string(),
        status: info.status.as_str().to_string(),
        project_id: info.project_id,
        root_project_id: info.root_project_id,
        source_snapshot_id: info.source_snapshot_id,
        task_scope: info.task_scope.clone(),
        progress: info.progress.as_ref().map(|progress| TaskProgressSummary {
            current: progress.current,
            total: progress.total,
            message: progress.message.clone(),
            percent: progress.percent() * 100.0,
            meta: progress.meta.clone(),
            updated_at: progress.updated_at.clone(),
        }),
        progress_events: info
            .progress
            .as_ref()
            .map(|progress| queue_progress_event_summary(&info.id, progress, info.created_at))
            .into_iter()
            .collect(),
        created_at: Some(info.created_at.to_string()),
        started_at: info.started_at.map(|value| value.to_string()),
        completed_at: info.completed_at.map(|value| value.to_string()),
    }
}

fn workflow_run_summary(run: workflow_run::Model) -> WorkflowRunSummary {
    WorkflowRunSummary {
        id: run.id,
        business_key: run.business_key,
        workflow_kind: run.workflow_kind,
        project_id: run.project_id,
        root_project_id: run.root_project_id,
        source_snapshot_id: run.source_snapshot_id,
        scope_table: run.scope_table,
        scope_id: run.scope_id,
        chapter_id: run.chapter_id,
        mode: run.mode,
        status: run.status,
        current_step_key: run.current_step_key,
        progress: run
            .progress_json
            .as_ref()
            .map(workflow_run_progress_summary),
        active_task_id: run.active_task_id,
        can_resume: run.can_resume,
        resume_block_reason: run.resume_block_reason,
        error_message: run.error_message,
        created_at: run.created_at.to_string(),
        updated_at: run.updated_at.to_string(),
        completed_at: run.completed_at.map(|value| value.to_string()),
    }
}

fn workflow_checkpoint_summary(
    checkpoint: workflow_checkpoint::Model,
) -> WorkflowCheckpointSummary {
    WorkflowCheckpointSummary {
        id: checkpoint.id,
        workflow_run_id: checkpoint.workflow_run_id,
        step_key: checkpoint.step_key,
        step_order: checkpoint.step_order,
        status: checkpoint.status,
        input_fingerprint_sha256: checkpoint.input_fingerprint_sha256,
        input_fingerprint_json: checkpoint.input_fingerprint_json,
        checkpoint_json: checkpoint.checkpoint_json,
        artifacts_json: checkpoint.artifacts_json,
        generation_run_ids_json: checkpoint.generation_run_ids_json,
        error_message: checkpoint.error_message,
        created_at: checkpoint.created_at.to_string(),
        updated_at: checkpoint.updated_at.to_string(),
        completed_at: checkpoint.completed_at.map(|value| value.to_string()),
    }
}

fn generation_run_summary(run: generation_run::Model) -> GenerationRunSummary {
    GenerationRunSummary {
        id: run.id,
        workflow_run_id: run.workflow_run_id,
        workflow: run.workflow,
        step_key: run.step_key,
        call_key: run.call_key,
        title: run.title,
        model_kind: run.model_kind,
        provider: run.provider,
        model: run.model,
        status: run.status,
        input_fingerprint_sha256: run.input_fingerprint_sha256,
        input_fingerprint_json: run.input_fingerprint_json,
        raw_response_json: run.raw_response_json,
        parsed_output_json: run.parsed_output_json,
        validation_json: run.validation_json,
        artifact_refs_json: run.artifact_refs_json,
        attempt: run.attempt,
        response_summary_json: run.response_summary_json,
        error_message: run.error_message,
        created_at: run.created_at.to_string(),
        completed_at: run.completed_at.map(|value| value.to_string()),
    }
}

fn workflow_run_progress_summary(value: &serde_json::Value) -> WorkflowRunProgressSummary {
    let current = value
        .get("current")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default() as u32;
    let total = value
        .get("total")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1) as u32;
    WorkflowRunProgressSummary {
        current,
        total,
        message: value
            .get("message")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string(),
        percent: value
            .get("percent")
            .and_then(serde_json::Value::as_f64)
            .map(|value| value as f32)
            .unwrap_or_else(|| {
                if total == 0 {
                    0.0
                } else {
                    (current as f32 / total as f32) * 100.0
                }
            }),
        meta: value
            .get("meta")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()),
        updated_at: value
            .get("updated_at")
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string),
    }
}

fn task_info_to_workflow(info: &TaskInfo) -> WorkflowStateSummary {
    WorkflowStateSummary {
        task_id: info.id.clone(),
        status: "running".to_string(),
        project_id: info.project_id,
        root_project_id: info.root_project_id,
        source_snapshot_id: info.source_snapshot_id,
        task_scope: info.task_scope.clone(),
        stage: info
            .progress
            .as_ref()
            .map(|progress| progress.message.clone()),
        progress: info
            .progress
            .as_ref()
            .map(|progress| WorkflowProgressSummary {
                task_id: info.id.clone(),
                stage: progress
                    .meta
                    .as_ref()
                    .map(|meta| meta.stage_label.clone())
                    .unwrap_or_else(|| progress.message.clone()),
                current: progress.current,
                total: progress.total,
                message: progress.message.clone(),
                percent: progress.percent() * 100.0,
                meta: progress.meta.clone(),
                updated_at: progress.updated_at.clone(),
            }),
        progress_events: info
            .progress
            .as_ref()
            .map(|progress| {
                queue_progress_event_summary(
                    &info.id,
                    progress,
                    info.started_at.unwrap_or(info.created_at),
                )
            })
            .into_iter()
            .collect(),
        started_at: info.started_at.map(|value| value.to_string()),
        completed_at: None,
    }
}

fn task_manager_snapshot_to_workflow(snapshot: TaskSnapshot) -> WorkflowStateSummary {
    let stage = snapshot.progress.message.clone();
    let percent = snapshot.progress.percent() * 100.0;
    WorkflowStateSummary {
        task_id: snapshot.task_id.clone(),
        status: if snapshot.cancelled {
            "cancelled".to_string()
        } else {
            "running".to_string()
        },
        project_id: snapshot.project_id,
        root_project_id: snapshot.root_project_id,
        source_snapshot_id: snapshot.source_snapshot_id,
        task_scope: snapshot.task_scope,
        stage: Some(format!(
            "{}: {}",
            snapshot.task_type.as_str(),
            snapshot.progress.message
        )),
        progress: Some(WorkflowProgressSummary {
            task_id: snapshot.task_id,
            stage,
            current: snapshot.progress.current,
            total: snapshot.progress.total,
            message: snapshot.progress.message,
            percent,
            meta: snapshot.progress.meta,
            updated_at: snapshot.progress.updated_at,
        }),
        progress_events: snapshot
            .progress_events
            .into_iter()
            .map(task_progress_event_summary)
            .collect(),
        started_at: None,
        completed_at: None,
    }
}

fn workflow_progress_event_summary(
    event: workflow_progress_event::Model,
) -> TaskProgressEventSummary {
    TaskProgressEventSummary {
        id: event.id.to_string(),
        task_id: event.task_id,
        current: event.current.max(0) as u32,
        total: event.total.max(1) as u32,
        message: event.message,
        percent: event.percent,
        meta: event
            .meta_json
            .and_then(|value| serde_json::from_value(value).ok()),
        timestamp: event.created_at.to_string(),
    }
}

fn task_progress_event_summary(event: TaskProgressEvent) -> TaskProgressEventSummary {
    TaskProgressEventSummary {
        id: event.id,
        task_id: Some(event.task_id),
        current: event.current,
        total: event.total,
        message: event.message,
        percent: event.percent,
        meta: event.meta,
        timestamp: event.timestamp,
    }
}

fn queue_progress_event_summary(
    task_id: &str,
    progress: &QueueTaskProgress,
    timestamp: chrono::NaiveDateTime,
) -> TaskProgressEventSummary {
    TaskProgressEventSummary {
        id: format!("{task_id}-snapshot"),
        task_id: Some(task_id.to_string()),
        current: progress.current,
        total: progress.total,
        message: progress.message.clone(),
        percent: progress.percent() * 100.0,
        meta: progress.meta.clone(),
        timestamp: progress
            .updated_at
            .clone()
            .unwrap_or_else(|| timestamp.to_string()),
    }
}

fn queue_stats_summary(stats: &QueueStats) -> QueueStatsSummary {
    QueueStatsSummary {
        llm_queue: queue_slot_stats_summary(&stats.llm_queue),
        image_queue: queue_slot_stats_summary(&stats.image_queue),
        video_queue: queue_slot_stats_summary(&stats.video_queue),
        embedding_queue: queue_slot_stats_summary(&stats.embedding_queue),
        background_queue: queue_slot_stats_summary(&stats.background_queue),
        high_priority_waiting: stats.high_priority_waiting,
        medium_priority_waiting: stats.medium_priority_waiting,
        low_priority_waiting: stats.low_priority_waiting,
    }
}

fn queue_slot_stats_summary(stats: &QueueSlotStats) -> QueueSlotStatsSummary {
    QueueSlotStatsSummary {
        max_concurrent: stats.max_concurrent,
        running: stats.running,
        waiting: stats.waiting,
    }
}

fn validate_task_history_limit(limit: u64) -> Result<(), String> {
    if limit == 0 {
        Err("limit 必须大于 0".to_string())
    } else {
        Ok(())
    }
}

fn validate_optional_project_id(project_id: Option<i64>) -> Result<(), String> {
    match project_id {
        Some(project_id) if project_id <= 0 => Err("project_id 必须大于 0".to_string()),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cancel_workflow_input_rejects_camel_case_task_id_alias() {
        let input = serde_json::from_value::<CancelWorkflowInput>(json!({
            "task_id": "workflow-1"
        }))
        .expect("current task_id field should parse");
        assert_eq!(input.task_id.as_deref(), Some("workflow-1"));
        assert_eq!(input.workflow_run_id, None);

        let run_input = serde_json::from_value::<CancelWorkflowInput>(json!({
            "workflow_run_id": 70
        }))
        .expect("workflow_run_id field should parse");
        assert_eq!(run_input.task_id, None);
        assert_eq!(run_input.workflow_run_id, Some(70));

        let err = serde_json::from_value::<CancelWorkflowInput>(json!({
            "taskId": "workflow-1"
        }))
        .expect_err("camelCase taskId alias should fail");
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn workflow_progress_events_input_accepts_task_or_workflow_run_id() {
        let task_input = serde_json::from_value::<WorkflowProgressEventsInput>(json!({
            "task_id": "task-1",
            "limit": 20
        }))
        .expect("task_id progress event query should parse");
        assert_eq!(task_input.task_id.as_deref(), Some("task-1"));
        assert_eq!(task_input.workflow_run_id, None);
        assert_eq!(task_input.limit, Some(20));

        let workflow_input = serde_json::from_value::<WorkflowProgressEventsInput>(json!({
            "workflow_run_id": 9
        }))
        .expect("workflow_run_id progress event query should parse");
        assert_eq!(workflow_input.task_id, None);
        assert_eq!(workflow_input.workflow_run_id, Some(9));

        let err = serde_json::from_value::<WorkflowProgressEventsInput>(json!({
            "workflowRunId": 9
        }))
        .expect_err("camelCase workflowRunId alias should fail");
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn workflow_progress_event_summary_keeps_missing_task_id_null() {
        let created_at = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let summary = workflow_progress_event_summary(workflow_progress_event::Model {
            id: 1,
            workflow_run_id: 9,
            task_id: None,
            event_index: 1,
            current: 2,
            total: 4,
            message: "正在处理".to_string(),
            percent: 50.0,
            meta_json: None,
            created_at,
        });
        assert_eq!(summary.task_id, None);
        assert_eq!(summary.id, "1");
        assert_eq!(summary.current, 2);
        assert_eq!(summary.total, 4);
    }

    #[test]
    fn task_history_limit_must_be_positive() {
        assert!(validate_task_history_limit(1).is_ok());
        let err = validate_task_history_limit(0).expect_err("zero limit should fail");
        assert_eq!(err, "limit 必须大于 0");
    }

    #[test]
    fn optional_project_id_must_be_positive_when_present() {
        assert!(validate_optional_project_id(None).is_ok());
        assert!(validate_optional_project_id(Some(1)).is_ok());

        let zero = validate_optional_project_id(Some(0)).expect_err("zero project_id should fail");
        assert_eq!(zero, "project_id 必须大于 0");

        let negative =
            validate_optional_project_id(Some(-1)).expect_err("negative project_id should fail");
        assert_eq!(negative, "project_id 必须大于 0");
    }
}
