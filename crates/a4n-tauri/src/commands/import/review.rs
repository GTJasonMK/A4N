use a4n_runtime::AppState;
use a4n_runtime::host_api::import_review;
use import_review::{
    BatchAcceptImportReviewsInput as BatchAcceptInput, CheckImportDedupInput, DedupResult,
    ImportReviewSummary, ModifyImportReviewInput,
};
use tauri::State;

macro_rules! import_review_id_command {
    ($name:ident, $output:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, AppState>,
            user_id: i64,
            review_id: i64,
        ) -> Result<$output, String> {
            $action(&state, user_id, review_id)
                .await
                .map_err(|e| e.to_string())
        }
    };
}

macro_rules! import_review_id_input_command {
    ($name:ident, $input:ty, $output:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, AppState>,
            user_id: i64,
            review_id: i64,
            input: $input,
        ) -> Result<$output, String> {
            $action(&state, user_id, review_id, input)
                .await
                .map_err(|e| e.to_string())
        }
    };
}

macro_rules! import_review_input_command {
    ($name:ident, $input:ty, $output:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, AppState>,
            user_id: i64,
            input: $input,
        ) -> Result<$output, String> {
            $action(&state, user_id, input)
                .await
                .map_err(|e| e.to_string())
        }
    };
}

import_review_id_command!(
    get_import_review,
    ImportReviewSummary,
    import_review::get_import_review,
);
import_review_id_command!(
    accept_import_review,
    ImportReviewSummary,
    import_review::accept_import_review,
);
import_review_id_input_command!(
    modify_import_review,
    ModifyImportReviewInput,
    ImportReviewSummary,
    import_review::modify_import_review,
);
import_review_id_command!(
    reject_import_review,
    ImportReviewSummary,
    import_review::reject_import_review,
);
import_review_input_command!(
    batch_accept_reviews,
    BatchAcceptInput,
    Vec<ImportReviewSummary>,
    import_review::batch_accept_import_reviews,
);
import_review_input_command!(
    check_import_dedup,
    CheckImportDedupInput,
    DedupResult,
    import_review::check_import_dedup,
);
