use super::*;

pub async fn preview_scene_image_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    prompt: String,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let context = build_scene_image_request(
        state,
        project_id,
        prompt,
        config_id,
        chapter_id,
        ImageChapterBindingAccess::ReadOnly,
    )
    .await?;
    Ok(model_request_preview(
        "image_scene_generation",
        "场景图模型请求预览",
        vec![
            image_generation_preview_step(
                state,
                project_id,
                "scene_image",
                "场景图 provider 请求",
                None,
                None,
                &context.config,
                context.request,
            )
            .await?,
        ],
    ))
}

pub async fn preview_canvas_image_request(
    state: &AppState,
    user_id: i64,
    input: GenerateCanvasImageInput,
) -> Result<ModelRequestPreview, String> {
    let direct = canvas_image_direct_input(input);
    preview_direct_image_request(state, user_id, direct).await
}

pub async fn generate_canvas_image(
    state: &AppState,
    user_id: i64,
    input: GenerateCanvasImageInput,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<GeneratedImageSummary, String> {
    let direct = canvas_image_direct_input(input);
    generate_direct_image(state, user_id, direct, progress).await
}

pub async fn generate_galgame_image(
    state: &AppState,
    user_id: i64,
    input: GenerateGalgameImageInput,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<GeneratedImageSummary, String> {
    if input.generation_spec_id <= 0 || input.target_entity_id <= 0 {
        return Err("generation_spec_id 和 target_entity_id 必须是正整数".to_string());
    }
    let metadata_json = Some(json!({
        "galgame": {
            "generation_spec_id": input.generation_spec_id,
            "target_kind": input.target_kind,
            "target_entity_kind": input.target_entity_kind,
            "target_entity_id": input.target_entity_id,
            "workflow_run_id": input.workflow_run_id,
        },
        "source": "galgame_asset_generation",
    }));
    let direct = DirectImageGenerationInput {
        project_id: input.project_id,
        prompt: input.prompt,
        image_type: input.image_type,
        config_id: input.config_id,
        chapter_id: input.chapter_id,
        panel_id: Some(format!(
            "galgame_{}_{}",
            input.target_entity_kind, input.target_entity_id
        )),
        metadata_json,
        workflow_run_id: input.workflow_run_id,
        command_name: "generate_galgame_image".to_string(),
        preview_workflow: "galgame_image_generation".to_string(),
        preview_title: "Galgame 图片模型请求预览".to_string(),
        preview_step_key: "galgame_image".to_string(),
        preview_step_title: "Galgame 图片 provider 请求".to_string(),
    };
    generate_direct_image(state, user_id, direct, progress).await
}

pub(super) fn canvas_image_direct_input(
    input: GenerateCanvasImageInput,
) -> DirectImageGenerationInput {
    let metadata_json = Some(json!({
        "canvas": {
            "graph_id": input.graph_id,
            "node_id": input.node_id,
            "workflow_run_id": input.workflow_run_id,
            "source_node_ids": input.source_node_ids,
        },
        "source": "video_canvas",
    }));
    DirectImageGenerationInput {
        project_id: input.project_id,
        prompt: input.prompt,
        image_type: "canvas_image".to_string(),
        config_id: input.config_id,
        chapter_id: None,
        panel_id: None,
        metadata_json,
        workflow_run_id: input.workflow_run_id,
        command_name: "generate_canvas_image".to_string(),
        preview_workflow: "canvas_image_generation".to_string(),
        preview_title: "画布图片模型请求预览".to_string(),
        preview_step_key: "canvas_image".to_string(),
        preview_step_title: "画布图片 provider 请求".to_string(),
    }
}

pub(super) async fn preview_direct_image_request(
    state: &AppState,
    user_id: i64,
    input: DirectImageGenerationInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let context =
        build_direct_image_request(state, &input, ImageChapterBindingAccess::ReadOnly).await?;
    Ok(model_request_preview(
        input.preview_workflow,
        input.preview_title,
        vec![
            image_generation_preview_step(
                state,
                input.project_id,
                input.preview_step_key,
                input.preview_step_title,
                None,
                Some(format!("image_type: {}", input.image_type)),
                &context.config,
                context.request,
            )
            .await?,
        ],
    ))
}
