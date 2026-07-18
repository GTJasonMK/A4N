use super::*;

pub async fn generate_object_profile_image_prompt(
    state: &AppState,
    user_id: i64,
    input: ObjectProfileImageInput,
) -> Result<ObjectProfileImagePromptOutput, String> {
    let material = build_object_profile_material(state, user_id, &input).await?;
    Ok(ObjectProfileImagePromptOutput {
        object_profile_id: material.profile_id,
        object_name: material.name,
        prompt: material.prompt,
    })
}

pub async fn generate_object_profile_image(
    state: &AppState,
    user_id: i64,
    input: ObjectProfileImageInput,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<GeneratedImageSummary, String> {
    let direct = profile_material_direct_input(
        build_object_profile_material(state, user_id, &input).await?,
        input.config_id,
        input.chapter_id,
    );
    generate_direct_image(state, user_id, direct, progress).await
}

pub async fn preview_object_profile_image_request(
    state: &AppState,
    user_id: i64,
    input: ObjectProfileImageInput,
) -> Result<ModelRequestPreview, String> {
    let direct = profile_material_direct_input(
        build_object_profile_material(state, user_id, &input).await?,
        input.config_id,
        input.chapter_id,
    );
    preview_direct_image_request(state, user_id, direct).await
}

pub async fn generate_scene_profile_image_prompt(
    state: &AppState,
    user_id: i64,
    input: SceneProfileImageInput,
) -> Result<SceneProfileImagePromptOutput, String> {
    let material = build_scene_profile_material(state, user_id, &input).await?;
    Ok(SceneProfileImagePromptOutput {
        scene_profile_id: material.profile_id,
        scene_name: material.name,
        prompt: material.prompt,
    })
}

pub async fn generate_scene_profile_image(
    state: &AppState,
    user_id: i64,
    input: SceneProfileImageInput,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<GeneratedImageSummary, String> {
    let direct = profile_material_direct_input(
        build_scene_profile_material(state, user_id, &input).await?,
        input.config_id,
        input.chapter_id,
    );
    generate_direct_image(state, user_id, direct, progress).await
}

pub async fn preview_page_image_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    page_number: i32,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let context = build_page_image_request(state, project_id, chapter_number, page_number).await?;
    Ok(model_request_preview(
        "manga_page_image_generation",
        "整页漫画图片模型请求预览",
        vec![
            image_generation_preview_step(
                state,
                project_id,
                "page_image",
                format!("第 {chapter_number} 章第 {page_number} 页 provider 请求"),
                Some(MANGA_PAGE_PROMPT_TYPE.to_string()),
                Some(format!(
                    "参考图数量：{}；参考图模式：{}",
                    context.reference_image_count, context.reference_image_mode
                )),
                &context.config,
                context.request,
            )
            .await?,
        ],
    ))
}

pub async fn preview_batch_images_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    requests_json: Value,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let arr = requests_json
        .as_array()
        .ok_or_else(|| "requests_json 必须是数组".to_string())?;
    let jobs = build_batch_image_jobs(state, project_id, arr, false).await?;
    let mut steps = Vec::with_capacity(jobs.len());
    for job in jobs {
        steps.push(
            image_generation_preview_step(
                state,
                project_id,
                format!("batch_image_{}", job.index + 1),
                format!("批量图片 {}/{} provider 请求", job.index + 1, arr.len()),
                None,
                Some(batch_image_step_note(&job.image_type, &job.preview_notes)),
                &job.config,
                job.request,
            )
            .await?,
        );
    }
    Ok(model_request_preview(
        "batch_image_generation",
        "批量图片模型请求预览",
        steps,
    ))
}
