use super::super::{
    ApiError, command_input, i64_arg, import_analysis_actions, import_file_actions,
    import_review_actions, to_value,
};
use crate::state::ServerState;
use serde_json::Value;

pub(in crate::routes::commands) async fn import_project_text_upload(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: import_file_actions::ImportProjectTextUploadInput =
        command_input(args, "import_project_text_upload")?;
    to_value(
        import_file_actions::import_project_text_upload(state.runtime(), user_id, input).await?,
    )
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

macro_rules! import_web_input_unit_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(in crate::routes::commands) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            $action(state.runtime(), user_id, input).await?;
            Ok(Value::Null)
        }
    };
}

import_web_input_command!(
    list_import_queue,
    import_review_actions::ImportProjectReviewsInput,
    import_review_actions::list_import_queue,
);
import_web_input_unit_command!(
    delete_import_review,
    import_review_actions::ImportReviewIdInput,
    import_review_actions::delete_import_review,
);
import_web_input_command!(
    accept_import_review,
    import_review_actions::AcceptImportReviewInput,
    import_review_actions::accept_import_review,
);
import_web_input_command!(
    batch_accept_reviews,
    import_review_actions::BatchAcceptImportReviewsInput,
    import_review_actions::batch_accept_import_reviews,
);
import_web_input_command!(
    reject_import_review,
    import_review_actions::ImportReviewIdInput,
    import_review_actions::reject_import_review,
);
import_web_string_project_command!(
    get_project_import_analysis_progress,
    import_analysis_actions::get_project_import_analysis_progress,
);
import_web_string_project_command!(
    run_import_project_analysis,
    import_analysis_actions::run_import_project_analysis,
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
