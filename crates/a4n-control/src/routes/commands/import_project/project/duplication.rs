use super::macros::project_web_input_summary_command;

project_web_input_summary_command!(
    duplicate_project,
    a4n_runtime::host_api::project_duplication::DuplicateProjectInput,
    a4n_runtime::host_api::project_duplication::duplicate_project_summary
);
