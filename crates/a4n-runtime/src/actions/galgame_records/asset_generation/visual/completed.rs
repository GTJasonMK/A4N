use super::super::visual_spec::GalgameVisualSpecPlan;
use super::super::{
    AppState, GalgameAssetGenerationItem, WORKFLOW_STATUS_COMPLETED, canonicalize_data_file_path,
    generation_spec,
};
use super::request::validate_galgame_visual_media_request_snapshot;
use crate::actions::app_error_message;

pub(super) async fn completed_galgame_visual_binding(
    state: &AppState,
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
) -> Result<Option<GalgameAssetGenerationItem>, String> {
    if spec.status != WORKFLOW_STATUS_COMPLETED {
        return Ok(None);
    }
    let records = state
        .orchestrator
        .completed_galgame_visual_binding_records(
            spec,
            plan.target_entity_kind,
            plan.target_entity_id,
            plan.role,
        )
        .await
        .map_err(app_error_message)?;
    let binding = records.binding;
    let asset = records.asset;
    let provenance = records.provenance;
    let Some(file_path) = asset.file_path.as_deref() else {
        return Err(format!(
            "Galgame 视觉生成意图已完成但绑定媒体缺少文件路径: spec_id={}, media_asset_id={}",
            spec.id, asset.id
        ));
    };
    canonicalize_data_file_path(&state.config.data_dir, file_path, "Galgame 视觉媒体文件")
        .map_err(|error| {
            format!(
                "Galgame 视觉生成意图已完成但绑定媒体文件不可用: spec_id={}, media_asset_id={}，{}",
                spec.id, asset.id, error
            )
        })?;
    validate_galgame_visual_media_request_snapshot(
        spec,
        plan,
        asset.id,
        provenance.request_snapshot_json.as_ref(),
    )?;
    Ok(Some(GalgameAssetGenerationItem {
        generation_spec_id: spec.id,
        target_kind: spec.target_kind.clone(),
        target_entity_kind: spec.target_entity_kind.clone(),
        target_entity_id: plan.target_entity_id,
        status: WORKFLOW_STATUS_COMPLETED.to_string(),
        prompt_artifact_id: provenance.prompt_artifact_id,
        generated_image_id: None,
        portrait_id: None,
        media_asset_id: Some(asset.id),
        binding_id: Some(binding.id),
    }))
}
