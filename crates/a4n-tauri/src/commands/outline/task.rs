macro_rules! outline_preview_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        #[tauri::command]
        pub async fn $name(
            state: tauri::State<'_, a4n_runtime::AppState>,
            user_id: i64,
            input: $input,
        ) -> Result<crate::commands::request_preview::ModelRequestPreview, String> {
            $action(state.inner(), user_id, input)
                .await
                .map_err(|e| e.to_string())
        }
    };
}

mod chapters;
mod parts;

pub use chapters::*;
pub use parts::*;
