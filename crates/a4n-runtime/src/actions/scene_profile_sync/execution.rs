mod chapters;
mod persistence;
mod scope;

use super::context::select_scene_profile_sync_chapters;
use super::progress::{SceneProfileSyncProgress, report_scene_profile_sync_progress};
use super::workflow::{
    finalize_scene_profile_sync_workflow, scene_profile_sync_business_key,
    scene_profile_sync_workflow_fingerprint,
};
use crate::state::AppState;
use a4n_orchestrator::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use serde_json::Value;

pub use scope::scene_profile_sync_task_scope;

pub async fn sync_scene_profiles_for_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    progress: Option<&SceneProfileSyncProgress<'_>>,
) -> Result<Value, String> {
    scene_profile_sync_task_scope(state, user_id, project_id).await?;
    report_scene_profile_sync_progress(progress, 0, "正在读取可同步章节", "场景档案同步已停止")?;
    let chapters = select_scene_profile_sync_chapters(state, project_id, chapter_number).await?;
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: scene_profile_sync_business_key(project_id, chapter_number),
            workflow_kind: "scene_profile_sync".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("novel_project".to_string()),
            scope_id: Some(project_id),
            chapter_id: None,
            mode: Some(
                chapter_number
                    .map(|value| format!("chapter:{value}"))
                    .unwrap_or_else(|| "all".to_string()),
            ),
            input_fingerprint_json: Some(scene_profile_sync_workflow_fingerprint(
                project_id,
                chapter_number,
                &chapters,
            )),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("scene_profile_sync.prepare".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = chapters::sync_scene_profile_chapters(
        state,
        project_id,
        workflow_run_id,
        chapters,
        progress,
    )
    .await;

    if let Err(status_err) =
        finalize_scene_profile_sync_workflow(state, workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!(
                "场景档案同步完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}
