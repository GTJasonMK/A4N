use super::contracts::{
    KIND_OBJECT, KIND_SCENE, ListProfileMaterialsInput, ProfileMaterialGroup,
    SetProfilePrimaryMaterialInput,
};
use super::list::list_profile_materials;
use super::queries::{material_identity, normalize_kind};
use crate::AppState;
use crate::actions::{access, app_error_message};

pub async fn set_profile_primary_material(
    state: &AppState,
    user_id: i64,
    input: SetProfilePrimaryMaterialInput,
) -> Result<ProfileMaterialGroup, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let kind = normalize_kind(&input.profile_kind)?;
    ensure_profile_exists(state, input.project_id, kind, input.profile_id).await?;

    if let Some(image_id) = input.image_id {
        let image = state
            .orchestrator
            .generated_image_by_id(image_id)
            .await
            .map_err(app_error_message)?
            .ok_or_else(|| "图片不存在".to_string())?;
        if image.project_id != input.project_id {
            return Err(format!("图片 #{image_id} 不属于项目 {}", input.project_id));
        }
        if material_identity(&image) != Some((kind, input.profile_id)) {
            return Err(format!(
                "图片 #{image_id} 不是 {} 档案 #{} 的素材图",
                kind, input.profile_id
            ));
        }
        upsert_primary_material(state, input.project_id, kind, input.profile_id, image_id).await?;
    } else {
        state
            .orchestrator
            .delete_profile_primary_material(input.project_id, kind, input.profile_id)
            .await
            .map_err(|error| error.to_string())?;
    }

    let groups = list_profile_materials(
        state,
        user_id,
        ListProfileMaterialsInput {
            project_id: input.project_id,
            kind: Some(kind.to_string()),
            query: None,
            filter: None,
            sort: None,
            image_limit_per_profile: Some(32),
        },
    )
    .await?;
    groups
        .into_iter()
        .find(|group| group.profile.id == input.profile_id)
        .ok_or_else(|| "主素材已更新，但无法重新读取档案素材".to_string())
}

pub(super) async fn ensure_profile_exists(
    state: &AppState,
    project_id: i64,
    kind: &str,
    profile_id: i64,
) -> Result<(), String> {
    match kind {
        KIND_SCENE => {
            let profile = state
                .orchestrator
                .scene_profile_by_id(profile_id)
                .await
                .map_err(|error| error.to_string())?;
            if profile.project_id != project_id {
                return Err(format!("场景档案 #{profile_id} 不属于项目 {project_id}"));
            }
        }
        KIND_OBJECT => {
            let profile = state
                .orchestrator
                .object_profile_by_id(profile_id)
                .await
                .map_err(crate::actions::app_error_message)?;
            if profile.project_id != project_id {
                return Err(format!("物体档案 #{profile_id} 不属于项目 {project_id}"));
            }
        }
        _ => return Err(format!("不支持的素材档案类型: {kind}")),
    }
    Ok(())
}

pub(super) async fn upsert_primary_material(
    state: &AppState,
    project_id: i64,
    kind: &str,
    profile_id: i64,
    image_id: i64,
) -> Result<(), String> {
    state
        .orchestrator
        .upsert_profile_primary_material(project_id, kind, profile_id, image_id)
        .await
        .map_err(|error| error.to_string())
}
