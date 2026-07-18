macro_rules! optimize_session_command {
    ($name:ident, $output:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, a4n_runtime::AppState>,
            user_id: i64,
            session_id: i64,
        ) -> Result<$output, String> {
            $action(state.inner(), user_id, session_id).await
        }
    };
}

macro_rules! optimize_input_command {
    ($name:ident, $input:ty, $output:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, a4n_runtime::AppState>,
            user_id: i64,
            input: $input,
        ) -> Result<$output, String> {
            $action(state.inner(), user_id, input).await
        }
    };
}

macro_rules! optimize_preview_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, a4n_runtime::AppState>,
            user_id: i64,
            input: $input,
        ) -> Result<$crate::commands::request_preview::ModelRequestPreview, String> {
            $action(state.inner(), user_id, input).await
        }
    };
}

pub(super) use optimize_input_command;
pub(super) use optimize_preview_command;
pub(super) use optimize_session_command;
