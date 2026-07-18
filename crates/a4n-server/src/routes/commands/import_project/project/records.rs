use super::macros::{
    project_web_project_input_value_command, project_web_project_node_value_command,
    project_web_project_unit_command, project_web_project_value_command,
};
use crate::routes::commands::project_records_actions;

project_web_project_input_value_command!(
    update_project,
    project_records_actions::UpdateProjectInput,
    project_records_actions::update_project
);
project_web_project_value_command!(
    get_project_asset_dependency_graph,
    project_records_actions::get_project_asset_dependency_graph
);
project_web_project_node_value_command!(
    get_project_asset_dependency_graph_node_detail,
    project_records_actions::get_project_asset_dependency_graph_node_detail
);
project_web_project_unit_command!(delete_project, project_records_actions::delete_project);
project_web_project_value_command!(
    reset_project_generated_data,
    project_records_actions::reset_project_generated_data
);
