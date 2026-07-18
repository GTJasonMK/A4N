use super::macros::{
    project_web_project_string_value_command, project_web_project_value_command,
    project_web_status_command,
};
use crate::routes::commands::project_status_actions;

project_web_status_command!(
    forward_project_status,
    project_status_actions::forward_project_status
);
project_web_status_command!(
    rollback_project_status,
    project_status_actions::rollback_project_status
);
project_web_project_string_value_command!(
    preview_rollback,
    "targetStatus",
    project_status_actions::preview_rollback
);
project_web_project_value_command!(
    get_project_status_history,
    project_status_actions::get_project_status_history
);
