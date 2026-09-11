use super::macros::{project_web_input_summary_command, project_web_input_value_command};
use crate::routes::commands::{ApiError, command_input, project_transfer_actions, to_value};
use crate::state::ServerState;
use serde_json::Value;

project_web_input_value_command!(
    export_project_family_data,
    project_transfer_actions::ExportProjectFamilyDataInput,
    project_transfer_actions::export_project_family_data
);
project_web_input_value_command!(
    export_derived_project_data,
    project_transfer_actions::ExportDerivedProjectDataInput,
    project_transfer_actions::export_derived_project_data
);
project_web_input_value_command!(
    export_video_studio_project_data,
    project_transfer_actions::ExportVideoStudioProjectDataInput,
    project_transfer_actions::export_video_studio_project_data
);

pub(in crate::routes::commands) async fn import_project_family_upload(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: project_transfer_actions::ImportProjectPackageUploadInput =
        command_input(args, "import_project_family_upload")?;
    to_value(
        project_transfer_actions::import_project_family_upload_summaries(
            state.runtime(),
            user_id,
            input,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

project_web_input_summary_command!(
    import_video_studio_project_upload,
    project_transfer_actions::ImportProjectPackageUploadInput,
    project_transfer_actions::import_video_studio_project_upload_summary
);
