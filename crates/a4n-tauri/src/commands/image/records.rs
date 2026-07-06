use a4n_runtime::AppState;
use a4n_runtime::actions::image_generation::GeneratedImageSummary;
use a4n_runtime::actions::image_records::{
    self as image_records_actions, ListImagesByChapterInput, ListVideoReferenceImagesInput,
    UpdateGeneratedImageInput, UploadImageInput,
};
use a4n_runtime::actions::profile_materials::{
    self as profile_materials_actions, ListProfileMaterialsInput, ProfileMaterialGroup,
    SetProfilePrimaryMaterialInput, UploadProfileMaterialImageInput,
};
use tauri::State;

#[tauri::command]
pub async fn list_images_by_project(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<GeneratedImageSummary>, String> {
    image_records_actions::list_images_by_project(state.inner(), user_id, project_id).await
}

#[tauri::command]
pub async fn list_images_by_chapter(
    state: State<'_, AppState>,
    user_id: i64,
    input: ListImagesByChapterInput,
) -> Result<Vec<GeneratedImageSummary>, String> {
    image_records_actions::list_images_by_chapter(
        state.inner(),
        user_id,
        input.project_id,
        input.chapter_id,
    )
    .await
}

#[tauri::command]
pub async fn list_video_reference_images(
    state: tauri::State<'_, AppState>,
    user_id: i64,
    input: ListVideoReferenceImagesInput,
) -> Result<Vec<GeneratedImageSummary>, String> {
    image_records_actions::list_video_reference_images(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn list_profile_materials(
    state: State<'_, AppState>,
    user_id: i64,
    input: ListProfileMaterialsInput,
) -> Result<Vec<ProfileMaterialGroup>, String> {
    profile_materials_actions::list_profile_materials(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn set_profile_primary_material(
    state: State<'_, AppState>,
    user_id: i64,
    input: SetProfilePrimaryMaterialInput,
) -> Result<ProfileMaterialGroup, String> {
    profile_materials_actions::set_profile_primary_material(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn upload_profile_material_image(
    state: State<'_, AppState>,
    user_id: i64,
    input: UploadProfileMaterialImageInput,
) -> Result<ProfileMaterialGroup, String> {
    profile_materials_actions::upload_profile_material_image(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn toggle_image_favorite(
    state: State<'_, AppState>,
    user_id: i64,
    image_id: i64,
) -> Result<GeneratedImageSummary, String> {
    image_records_actions::toggle_image_favorite(state.inner(), user_id, image_id).await
}

#[tauri::command]
pub async fn update_image(
    state: State<'_, AppState>,
    user_id: i64,
    image_id: i64,
    input: UpdateGeneratedImageInput,
) -> Result<GeneratedImageSummary, String> {
    image_records_actions::update_image(state.inner(), user_id, image_id, input).await
}

#[tauri::command]
pub async fn upload_image(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    input: UploadImageInput,
) -> Result<GeneratedImageSummary, String> {
    image_records_actions::upload_image(state.inner(), user_id, project_id, input).await
}

#[tauri::command]
pub async fn delete_image(
    state: State<'_, AppState>,
    user_id: i64,
    image_id: i64,
) -> Result<(), String> {
    image_records_actions::delete_image(state.inner(), user_id, image_id).await
}

#[tauri::command]
pub async fn score_image(
    state: State<'_, AppState>,
    user_id: i64,
    image_id: i64,
    score: f32,
) -> Result<GeneratedImageSummary, String> {
    image_records_actions::score_image(state.inner(), user_id, image_id, score).await
}
