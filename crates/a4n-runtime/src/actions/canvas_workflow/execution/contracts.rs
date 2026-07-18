#[derive(Debug, Clone)]
pub(super) struct PreparedCanvasGraphRun {
    pub(super) workflow_run_id: i64,
    pub(super) project_id: i64,
    pub(super) graph_id: i64,
    pub(super) mode: String,
    pub(super) runnable_node_ids: Vec<i64>,
}
