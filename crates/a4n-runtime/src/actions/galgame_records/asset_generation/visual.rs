mod completed;
mod persistence;
mod request;

use super::prompt::upsert_galgame_prompt_artifact;
use super::visual_spec::{GalgameVisualSpecPlan, validate_galgame_visual_spec_params};
use super::*;
use completed::completed_galgame_visual_binding;
use persistence::sync_completed_galgame_image_asset;
#[cfg(test)]
pub(super) use request::validate_galgame_visual_media_request_snapshot;

pub(super) async fn run_galgame_visual_spec_generation(
    state: &AppState,
    user_id: i64,
    input: &RunGalgameAssetGenerationInput,
    spec: &generation_spec::Model,
) -> Result<(GalgameAssetGenerationItem, bool), String> {
    let plan = GalgameVisualSpecPlan::from_spec(spec)?;
    let target_context = validate_galgame_media_target(
        state,
        input.project_id,
        plan.target_entity_kind,
        plan.target_entity_id,
    )
    .await?;
    validate_galgame_visual_spec_params(spec, &plan, &target_context)?;
    if let Some(item) = completed_galgame_visual_binding(state, spec, &plan).await? {
        return Ok((item, false));
    }

    mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_RUNNING).await?;
    let prompt = upsert_galgame_prompt_artifact(state, spec, &plan).await?;
    let image_result = image_generation::generate_galgame_image(
        state,
        user_id,
        GenerateGalgameImageInput {
            project_id: input.project_id,
            prompt: prompt.prompt_text.clone(),
            image_type: plan.image_type.to_string(),
            config_id: input.config_id,
            chapter_id: spec.source_chapter_id,
            generation_spec_id: spec.id,
            target_kind: spec.target_kind.clone(),
            target_entity_kind: spec.target_entity_kind.clone(),
            target_entity_id: plan.target_entity_id,
            workflow_run_id: None,
        },
        None,
    )
    .await;
    let image = match image_result {
        Ok(image) => image,
        Err(err) => {
            mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            return Err(format!(
                "Galgame 图片生成失败: spec_id={}，{}",
                spec.id, err
            ));
        }
    };
    let result = sync_completed_galgame_image_asset(state, spec, &plan, &prompt, &image).await;
    let binding = match result {
        Ok(binding) => binding,
        Err(err) => {
            mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            return Err(format!(
                "Galgame 图片生成后媒体绑定失败: spec_id={}，{}",
                spec.id, err
            ));
        }
    };
    mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_COMPLETED).await?;
    Ok((
        GalgameAssetGenerationItem {
            generation_spec_id: spec.id,
            target_kind: spec.target_kind.clone(),
            target_entity_kind: spec.target_entity_kind.clone(),
            target_entity_id: plan.target_entity_id,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            prompt_artifact_id: Some(prompt.id),
            generated_image_id: Some(image.id),
            portrait_id: None,
            media_asset_id: Some(binding.asset.id),
            binding_id: Some(binding.binding.id),
        },
        true,
    ))
}
