mod lifecycle;
mod status;
mod targets;
mod trace;
mod workflow;

pub use lifecycle::{
    record_completed_generation_run, record_completed_generation_run_in_transaction,
    record_completed_run_with_checkpoint_transaction,
};
pub(crate) use lifecycle::{
    record_completed_generation_run_with_checkpoint_in, record_started_generation_run,
};
pub(crate) use status::interrupt_runs_for_target;
pub use status::{cancel_runs_for_target, complete_runs_for_target, fail_runs_for_target};
pub use targets::{delete_generation_run_targets_for, generation_run_exists_for_target_output};
pub use trace::record_llm_call_trace;
pub use workflow::attach_runs_to_workflow_for_target;

#[cfg(test)]
pub(in crate::generation) use trace::usage_trace_fields;
