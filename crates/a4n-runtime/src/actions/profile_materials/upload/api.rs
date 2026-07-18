use super::file::{
    append_cleanup_error, decode_uploaded_profile_material_image,
    persist_uploaded_profile_material_image, remove_uploaded_profile_material_file,
};
use crate::actions::access;
use crate::actions::image_records;
use crate::actions::profile_materials::contracts::{
    IMAGE_TYPE_OBJECT, IMAGE_TYPE_SCENE, KIND_OBJECT, KIND_SCENE, MANUAL_UPLOAD_MODEL,
    ProfileMaterialGroup, UploadProfileMaterialImageInput,
};
use crate::actions::profile_materials::list::{
    object_source_from_model, read_single_profile_material_group, scene_source_from_model,
};
use crate::actions::profile_materials::metadata::{
    ProfileMaterialStyleMetadata, profile_material_metadata,
};
use crate::actions::profile_materials::primary::upsert_primary_material;
use crate::actions::profile_materials::prompts::{
    non_empty, object_profile_image_prompt, scene_profile_image_prompt,
};
use crate::actions::profile_materials::queries::normalize_kind;
use crate::state::AppState;
use a4n_orchestrator::video::CreateGeneratedImageRecordDraftInput as CreateImageRecord;
use chrono::Utc;
use serde_json::json;

pub async fn upload_profile_material_image(
    state: &AppState,
    user_id: i64,
    input: UploadProfileMaterialImageInput,
) -> Result<ProfileMaterialGroup, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let kind = normalize_kind(&input.profile_kind)?;
    let uploaded = decode_uploaded_profile_material_image(&input)?;
    let style = ProfileMaterialStyleMetadata {
        style_template_id: input.style_template_id.clone(),
        style_template_name: input.style_template_name.clone(),
        style_template_version: input.style_template_version.clone(),
        custom_style: input.custom_style.clone(),
    };

    let (source, image_type, prompt) = match kind {
        KIND_SCENE => {
            let profile = state
                .orchestrator
                .scene_profile_by_id(input.profile_id)
                .await
                .map_err(|error| error.to_string())?;
            if profile.project_id != input.project_id {
                return Err(format!(
                    "场景档案 #{} 不属于项目 {}",
                    input.profile_id, input.project_id
                ));
            }
            let prompt = input
                .prompt_snapshot
                .as_deref()
                .and_then(non_empty)
                .map(ToString::to_string)
                .unwrap_or_else(|| scene_profile_image_prompt(&profile, None));
            (scene_source_from_model(profile), IMAGE_TYPE_SCENE, prompt)
        }
        KIND_OBJECT => {
            let profile = state
                .orchestrator
                .object_profile_by_id(input.profile_id)
                .await
                .map_err(crate::actions::app_error_message)?;
            if profile.project_id != input.project_id {
                return Err(format!(
                    "物体档案 #{} 不属于项目 {}",
                    input.profile_id, input.project_id
                ));
            }
            let prompt = input
                .prompt_snapshot
                .as_deref()
                .and_then(non_empty)
                .map(ToString::to_string)
                .unwrap_or_else(|| object_profile_image_prompt(&profile, None));
            (object_source_from_model(profile), IMAGE_TYPE_OBJECT, prompt)
        }
        _ => return Err(format!("不支持的素材档案类型: {kind}")),
    };

    let (path, path_str, file_size) = persist_uploaded_profile_material_image(
        &state.config.data_dir,
        input.project_id,
        kind,
        input.profile_id,
        uploaded.extension,
        &uploaded.bytes,
    )?;

    let mut metadata = profile_material_metadata(
        kind,
        source.profile.id,
        &source.profile.name,
        &source.content_hash,
        &style,
    );
    if let Some(map) = metadata.as_object_mut() {
        map.insert("source".to_string(), json!("manual_upload"));
        map.insert("manual_upload".to_string(), json!(true));
        map.insert("chapter_video_material".to_string(), json!(true));
        map.insert("prompt_snapshot".to_string(), json!(prompt));
        map.insert(
            "upload".to_string(),
            json!({
                "original_file_name": uploaded.source_file_name,
                "mime_type": uploaded.mime_type,
                "file_size": file_size,
            }),
        );
    }

    let now = Utc::now().naive_utc();
    let image = match state
        .orchestrator
        .create_generated_image_record(CreateImageRecord {
            project_id: input.project_id,
            image_type: image_type.to_string(),
            file_path: path_str,
            prompt,
            model: MANUAL_UPLOAD_MODEL.to_string(),
            width: uploaded.width,
            height: uploaded.height,
            file_size: Some(file_size),
            chapter_id: None,
            panel_id: Some(format!("{kind}_profile_{}", input.profile_id)),
            metadata_json: Some(metadata),
            created_at: Some(now),
        })
        .await
    {
        Ok(image) => image,
        Err(err) => {
            let cleanup_error = remove_uploaded_profile_material_file(&path).err();
            return Err(append_cleanup_error(
                format!("素材图记录创建失败：{err}"),
                cleanup_error,
            ));
        }
    };

    if input.set_primary
        && let Err(err) =
            upsert_primary_material(state, input.project_id, kind, input.profile_id, image.id).await
    {
        let cleanup_error = cleanup_uploaded_profile_material_image(state, user_id, image.id)
            .await
            .err();
        return Err(append_cleanup_error(err, cleanup_error));
    }

    read_single_profile_material_group(state, user_id, input.project_id, kind, input.profile_id)
        .await
}

async fn cleanup_uploaded_profile_material_image(
    state: &AppState,
    user_id: i64,
    image_id: i64,
) -> Result<(), String> {
    image_records::delete_image(state, user_id, image_id)
        .await
        .map_err(|err| format!("素材图记录回滚失败：{err}"))
}
