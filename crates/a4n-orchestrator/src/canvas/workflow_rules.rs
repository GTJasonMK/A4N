//! 画布图/节点工作流 business key、begin 输入与节点完成态纯规则（无 DB）。

use serde_json::Value;

use crate::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING, WorkflowRunFinalizeOutcome,
};

/// 视频画布图工作流 kind。
pub const WORKFLOW_KIND_VIDEO_CANVAS_GRAPH: &str = "video_canvas_graph";

/// 画布整图 exclusive business key（含时间戳以保证每次 run 独立）。
pub fn canvas_graph_workflow_business_key(
    graph_id: i64,
    mode: &str,
    unique_suffix: i64,
) -> Result<String, String> {
    if graph_id <= 0 {
        return Err("画布 graph_id 必须大于 0".to_string());
    }
    if mode.trim().is_empty() {
        return Err("画布 mode 不能为空".to_string());
    }
    Ok(format!(
        "video_canvas_graph:{graph_id}:{mode}:graph:{unique_suffix}"
    ))
}

/// 画布单节点 exclusive business key。
pub fn canvas_single_node_workflow_business_key(
    graph_id: i64,
    node_id: i64,
    unique_suffix: i64,
) -> Result<String, String> {
    if graph_id <= 0 {
        return Err("画布 graph_id 必须大于 0".to_string());
    }
    if node_id <= 0 {
        return Err("画布 node_id 必须大于 0".to_string());
    }
    Ok(format!(
        "video_canvas_graph:{graph_id}:single_node:{node_id}:{unique_suffix}"
    ))
}

/// 画布节点共享 run exclusive business key（无时间戳，同 step 可续）。
pub fn canvas_node_step_workflow_business_key(
    graph_id: i64,
    mode: &str,
    step_key: &str,
) -> Result<String, String> {
    if graph_id <= 0 {
        return Err("画布 graph_id 必须大于 0".to_string());
    }
    if mode.trim().is_empty() {
        return Err("画布 mode 不能为空".to_string());
    }
    if step_key.trim().is_empty() {
        return Err("画布 step_key 不能为空".to_string());
    }
    Ok(format!("video_canvas_graph:{graph_id}:{mode}:{step_key}"))
}

/// 构造画布整图 begin 输入（status=RUNNING）。
pub fn canvas_graph_workflow_run_input(
    project_id: i64,
    graph_id: i64,
    mode: &str,
    unique_suffix: i64,
    input_fingerprint_json: Value,
    first_step_key: Option<String>,
) -> Result<UpsertWorkflowRunInput, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    Ok(UpsertWorkflowRunInput {
        business_key: canvas_graph_workflow_business_key(graph_id, mode, unique_suffix)?,
        workflow_kind: WORKFLOW_KIND_VIDEO_CANVAS_GRAPH.to_string(),
        project_id: Some(project_id),
        root_project_id: Some(project_id),
        source_snapshot_id: None,
        scope_table: Some("canvas_graph".to_string()),
        scope_id: Some(graph_id),
        chapter_id: None,
        mode: Some(mode.to_string()),
        input_fingerprint_json: Some(input_fingerprint_json),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: first_step_key,
        active_task_id: None,
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    })
}

/// 构造画布单节点 begin 输入（status=RUNNING）。
pub fn canvas_single_node_workflow_run_input(
    project_id: i64,
    graph_id: i64,
    node_id: i64,
    unique_suffix: i64,
    input_fingerprint_json: Value,
    step_key: String,
) -> Result<UpsertWorkflowRunInput, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    Ok(UpsertWorkflowRunInput {
        business_key: canvas_single_node_workflow_business_key(graph_id, node_id, unique_suffix)?,
        workflow_kind: WORKFLOW_KIND_VIDEO_CANVAS_GRAPH.to_string(),
        project_id: Some(project_id),
        root_project_id: Some(project_id),
        source_snapshot_id: None,
        scope_table: Some("canvas_graph".to_string()),
        scope_id: Some(graph_id),
        chapter_id: None,
        mode: Some("single_node".to_string()),
        input_fingerprint_json: Some(input_fingerprint_json),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(step_key),
        active_task_id: None,
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    })
}

