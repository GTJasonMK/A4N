use super::image_pipeline::{PortraitImageMirror, mirror_or_rollback, persist_portrait_file};
use super::*;

pub async fn upload_portrait(
    state: &AppState,
    user_id: i64,
    input: UploadPortraitInput,
) -> Result<PortraitSummary, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let profile = state
        .orchestrator
        .portrait_prompt_profile_rows(input.project_id, input.profile_id)
        .await
        .map_err(|err| err.to_string())?
        .profile;
    let prompt = required_upload_text(&input.prompt, "立绘描述")?;
    let imported =
        read_uploaded_image_data(&input.file_name, &input.mime_type, &input.data_base64)?;
    let (_path, path_str, file_size) = persist_portrait_file(
        &state.config.data_dir,
        input.project_id,
        input.profile_id,
        &imported.bytes,
        imported.extension,
        "上传立绘图片",
    )?;
    let portrait = match state
        .orchestrator
        .create_character_portrait_for_profile(CharacterPortraitCreateInput {
            profile_id: input.profile_id,
            project_id: input.project_id,
            portrait_type: input.portrait_type.clone(),
            image_path: path_str.clone(),
            prompt: prompt.clone(),
            expression: input.expression.clone(),
            pose: input.pose.clone(),
            is_primary: false,
        })
        .await
    {
        Ok(portrait) => portrait,
        Err(err) => {
            let err = err.to_string();
            let cleanup =
                remove_data_file_in_data_dir(&state.config.data_dir, &path_str, "上传立绘图片")
                    .err()
                    .filter(|value| !value.trim().is_empty());
            return match cleanup {
                Some(cleanup) => Err(format!(
                    "上传立绘记录创建失败：{err}；后续清理也失败：{cleanup}"
                )),
                None => Err(format!("上传立绘记录创建失败：{err}")),
            };
        }
    };

    mirror_or_rollback(
        state,
        &portrait,
        &path_str,
        PortraitImageMirror {
            workflow_run_id: None,
            project_id: input.project_id,
            profile_id: input.profile_id,
            portrait_id: portrait.id,
            character_name: &profile.name,
            portrait_type: &portrait.portrait_type,
            image_path: &path_str,
            prompt: &prompt,
            model: IMPORTED_IMAGE_MODEL,
            width: imported.width,
            height: imported.height,
            file_size,
            expression: portrait.expression.as_deref(),
            pose: portrait.pose.as_deref(),
            variant_kind: Some("manual_upload"),
            auto_generated: false,
            reference_image_ids: &[],
            provider_trace: None,
        },
    )
    .await?;

    if input.set_primary && !portrait.is_primary {
        state
            .orchestrator
            .set_primary_character_portrait_record(portrait.id)
            .await
            .map_err(|err| format!("立绘上传成功，但设置主图失败：{}", err))?;
        let updated = character_portrait_by_id(state, portrait.id).await?;
        return Ok(to_summary(&updated));
    }

    Ok(to_summary(&portrait))
}

pub(super) fn required_upload_text(value: &str, field_name: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field_name}不能为空"));
    }
    Ok(value.to_string())
}
