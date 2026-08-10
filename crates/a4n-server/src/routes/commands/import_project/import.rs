use super::super::{
    ApiError, command_input, i64_arg, import_analysis_actions, import_review_actions, to_value,
};
use crate::state::ServerState;
use serde_json::{Value, json};

pub(in crate::routes::commands) async fn import_uploaded_file(
    _state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: a4n_runtime::host_api::import_file::ImportUploadedFileInput =
        command_input(args, "import_uploaded_file")?;
    let result = a4n_runtime::host_api::import_file::import_uploaded_file(input)
        .map_err(ApiError::bad_request)?;
    Ok(json!(result))
}

macro_rules! import_web_project_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let project_id = i64_arg(args, "projectId")?;
            to_value($action(state.runtime(), user_id, project_id).await?)
        }
    };
}

macro_rules! import_web_string_project_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let project_id = i64_arg(args, "projectId")?;
            to_value(
                $action(state.runtime(), user_id, project_id)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! import_web_review_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let review_id = i64_arg(args, "reviewId")?;
            to_value($action(state.runtime(), user_id, review_id).await?)
        }
    };
}

macro_rules! import_web_review_unit_command {
    ($name:ident, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let review_id = i64_arg(args, "reviewId")?;
            $action(state.runtime(), user_id, review_id).await?;
            Ok(Value::Null)
        }
    };
}

macro_rules! import_web_review_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let review_id = i64_arg(args, "reviewId")?;
            let input: $input = command_input(args, stringify!($name))?;
            to_value($action(state.runtime(), user_id, review_id, input).await?)
        }
    };
}

macro_rules! import_web_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value($action(state.runtime(), user_id, input).await?)
        }
    };
}

import_web_project_command!(list_import_queue, import_review_actions::list_import_queue);
import_web_review_input_command!(
    update_import_status,
    import_review_actions::UpdateImportStatusInput,
    import_review_actions::update_import_status,
);
import_web_review_unit_command!(
    delete_import_review,
    import_review_actions::delete_import_review,
);
import_web_review_command!(get_import_review, import_review_actions::get_import_review);
import_web_review_command!(
    accept_import_review,
    import_review_actions::accept_import_review,
);
import_web_review_input_command!(
    modify_import_review,
    import_review_actions::ModifyImportReviewInput,
    import_review_actions::modify_import_review,
);
import_web_input_command!(
    batch_accept_reviews,
    import_review_actions::BatchAcceptImportReviewsInput,
    import_review_actions::batch_accept_import_reviews,
);
import_web_review_command!(
    reject_import_review,
    import_review_actions::reject_import_review,
);

pub(in crate::routes::commands) async fn start_import_analysis(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let input: import_analysis_actions::StartImportAnalysisInput =
        command_input(args, "start_import_analysis")?;
    let reviews =
        import_analysis_actions::start_import_analysis(state.runtime(), user_id, project_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    let summaries = reviews
        .iter()
        .map(import_review_actions::import_review_summary)
        .collect::<Result<Vec<_>, _>>()?;
    to_value(summaries)
}

import_web_project_command!(
    get_import_analysis_status,
    import_review_actions::get_import_analysis_status,
);
import_web_string_project_command!(
    get_project_import_analysis_progress,
    import_analysis_actions::get_project_import_analysis_progress,
);
import_web_string_project_command!(
    run_import_project_analysis,
    import_analysis_actions::run_import_project_analysis,
);
import_web_string_project_command!(
    apply_import_analysis_blueprint_candidate,
    import_analysis_actions::apply_import_analysis_blueprint_candidate,
);
import_web_string_project_command!(
    apply_import_analysis_part_outline_candidate,
    import_analysis_actions::apply_import_analysis_part_outline_candidate,
);

pub(in crate::routes::commands) async fn preview_import_project_analysis_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let preview = import_analysis_actions::preview_import_project_analysis_request(
        state.runtime(),
        user_id,
        project_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

import_web_string_project_command!(
    cancel_import_analysis,
    import_analysis_actions::cancel_import_analysis,
);
import_web_input_command!(
    check_import_dedup,
    import_review_actions::CheckImportDedupInput,
    import_review_actions::check_import_dedup,
);
