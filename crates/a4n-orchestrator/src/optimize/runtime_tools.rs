pub(in crate::optimize) mod accounting;
mod agent_context;
mod execution;
mod read;
mod rules;
mod search;
mod specs;

pub(in crate::optimize) use agent_context::{
    inspect_project_context, read_project_context, search_project_context,
};
pub(in crate::optimize) use execution::execute_optimization_runtime_tool;
pub(in crate::optimize) use read::observed_evidence_context;
pub(in crate::optimize) use read::source_hash;
pub(in crate::optimize) use specs::INFORMATION_SOURCES;
pub(crate) use specs::optimization_runtime_tool_specs;
pub(in crate::optimize) use specs::{EDIT_ARTICLE_TOOL, GET_PROJECT_INFORMATION_TOOL};

pub(crate) fn optimization_runtime_tool_specs_for_prompt() -> Vec<serde_json::Value> {
    specs::optimization_runtime_tool_specs_for_prompt()
}

#[cfg(test)]
mod tests;
