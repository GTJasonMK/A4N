mod call;
mod profiles;

use self::call::run_object_discovery_call;
use self::profiles::persist_discovered_object_profiles;
use super::context::build_object_discovery_request;
use super::progress::{
    OBJECT_DISCOVERY_CALL_KEY, ObjectDiscoveryProgress, report_object_discovery_progress,
};
use super::types::ChapterAnchorMode;
use super::workflow::{
    finalize_object_discovery_workflow, object_discovery_business_key,
    object_discovery_workflow_fingerprint,
};
use crate::state::AppState;
use a4n_orchestrator::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use serde_json::Value;

pub(crate) async fn discover_object_profiles_for_chapters(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
    progress: Option<&ObjectDiscoveryProgress<'_>>,
) -> Result<Value, String> {
    report_object_discovery_progress(
        progress,
        0,
        "正在读取可分析章节和已有物体档案",
        "物体档案发现已停止",
    )?;
    let request_context = build_object_discovery_request(
        state,
        project_id,
        chapter_number,
        ChapterAnchorMode::EnsureCurrent,
    )
    .await?;
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: object_discovery_business_key(project_id, chapter_number),
            workflow_kind: "object_discovery".to_string(),
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
            input_fingerprint_json: Some(object_discovery_workflow_fingerprint(
                project_id,
                chapter_number,
                &request_context.chapters,
                &request_context.text_anchors,
                &request_context.request,
            )),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(OBJECT_DISCOVERY_CALL_KEY.to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        let call_result = run_object_discovery_call(
            state,
            workflow_run_id,
            project_id,
            chapter_number,
            &request_context.request,
            &request_context.chapters,
            &request_context.text_anchors,
            progress,
        )
        .await?;
        persist_discovered_object_profiles(
            state,
            project_id,
            chapter_number,
            request_context,
            call_result,
            progress,
        )
        .await
    }
    .await;

    if let Err(status_err) =
        finalize_object_discovery_workflow(state, workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!(
                "物体档案发现完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}
