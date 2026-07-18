use super::prelude::*;
use super::*;

pub(super) async fn mark_chapter_analysis_current_step(
    state: &AppState,
    workflow_run_id: Option<i64>,
    step_key: &str,
) -> Result<(), String> {
    if let Some(workflow_run_id) = workflow_run_id {
        state
            .orchestrator
            .mark_workflow_current_step(workflow_run_id, step_key)
            .await
            .map_err(|err| format!("更新章节分析工作流当前步骤失败: {err}"))?;
    }
    Ok(())
}

pub(super) async fn completed_chapter_analysis_phase(
    state: &AppState,
    workflow_run_id: Option<i64>,
    ch: &chapter::Model,
    step_key: &str,
) -> Result<Option<Value>, String> {
    let fingerprint = chapter_analysis_phase_fingerprint(ch, step_key);
    state
        .orchestrator
        .load_completed_workflow_checkpoint(workflow_run_id, step_key, &fingerprint)
        .await
        .map_err(|error| error.to_string())?
        .map(|checkpoint| {
            required_checkpoint_json(&checkpoint, step_key).map_err(|error| error.to_string())
        })
        .transpose()
}

pub(super) async fn save_completed_chapter_analysis_phase(
    state: &AppState,
    workflow_run_id: Option<i64>,
    ch: &chapter::Model,
    step_key: &str,
    step_order: i32,
    checkpoint_json: Value,
) -> Result<(), String> {
    state
        .orchestrator
        .save_completed_workflow_checkpoint(CompletedWorkflowCheckpointInput {
            workflow_run_id,
            step_key: step_key.to_string(),
            step_order,
            input_fingerprint_json: chapter_analysis_phase_fingerprint(ch, step_key),
            checkpoint_json,
        })
        .await
        .map_err(|error| error.to_string())
}

pub(super) async fn preflight_chapter_analysis(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
    operation: &str,
    empty_content_message: &str,
    require_saved_analysis: bool,
) -> Result<chapter::Model, String> {
    let ch = load_chapter_by_id(
        state,
        chapter_id,
        &format!("{operation}失败：查询章节失败"),
        format!("{operation}失败：章节不存在: chapter_id={chapter_id}"),
    )
    .await?;
    access::verify_project_editor(state, user_id, ch.project_id)
        .await
        .map_err(|err| err.to_string())?;
    ensure_chapter_not_generating(&ch, operation)?;
    if !ch.has_available_content() {
        return Err(empty_content_message.to_string());
    }
    if require_saved_analysis {
        let analysis = ch
            .analysis_data
            .as_ref()
            .ok_or_else(|| "当前章节还没有已保存的结构化分析，不能继续同步".to_string())?;
        validate_saved_chapter_analysis(analysis)
            .map_err(|err| format!("当前章节已保存的结构化分析数据无效，不能继续同步：{err}"))?;
        ensure_chapter_analysis_sync_resume_needed(state, &ch).await?;
    }
    ensure_previous_chapter_analysis_complete(state, &ch, operation).await?;
    Ok(ch)
}

pub(super) async fn ensure_chapter_analysis_sync_resume_needed(
    state: &AppState,
    ch: &chapter::Model,
) -> Result<(), String> {
    let run = state
        .orchestrator
        .latest_workflow_run_by_business_key(&chapter_analysis_workflow_business_key(
            ch.project_id,
            ch.id,
        ))
        .await
        .map_err(|err| format!("检查章节结构化分析同步状态失败: {err}"))?;
    let Some(run) = run else {
        return Ok(());
    };
    if let Some(reason) = chapter_analysis_resume_block_reason(&run, ch) {
        return Err(reason.to_string());
    }
    Ok(())
}
