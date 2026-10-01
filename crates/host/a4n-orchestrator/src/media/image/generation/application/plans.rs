pub fn normalize_image_metadata(metadata_json: Option<Value>) -> Option<Value> {
    match metadata_json {
        Some(Value::Object(mut map)) => {
            map.entry("metadata_schema_version".to_string())
                .or_insert_with(|| json!(IMAGE_METADATA_SCHEMA_VERSION));
            Some(Value::Object(map))
        }
        Some(value) => Some(json!({
            "metadata_schema_version": IMAGE_METADATA_SCHEMA_VERSION,
            "value": value,
        })),
        None => Some(json!({
            "metadata_schema_version": IMAGE_METADATA_SCHEMA_VERSION,
        })),
    }
}

fn scene_image_plan(input: GenerateSceneImageInput) -> DirectImagePlan {
    DirectImagePlan {
        project_id: input.project_id,
        prompt: input.prompt,
        image_type: "scene".to_string(),
        config_id: input.config_id,
        width: input.width,
        height: input.height,
        chapter_id: input.chapter_id,
        source_snapshot_id: None,
        panel_id: None,
        metadata_json: None,
        workflow_run_id: None,
        command: "image_scene".to_string(),
        file_extra: None,
        file_context: "图片文件".to_string(),
    }
}

fn canvas_image_plan(input: GenerateCanvasImageInput) -> DirectImagePlan {
    let metadata_json = Some(json!({
        "canvas": {
            "graph_id": input.graph_id,
            "node_id": input.node_id,
            "workflow_run_id": input.workflow_run_id,
            "source_node_ids": input.source_node_ids,
        },
        "source": "video_canvas",
    }));
    DirectImagePlan {
        project_id: input.project_id,
        prompt: input.prompt,
        image_type: IMAGE_TYPE_CANVAS.to_string(),
        config_id: input.config_id,
        width: input.width,
        height: input.height,
        chapter_id: None,
        source_snapshot_id: None,
        panel_id: None,
        metadata_json,
        workflow_run_id: input.workflow_run_id,
        command: "generate_canvas_image".to_string(),
        file_extra: None,
        file_context: "图片文件".to_string(),
    }
}

fn galgame_image_plan(input: GenerateGalgameImageInput) -> DirectImagePlan {
    let panel_id = format!(
        "galgame_{}_{}",
        input.target_entity_kind, input.target_entity_id
    );
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
    DirectImagePlan {
        project_id: input.project_id,
        prompt: input.prompt,
        image_type: input.image_type,
        config_id: input.config_id,
        width: input.width,
        height: input.height,
        chapter_id: input.chapter_id,
        source_snapshot_id: Some(input.source_snapshot_id),
        panel_id: Some(panel_id.clone()),
        metadata_json,
        workflow_run_id: input.workflow_run_id,
        command: "generate_galgame_image".to_string(),
        file_extra: Some(panel_id),
        file_context: "图片文件".to_string(),
    }
}

fn profile_image_plan(
    material: ProfileImageMaterial,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
    width: i32,
    height: i32,
) -> DirectImagePlan {
    let panel_id = format!(
        "{}_profile_{}",
        material.spec.profile_kind.as_str(),
        material.profile_id
    );
    DirectImagePlan {
        project_id: material.project_id,
        prompt: material.prompt.clone(),
        image_type: material.spec.image_type.to_string(),
        config_id,
        width,
        height,
        chapter_id,
        source_snapshot_id: None,
        panel_id: Some(panel_id.clone()),
        metadata_json: Some(profile_image_metadata(&material)),
        workflow_run_id: None,
        command: material.spec.command.to_string(),
        file_extra: Some(panel_id),
        file_context: "图片文件".to_string(),
    }
}

