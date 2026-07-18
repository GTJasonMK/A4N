mod checkpoints;
mod finalize;
mod fingerprints;
mod start;

use serde_json::Value;

pub(super) use self::checkpoints::{
    load_completed_image_checkpoint, save_completed_batch_image_checkpoint,
    save_completed_image_checkpoint,
};
pub(super) use self::finalize::{finalize_batch_image_workflow, finalize_single_image_workflow};
pub(super) use self::fingerprints::{batch_image_item_fingerprint, batch_image_item_step_key};
pub(super) use self::start::{
    attach_image_generation_workflow, start_batch_image_workflow, start_image_generation_workflow,
};

pub(super) struct ImageGenerationWorkflowRun {
    pub(super) workflow_run_id: i64,
    pub(super) step_key: String,
    pub(super) fingerprint_json: Value,
}
