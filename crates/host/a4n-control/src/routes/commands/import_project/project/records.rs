use super::macros::{
    project_web_input_value_command, project_web_project_input_value_command,
    project_web_project_node_value_command, project_web_project_unit_command,
    project_web_project_value_command,
};
use crate::routes::commands::{
    ApiError, nullable_i64_arg, optional_string_field, project_records_actions, to_value, u64_arg,
};
use a4n_api_contract::project::{
    CreateCanvasProjectInput, CreateDerivedProjectInput, CreateProjectInput,
    CreateStandaloneComicProjectInput, SourceSnapshotChapterReadInput, UpdateProjectInput,
};
use serde_json::Value;

pub(in crate::routes::commands) async fn list_projects(
    state: &crate::state::ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let page = args.get("page").map_or(Ok(1), |_| u64_arg(args, "page"))?;
    let page_size = args
        .get("pageSize")
        .map_or(Ok(20), |_| u64_arg(args, "pageSize"))?;
    let project_kind = optional_string_field(args, "projectKind")?;
    let root_project_id = nullable_i64_arg(args, "rootProjectId")?;
    let access_scope = optional_string_field(args, "accessScope")?;
    to_value(
        project_records_actions::list_projects(
            state.runtime(),
            user_id,
            page,
            page_size,
            project_kind.as_deref(),
            root_project_id,
            access_scope.as_deref(),
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

project_web_input_value_command!(
    create_project,
    CreateProjectInput,
    project_records_actions::create_project
);
project_web_input_value_command!(
    create_derived_project,
    CreateDerivedProjectInput,
    project_records_actions::create_derived_project
);
project_web_input_value_command!(
    create_canvas_project,
    CreateCanvasProjectInput,
    project_records_actions::create_canvas_project
);
project_web_input_value_command!(
    create_standalone_comic_project,
    CreateStandaloneComicProjectInput,
    project_records_actions::create_standalone_comic_project
);
project_web_project_value_command!(get_project, project_records_actions::get_project_summary);
project_web_project_value_command!(
    list_source_snapshot_chapters,
    project_records_actions::list_source_snapshot_chapters
);
project_web_project_value_command!(
    list_source_snapshot_chapter_index,
    project_records_actions::list_source_snapshot_chapter_index
);
project_web_project_input_value_command!(
    get_source_snapshot_chapter,
    SourceSnapshotChapterReadInput,
    project_records_actions::get_source_snapshot_chapter
);

project_web_project_input_value_command!(
    update_project,
    UpdateProjectInput,
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
