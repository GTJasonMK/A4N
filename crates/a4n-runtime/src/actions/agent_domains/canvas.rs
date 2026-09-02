use crate::AppState;
use a4n_api_contract::{
    agent::AgentScope,
    canvas::{
        CanvasGraphDocument, CanvasNodeContract, CanvasRunSummary, GetCanvasGraphInput,
        GetCanvasNodeContractsInput, RunCanvasGraphInput, RunCanvasNodeInput, SaveCanvasGraphInput,
    },
    workflow::{GetWorkflowRunInput, WorkflowRunDetailSummary},
};
use a4n_core::error::AppError;

pub(crate) async fn get_graph(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: GetCanvasGraphInput,
) -> Result<CanvasGraphDocument, AppError> {
    super::super::agent_runtime::ensure_project_scope(scope, input.project_id)?;
    if input.graph_id.is_none() {
        return Err(AppError::InvalidArgument(
            "Agent 工具 get_canvas_graph 必须提供 graphId，避免读取时隐式创建默认画布".into(),
        ));
    }
    crate::actions::canvas_workflow::get_canvas_graph(state, user_id, input)
        .await
        .map_err(AppError::InvalidArgument)
}

pub(crate) async fn get_node_contracts(
    state: &AppState,
    user_id: i64,
    input: GetCanvasNodeContractsInput,
) -> Result<Vec<CanvasNodeContract>, AppError> {
    crate::actions::canvas_workflow::get_canvas_node_contracts(state, user_id, input)
        .await
        .map_err(AppError::InvalidArgument)
}

pub(crate) async fn save_graph(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: SaveCanvasGraphInput,
) -> Result<CanvasGraphDocument, AppError> {
    super::super::agent_runtime::ensure_project_scope(scope, input.project_id)?;
    validate_save_version(&input)?;
    crate::actions::canvas_workflow::save_canvas_graph(state, user_id, input)
        .await
        .map_err(AppError::InvalidArgument)
}

pub(crate) async fn run_graph(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: RunCanvasGraphInput,
) -> Result<CanvasRunSummary, AppError> {
    super::super::agent_runtime::ensure_project_scope(scope, input.project_id)?;
    let result = crate::actions::canvas_workflow::run_canvas_graph(state, user_id, input)
        .await
        .map_err(AppError::InvalidArgument)?;
    ensure_running_workflow(&result)?;
    Ok(result)
}

pub(crate) async fn run_node(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: RunCanvasNodeInput,
) -> Result<CanvasRunSummary, AppError> {
    super::super::agent_runtime::ensure_project_scope(scope, input.project_id)?;
    let result = crate::actions::canvas_workflow::run_canvas_node(state, user_id, input)
        .await
        .map_err(AppError::InvalidArgument)?;
    ensure_running_workflow(&result)?;
    Ok(result)
}

pub(crate) async fn get_workflow_run(
    state: &AppState,
    user_id: i64,
    scope: &AgentScope,
    input: GetWorkflowRunInput,
) -> Result<WorkflowRunDetailSummary, AppError> {
    let detail =
        crate::actions::task_records::get_workflow_run_detail_for_user(state, user_id, input)
            .await
            .map_err(AppError::InvalidArgument)?;
    if let Some(project_id) = detail.run.project_id.or(detail.run.root_project_id) {
        super::super::agent_runtime::ensure_project_scope(scope, project_id)?;
    } else if scope.project_id.is_some() {
        return Err(AppError::Forbidden(
            "项目域 Canvas Agent 不能读取无项目归属的 Workflow".into(),
        ));
    }
    Ok(detail)
}

fn validate_save_version(input: &SaveCanvasGraphInput) -> Result<(), AppError> {
    match (input.graph.id, input.expected_version) {
        (Some(_), None) => Err(AppError::InvalidArgument(
            "Agent 更新画布必须提供 expectedVersion".into(),
        )),
        (None, Some(_)) => Err(AppError::InvalidArgument(
            "Agent 新建画布不能提供 expectedVersion".into(),
        )),
        _ => Ok(()),
    }
}

fn ensure_running_workflow(result: &CanvasRunSummary) -> Result<(), AppError> {
    if result.status == "running" {
        Ok(())
    } else {
        Err(AppError::InvalidState {
            current: result.status.clone(),
            operation: "启动 Canvas Workflow".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_api_contract::canvas::CanvasGraphInput;

    fn graph(id: Option<i64>) -> CanvasGraphInput {
        CanvasGraphInput {
            id,
            name: "Agent graph".into(),
            graph_kind: "video_studio".into(),
            viewport_json: None,
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    #[test]
    fn save_version_contract_distinguishes_create_and_update() {
        assert!(
            validate_save_version(&SaveCanvasGraphInput {
                project_id: 1,
                expected_version: None,
                graph: graph(Some(2)),
            })
            .is_err()
        );
        assert!(
            validate_save_version(&SaveCanvasGraphInput {
                project_id: 1,
                expected_version: Some(1),
                graph: graph(None),
            })
            .is_err()
        );
        assert!(
            validate_save_version(&SaveCanvasGraphInput {
                project_id: 1,
                expected_version: Some(1),
                graph: graph(Some(2)),
            })
            .is_ok()
        );
    }

    #[test]
    fn canvas_run_is_an_async_workflow_handle() {
        let running = CanvasRunSummary {
            workflow_run_id: 9,
            project_id: 1,
            graph_id: 2,
            mode: "full_graph".into(),
            status: "running".into(),
            error_message: None,
        };
        assert!(ensure_running_workflow(&running).is_ok());
        let mut fake_completed = running;
        fake_completed.status = "completed".into();
        assert!(ensure_running_workflow(&fake_completed).is_err());
    }
}
