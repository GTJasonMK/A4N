use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::host_api::portrait_generation as portrait_actions;
use tauri::State;

macro_rules! portrait_preview_command {
    (
        $name:ident,
        ($($field:ident : $field_ty:ty),* $(,)?),
        $input:expr,
        $action:path $(,)?
    ) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, AppState>,
            user_id: i64,
            $($field: $field_ty,)*
        ) -> Result<ModelRequestPreview, String> {
            $action(state.inner(), user_id, $input).await
        }
    };
}

portrait_preview_command!(
    preview_generate_portrait_request,
    (
        project_id: i64,
        character_name: String,
        style: Option<String>,
        expressions: Option<Vec<String>>,
        pose: Option<String>,
        reference_image_ids: Option<Vec<i64>>,
    ),
    portrait_actions::GeneratePortraitInput {
        project_id,
        character_name,
        style,
        expressions,
        pose,
        reference_image_ids: reference_image_ids.unwrap_or_default(),
    },
    portrait_actions::preview_generate_portrait_request,
);

portrait_preview_command!(
    preview_regenerate_portrait_request,
    (
        portrait_id: i64,
        reference_image_ids: Option<Vec<i64>>,
    ),
    portrait_actions::RegeneratePortraitInput {
        portrait_id,
        reference_image_ids: reference_image_ids.unwrap_or_default(),
    },
    portrait_actions::preview_regenerate_portrait_request,
);

portrait_preview_command!(
    preview_generate_age_variant_request,
    (
        portrait_id: i64,
        age: String,
        reference_image_ids: Option<Vec<i64>>,
    ),
    portrait_actions::GenerateAgeVariantInput {
        portrait_id,
        age,
        reference_image_ids: reference_image_ids.unwrap_or_default(),
    },
    portrait_actions::preview_generate_age_variant_request,
);

portrait_preview_command!(
    preview_generate_missing_portraits_request,
    (project_id: i64),
    portrait_actions::GenerateMissingPortraitsInput { project_id },
    portrait_actions::preview_generate_missing_portraits_request,
);
