use super::scope::{current_rag_operation_trace_id, rag_operation_timestamp};
use crate::rag_ingest::SceneRagSyncTraceSource;

pub(crate) fn scene_rag_sync_operation_trace_id(
    project_id: i64,
    source: SceneRagSyncTraceSource,
) -> Result<String, String> {
    if project_id <= 0 {
        return Err(format!(
            "Scene RAG sync project_id 必须是正整数，实际为 {project_id}"
        ));
    }
    match source {
        SceneRagSyncTraceSource::WorkflowRun(workflow_run_id) => {
            workflow_rag_operation_trace_id(workflow_run_id)
        }
        SceneRagSyncTraceSource::Independent => Ok(format!(
            "scene_rag_sync:{project_id}:{}",
            rag_operation_timestamp()
        )),
        SceneRagSyncTraceSource::CurrentOperation => {
            current_rag_operation_trace_id().ok_or_else(|| {
                "Scene RAG sync 要求继承当前 operation，但当前不存在 RAG operation trace"
                    .to_string()
            })
        }
    }
}

pub(crate) fn workflow_rag_operation_trace_id(workflow_run_id: i64) -> Result<String, String> {
    if workflow_run_id <= 0 {
        return Err(format!(
            "RAG workflow_run_id 必须是正整数，实际为 {workflow_run_id}"
        ));
    }
    Ok(format!("workflow_run:{workflow_run_id}"))
}
