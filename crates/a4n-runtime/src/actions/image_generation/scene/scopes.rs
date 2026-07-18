use super::super::{
    ImageGenerationTaskScope, ObjectProfileImageInput, SceneProfileImageInput,
    build_object_profile_material, build_scene_profile_material,
};
use crate::AppState;
use crate::actions::access;
use serde_json::Value;

pub async fn scene_image_generation_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ImageGenerationTaskScope, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    Ok(ImageGenerationTaskScope {
        project_id,
        total: 3,
    })
}

pub async fn page_image_generation_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ImageGenerationTaskScope, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    Ok(ImageGenerationTaskScope {
        project_id,
        total: 4,
    })
}

pub async fn batch_image_generation_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    requests_json: &Value,
) -> Result<Option<ImageGenerationTaskScope>, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let requests = requests_json
        .as_array()
        .ok_or_else(|| "requests_json 必须是数组".to_string())?;
    if requests.is_empty() {
        return Ok(None);
    }
    let total = u32::try_from(requests.len())
        .map_err(|_| format!("图片生成请求数量超过 u32 上限：{}", requests.len()))?
        .max(1);
    Ok(Some(ImageGenerationTaskScope { project_id, total }))
}

pub async fn object_profile_image_generation_task_scope(
    state: &AppState,
    user_id: i64,
    input: &ObjectProfileImageInput,
) -> Result<ImageGenerationTaskScope, String> {
    let material = build_object_profile_material(state, user_id, input).await?;
    Ok(ImageGenerationTaskScope {
        project_id: material.project_id,
        total: 3,
    })
}

pub async fn scene_profile_image_generation_task_scope(
    state: &AppState,
    user_id: i64,
    input: &SceneProfileImageInput,
) -> Result<ImageGenerationTaskScope, String> {
    let material = build_scene_profile_material(state, user_id, input).await?;
    Ok(ImageGenerationTaskScope {
        project_id: material.project_id,
        total: 3,
    })
}
