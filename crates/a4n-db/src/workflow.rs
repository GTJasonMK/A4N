use a4n_core::event::sse::ProgressMeta;
use a4n_core::models::generation::{
    generation_run, workflow_checkpoint, workflow_progress_event, workflow_run,
};
#[cfg(test)]
use a4n_core::models::task::task_queue;
#[cfg(test)]
use a4n_queue::priority::{TaskPriority, TaskStatus, TaskType};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, NotSet,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;
use serde_json::{Value, json};

mod errors;
pub(in crate::workflow) use errors::WorkflowValidationError;
pub use errors::{WorkflowCheckpointContractError, WorkflowReadError, WorkflowWriteError};

mod status;
pub(in crate::workflow) use status::MAX_PERSISTED_PROGRESS_EVENTS;
pub use status::{
    WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING, WORKFLOW_STATUS_SUPERSEDED,
};

mod inputs;
pub use inputs::*;

mod summaries;
pub use summaries::*;

mod validation;
pub(in crate::workflow) use validation::*;

mod runs;
pub use runs::*;

mod tasks;
pub use tasks::*;

mod llm_checkpoint;
pub use llm_checkpoint::*;

// ── LLM call level checkpoint 合同（阶段 11 收口） ──────────────────────────

#[cfg(test)]
mod tests;
