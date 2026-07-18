mod call;
mod profiles;

use self::{call::run_character_discovery_call, profiles::persist_discovered_character_profiles};
use super::{
    contracts::{
        CHARACTER_DISCOVERY_CALL_KEY, CHARACTER_DISCOVERY_PROGRESS_TOTAL, ChapterAnchorMode,
    },
    request::build_character_discovery_request,
    rules::report_character_discovery_progress,
    workflow::{
        character_discovery_business_key, character_discovery_workflow_fingerprint,
        finalize_character_discovery_workflow,
    },
};
use crate::{AppState, actions::access};
use a4n_orchestrator::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use serde_json::Value;

use super::super::CharacterAiProgress;

pub async fn discover_character_profiles_for_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<Value, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    report_character_discovery_progress(
        progress,
        0,
        CHARACTER_DISCOVERY_PROGRESS_TOTAL,
        "正在读取可分析章节和已有角色",
        "角色发现已停止",
    )?;
    let request_context = build_character_discovery_request(
        state,
        project_id,
        chapter_number,
        ChapterAnchorMode::EnsureCurrent,
    )
    .await?;
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: character_discovery_business_key(project_id, chapter_number),
            workflow_kind: "character_discovery".to_string(),
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
            input_fingerprint_json: Some(character_discovery_workflow_fingerprint(
                project_id,
                chapter_number,
                &request_context.chapters,
                &request_context.text_anchors,
                &request_context.request,
            )),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(CHARACTER_DISCOVERY_CALL_KEY.to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        let call_result = run_character_discovery_call(
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
        let result = persist_discovered_character_profiles(
            state,
            project_id,
            chapter_number,
            request_context,
            call_result,
            progress,
        )
        .await?;
        report_character_discovery_progress(
            progress,
            CHARACTER_DISCOVERY_PROGRESS_TOTAL,
            CHARACTER_DISCOVERY_PROGRESS_TOTAL,
            "角色发现已完成",
            "角色发现已停止",
        )?;
        Ok(result)
    }
    .await;

    if let Err(status_err) =
        finalize_character_discovery_workflow(state, workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!("角色发现完成，但工作流状态写入失败: {status_err}")),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}
