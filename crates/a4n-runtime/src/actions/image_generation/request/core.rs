use super::super::{
    AppState, DirectImageGenerationInput, ImageChapterBindingAccess, ImageGenerationRequest,
    SceneImageRequestContext, resolve_image_chapter_binding, resolve_image_config,
};

pub async fn build_scene_image_request(
    state: &AppState,
    project_id: i64,
    prompt: String,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
    binding_access: ImageChapterBindingAccess,
) -> Result<SceneImageRequestContext, String> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("prompt 不能为空".to_string());
    }
    let (chapter_id, source_metadata) =
        resolve_image_chapter_binding(state, project_id, chapter_id, binding_access).await?;
    let config = resolve_image_config(state, config_id).await?;
    Ok(SceneImageRequestContext {
        request: ImageGenerationRequest {
            prompt: prompt.to_string(),
            width: config.default_width,
            height: config.default_height,
            model: config.model.clone(),
            reference_images: Vec::new(),
        },
        config,
        chapter_id,
        source_metadata,
    })
}

pub async fn build_direct_image_request(
    state: &AppState,
    input: &DirectImageGenerationInput,
    binding_access: ImageChapterBindingAccess,
) -> Result<SceneImageRequestContext, String> {
    let image_type = input.image_type.trim();
    if image_type.is_empty() {
        return Err("image_type 不能为空".to_string());
    }
    let prompt = input.prompt.trim();
    if prompt.is_empty() {
        return Err("prompt 不能为空".to_string());
    }
    let (chapter_id, source_metadata) =
        resolve_image_chapter_binding(state, input.project_id, input.chapter_id, binding_access)
            .await?;
    let config = resolve_image_config(state, input.config_id).await?;
    Ok(SceneImageRequestContext {
        request: ImageGenerationRequest {
            prompt: prompt.to_string(),
            width: config.default_width,
            height: config.default_height,
            model: config.model.clone(),
            reference_images: Vec::new(),
        },
        config,
        chapter_id,
        source_metadata,
    })
}
