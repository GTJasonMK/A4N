#[derive(Debug, Clone, Copy)]
pub(in crate::actions::canvas_workflow) struct CanvasRunHandle {
    pub(in crate::actions::canvas_workflow) workflow_run_id: i64,
    pub(in crate::actions::canvas_workflow) finalize_on_success: bool,
}
