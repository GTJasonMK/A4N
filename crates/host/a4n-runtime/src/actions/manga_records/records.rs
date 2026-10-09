use crate::{AppState, actions::app_error_message};
use a4n_api_contract::manga::{
    BindMangaReferenceImageInput, DeleteMangaPageInput, DeleteMangaPipelineArtifactInput,
    DeleteMangaPromptInput, MangaChapterInput, MangaPagePromptSummary,
    MangaPipelineArtifactDeletionPreview, PreviewMangaPipelineArtifactDeletionInput,
    SaveMangaPipelineArtifactInput, UpdateMangaPromptInput,
};

pub async fn list_manga_page_prompts(
    state: &AppState,
    user_id: i64,
    input: MangaChapterInput,
) -> Result<Vec<MangaPagePromptSummary>, String> {
    state
        .manga_record_read_application()
        .list_page_prompts(user_id, input.project_id, input.chapter_id)
        .await
        .map_err(app_error_message)
}

pub async fn list_manga_pipeline_artifacts(
    state: &AppState,
    user_id: i64,
    input: MangaChapterInput,
) -> Result<Vec<a4n_domain::manga::MangaPipelineArtifactRecord>, String> {
    state
        .manga_pipeline_artifact_application()
        .list_artifacts(user_id, input.project_id, input.chapter_id)
        .await
        .map_err(app_error_message)
}

pub async fn save_manga_pipeline_artifact(
    state: &AppState,
    user_id: i64,
    input: SaveMangaPipelineArtifactInput,
) -> Result<a4n_domain::manga::MangaPipelineArtifactRecord, String> {
    state
        .manga_pipeline_artifact_application()
        .save_artifact(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn preview_delete_manga_pipeline_artifact(
    state: &AppState,
    user_id: i64,
    input: PreviewMangaPipelineArtifactDeletionInput,
) -> Result<MangaPipelineArtifactDeletionPreview, String> {
    state
        .manga_pipeline_artifact_application()
        .preview_artifact_deletion(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn delete_manga_pipeline_artifact(
    state: &AppState,
    user_id: i64,
    input: DeleteMangaPipelineArtifactInput,
) -> Result<(), String> {
    state
        .manga_pipeline_artifact_application()
        .delete_artifact(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn bind_manga_reference_image(
    state: &AppState,
    user_id: i64,
    input: BindMangaReferenceImageInput,
) -> Result<MangaPagePromptSummary, String> {
    state
        .manga_record_mutation_application()
        .bind_reference_image(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn update_manga_prompt(
    state: &AppState,
    user_id: i64,
    input: UpdateMangaPromptInput,
) -> Result<MangaPagePromptSummary, String> {
    state
        .manga_record_mutation_application()
        .update_prompt(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn delete_manga_by_chapter(
    state: &AppState,
    user_id: i64,
    input: MangaChapterInput,
) -> Result<(), String> {
    state
        .manga_record_mutation_application()
        .delete_by_chapter(user_id, input.project_id, input.chapter_id)
        .await
        .map_err(app_error_message)
}

pub async fn delete_manga_page(
    state: &AppState,
    user_id: i64,
    input: DeleteMangaPageInput,
) -> Result<(), String> {
    state
        .manga_record_mutation_application()
        .delete_page(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn delete_manga_prompt(
    state: &AppState,
    user_id: i64,
    input: DeleteMangaPromptInput,
) -> Result<(), String> {
    state
        .manga_record_mutation_application()
        .delete_prompt(user_id, input)
        .await
        .map_err(app_error_message)
}
