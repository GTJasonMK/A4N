//! Typed commands owned by the Canvas graph-record worker.

pub use a4n_api_contract::canvas::*;
pub use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};

pub const CANVAS_RECORD_WORKER_ID: &str = "canvas-record-worker";
pub const CANVAS_RECORD_WORKER_CONTRACT_VERSION: &str = "canvas-records-v1";
pub const CANVAS_GRAPH_READ_CAPABILITY: &str = "canvas.graph.read";
pub const CANVAS_GRAPH_MUTATION_CAPABILITY: &str = "canvas.graph.mutation";
pub const CANVAS_GRAPH_GET_OPERATION: &str = "canvas.graph.get";
pub const CANVAS_GRAPH_READ_OPERATION: &str = "canvas.graph.read";
pub const CANVAS_GRAPH_SAVE_OPERATION: &str = "canvas.graph.save";
pub const CANVAS_GRAPH_EDIT_OPERATION: &str = "canvas.graph.edit";
pub const CANVAS_TEXT_ASSET_SAVE_OPERATION: &str = "canvas.text_asset.save";