/// 构造画布节点独立 run begin 输入（status=RUNNING）。
pub fn canvas_node_step_workflow_run_input(
    project_id: i64,
    graph_id: i64,
    mode: &str,
    step_key: &str,
    input_fingerprint_json: Value,
) -> Result<UpsertWorkflowRunInput, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    Ok(UpsertWorkflowRunInput {
        business_key: canvas_node_step_workflow_business_key(graph_id, mode, step_key)?,
        workflow_kind: WORKFLOW_KIND_VIDEO_CANVAS_GRAPH.to_string(),
        project_id: Some(project_id),
        root_project_id: Some(project_id),
        source_snapshot_id: None,
        scope_table: Some("canvas_graph".to_string()),
        scope_id: Some(graph_id),
        chapter_id: None,
        mode: Some(mode.to_string()),
        input_fingerprint_json: Some(input_fingerprint_json),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(step_key.to_string()),
        active_task_id: None,
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    })
}

/// 画布节点 step_key：`canvas_node:{node_id}`。
pub fn canvas_node_step_key(node_id: i64) -> Result<String, String> {
    if node_id <= 0 {
        return Err("画布 node_id 必须大于 0".to_string());
    }
    Ok(format!("canvas_node:{node_id}"))
}

/// 独立节点 run 成功时是否应 finalize workflow（与 Host CanvasRunHandle 约定一致）。
pub fn canvas_node_should_finalize_on_success(finalize_on_success: bool) -> bool {
    finalize_on_success
}

/// 视频画布节点关联 workflow 的状态推进策略（纯规则）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanvasVideoWorkflowStatusAction {
    /// 标 completed。
    Complete,
    /// 标 failed（可 resume）。
    Fail,
    /// 非终态：按给定 status 回落 mark，can_resume=false。
    MarkNonTerminal,
}

/// 将视频画布 workflow status 解析为推进动作。
pub fn canvas_video_workflow_status_action(
    status: &str,
) -> Result<CanvasVideoWorkflowStatusAction, String> {
    let status = status.trim();
    if status.is_empty() {
        return Err("画布视频 workflow status 不能为空".to_string());
    }
    Ok(match status {
        s if s == WORKFLOW_STATUS_COMPLETED => CanvasVideoWorkflowStatusAction::Complete,
        s if s == WORKFLOW_STATUS_FAILED => CanvasVideoWorkflowStatusAction::Fail,
        s if s == WORKFLOW_STATUS_CANCELLED => CanvasVideoWorkflowStatusAction::MarkNonTerminal,
        _ => CanvasVideoWorkflowStatusAction::MarkNonTerminal,
    })
}

/// 节点成功 finalize 对应的终态 outcome。
pub fn canvas_node_success_finalize_outcome() -> WorkflowRunFinalizeOutcome {
    WorkflowRunFinalizeOutcome::Completed
}

/// checkpoint 是否视为节点/视频画布已完成（completed 或 failed）。
pub fn canvas_workflow_checkpoint_is_terminal(status: &str) -> bool {
    matches!(
        status.trim(),
        s if s == WORKFLOW_STATUS_COMPLETED || s == WORKFLOW_STATUS_FAILED
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_invalid_graph_id() {
        assert!(canvas_graph_workflow_business_key(0, "full", 1).is_err());
    }

    #[test]
    fn builds_graph_running_input() {
        let input = canvas_graph_workflow_run_input(
            2,
            3,
            "full_graph",
            99,
            json!({"a": 1}),
            Some("canvas_node:1".into()),
        )
        .unwrap();
        assert_eq!(input.workflow_kind, WORKFLOW_KIND_VIDEO_CANVAS_GRAPH);
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
        assert!(input.business_key.contains("full_graph"));
    }

    #[test]
    fn node_step_key_requires_positive_id() {
        assert!(canvas_node_step_key(0).is_err());
        assert_eq!(canvas_node_step_key(9).unwrap(), "canvas_node:9");
    }

    #[test]
    fn video_status_action_maps_terminal_and_non_terminal() {
        assert_eq!(
            canvas_video_workflow_status_action(WORKFLOW_STATUS_COMPLETED).unwrap(),
            CanvasVideoWorkflowStatusAction::Complete
        );
        assert_eq!(
            canvas_video_workflow_status_action(WORKFLOW_STATUS_FAILED).unwrap(),
            CanvasVideoWorkflowStatusAction::Fail
        );
        assert_eq!(
            canvas_video_workflow_status_action("interrupted").unwrap(),
            CanvasVideoWorkflowStatusAction::MarkNonTerminal
        );
        assert!(canvas_video_workflow_status_action("  ").is_err());
    }

    #[test]
    fn checkpoint_terminal_predicate() {
        assert!(canvas_workflow_checkpoint_is_terminal(
            WORKFLOW_STATUS_COMPLETED
        ));
        assert!(canvas_workflow_checkpoint_is_terminal(
            WORKFLOW_STATUS_FAILED
        ));
        assert!(!canvas_workflow_checkpoint_is_terminal(
            WORKFLOW_STATUS_RUNNING
        ));
    }
}
