use super::super::super::{DirectImageGenerationInput, ProfileImageMaterial};
use super::metadata::profile_image_metadata;

pub(in crate::actions::image_generation) fn profile_material_direct_input(
    material: ProfileImageMaterial,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
) -> DirectImageGenerationInput {
    let (
        profile_kind,
        image_type,
        command_name,
        preview_workflow,
        preview_step_key,
        preview_step_title,
        preview_title_suffix,
    ) = material.spec;
    let metadata = profile_image_metadata(
        profile_kind,
        material.profile_id,
        &material.name,
        &material.profile_content,
        &material.source_material,
        &material.prompt,
        &material.style_metadata,
    );
    DirectImageGenerationInput {
        project_id: material.project_id,
        prompt: material.prompt,
        image_type: image_type.to_string(),
        config_id,
        chapter_id,
        panel_id: Some(format!("{profile_kind}_profile_{}", material.profile_id)),
        metadata_json: Some(metadata),
        workflow_run_id: None,
        command_name: command_name.to_string(),
        preview_workflow: preview_workflow.to_string(),
        preview_title: format!("{} {preview_title_suffix}", material.name),
        preview_step_key: preview_step_key.to_string(),
        preview_step_title: preview_step_title.to_string(),
    }
}