fn object_profile_material(
    profile: object_profile::Model,
    input: &ObjectProfileImageInput,
) -> ProfileImageMaterial {
    let prompt = object_profile_image_prompt(&profile, input.style.as_deref());
    let source_material = profile_source_material(
        &[
            profile.name.as_str(),
            profile.visual_description.as_str(),
            profile.material_and_state.as_str(),
            profile.story_usage.as_str(),
        ],
        &prompt,
    );
    let profile_content = object_profile_content(&profile);
    ProfileImageMaterial {
        spec: OBJECT_PROFILE_IMAGE_SPEC,
        profile_id: profile.id,
        project_id: profile.project_id,
        name: profile.name,
        prompt,
        profile_content,
        source_material,
        style_metadata: profile_style_metadata(
            input.style_template_id.clone(),
            input.style_template_name.clone(),
            input.style_template_version.clone(),
            input.custom_style.clone(),
        ),
    }
}

fn scene_profile_material(
    profile: a4n_domain::scene_profile::SceneProfileRecord,
    input: &SceneProfileImageInput,
) -> ProfileImageMaterial {
    let prompt = scene_profile_image_prompt(&profile, input.style.as_deref());
    let source_material = profile_source_material(
        &[
            profile.name.as_str(),
            profile.base_description.as_str(),
            profile.visual_description.as_str(),
            profile.spatial_layout.as_str(),
            profile.atmosphere.as_str(),
            profile.image_prompt_seed.as_str(),
        ],
        &prompt,
    );
    let profile_content = scene_profile_content(&profile);
    ProfileImageMaterial {
        spec: SCENE_PROFILE_IMAGE_SPEC,
        profile_id: profile.id,
        project_id: profile.project_id,
        name: profile.name,
        prompt,
        profile_content,
        source_material,
        style_metadata: profile_style_metadata(
            input.style_template_id.clone(),
            input.style_template_name.clone(),
            input.style_template_version.clone(),
            input.custom_style.clone(),
        ),
    }
}

fn profile_style_metadata(
    style_template_id: Option<String>,
    style_template_name: Option<String>,
    style_template_version: Option<String>,
    custom_style: Option<String>,
) -> ProfileMaterialStyleMetadata {
    ProfileMaterialStyleMetadata {
        style_template_id,
        style_template_name,
        style_template_version,
        custom_style,
    }
}

fn profile_source_material(parts: &[&str], prompt: &str) -> String {
    let mut source = parts.join("\n");
    source.push('\n');
    source.push_str(prompt);
    source
}

fn profile_image_metadata(material: &ProfileImageMaterial) -> Value {
    let mut metadata = profile_material_metadata(
        material.spec.profile_kind,
        material.profile_id,
        &material.name,
        &source_content_sha256(&material.profile_content),
        &material.style_metadata,
    );
    if let Some(map) = metadata.as_object_mut() {
        map.insert(
            "source".to_string(),
            json!(format!("{}_profile", material.spec.profile_kind.as_str())),
        );
        map.insert("chapter_video_material".to_string(), json!(true));
        map.insert("prompt_snapshot".to_string(), json!(material.prompt));
        map.insert(
            "source_content_sha256".to_string(),
            json!(source_content_sha256(&material.source_material)),
        );
    }
    metadata
}

pub(super) fn collect_bound_reference_images(
    host: &dyn ImageGenerationHostPort,
    bindings: &[MangaReferenceBindingSummary],
) -> Result<Vec<ImageReference>, AppError> {
    let mut references = Vec::new();
    for (index, binding) in bindings.iter().enumerate() {
        let binding_context = format!("reference_bindings[{index}]");
        let asset = binding.bound_asset.as_ref().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "参考图“{}”尚未绑定，无法生成漫画页图片",
                binding.character_name
            ))
        })?;
        let label = binding.character_name.trim();
        if label.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "{binding_context}.character_name 不能为空"
            )));
        }
        let path = asset.image_path.trim();
        if path.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "{binding_context}.bound_asset.image_path 不能为空"
            )));
        }
        references.push(host.read_reference_image(label, path)?);
    }
    Ok(references)
}
