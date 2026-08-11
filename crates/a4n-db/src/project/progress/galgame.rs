mod query;
mod status;

pub(super) use query::galgame_project_progress_by_project;

const GALGAME_PIPELINE_WORKFLOW_KIND: &str = "galgame_pipeline";

#[cfg(test)]
mod tests;
