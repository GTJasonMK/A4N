mod pipeline;
mod query;
mod status;

pub(super) use query::galgame_project_progress_by_project;

const GALGAME_PIPELINE_WORKFLOW_KIND: &str = "galgame_pipeline";
const WORKFLOW_STATUS_RUNNING: &str = "running";
const WORKFLOW_STATUS_FAILED: &str = "failed";
const WORKFLOW_STATUS_CANCELLED: &str = "cancelled";

#[cfg(test)]
mod tests;
