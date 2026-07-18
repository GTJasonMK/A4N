use super::super::super::node_lifecycle::CanvasRunHandle;
use serde_json::Value;

pub(super) struct CanvasVideoNodeContext {
    pub(super) project_id: i64,
    pub(super) graph_id: i64,
    pub(super) mode: String,
    pub(super) step_key: String,
    pub(super) input_json: Value,
    pub(super) run: CanvasRunHandle,
    pub(super) generation_spec_id: Option<i64>,
}
