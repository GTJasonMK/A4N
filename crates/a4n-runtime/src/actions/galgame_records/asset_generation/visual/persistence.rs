use super::super::visual_spec::GalgameVisualSpecPlan;
use super::super::{
    AppState, BindGalgameMediaInput, CreateMediaAssetInput, CreateMediaProvenanceInput,
    GalgameMediaBindingResult, GeneratedImageSummary, MEDIA_KIND_IMAGE, MEDIA_ORIGIN_GENERATED,
    MEDIA_STATUS_COMPLETED, bind_galgame_media_inner, generation_spec, project_record_by_id,
    prompt_artifact,
};
use crate::actions::app_error_message;
use serde_json::json;

pub(super) async fn sync_completed_galgame_image_asset(
    state: &AppState,
    spec: &generation_spec::Model,
    plan: &GalgameVisualSpecPlan,
    prompt: &prompt_artifact::Model,
    image: &GeneratedImageSummary,
) -> Result<GalgameMediaBindingResult, String> {
    let generated = state
        .orchestrator
        .galgame_generated_image_by_id(image.id, spec.project_id)
        .await
        .map_err(app_error_message)?;
    let file_path = generated.file_path.trim();
    let project = project_record_by_id(state, spec.project_id).await?;
    let source_snapshot_id = project
        .source_snapshot_id
        .ok_or_else(|| format!("Galgame 项目缺少源文本快照: {}", project.id))?;
    let asset = state
        .create_media_asset(CreateMediaAssetInput {
            project_id: spec.project_id,
            root_project_id: spec.root_project_id,
            source_snapshot_id: Some(source_snapshot_id),
            source_chapter_id: spec.source_chapter_id,
            source_snapshot_chapter_id: spec.source_snapshot_chapter_id,
            media_kind: MEDIA_KIND_IMAGE.to_string(),
            origin: MEDIA_ORIGIN_GENERATED.to_string(),
            file_path: Some(file_path.to_string()),
            thumbnail_path: None,
            width: Some(generated.width),
            height: Some(generated.height),
            duration_secs: None,
            fps: None,
            status: MEDIA_STATUS_COMPLETED.to_string(),
            error_message: None,
            provenance: CreateMediaProvenanceInput {
                generation_spec_id: Some(spec.id),
                prompt_artifact_id: Some(prompt.id),
                provider_job_id: None,
                source_media_asset_id: None,
                source_external_path: Some(format!("generated_image:{}", generated.id)),
                imported_from_user: false,
                request_snapshot_json: Some(json!({
                    "source": "galgame_asset_generation",
                    "generated_image_id": generated.id,
                    "image_type": generated.image_type,
                    "model": generated.model,
                    "prompt": generated.prompt,
                    "metadata_json": generated.metadata_json,
                    "target_kind": spec.target_kind,
                    "target_entity_kind": spec.target_entity_kind,
                    "target_entity_id": plan.target_entity_id,
                })),
            },
        })
        .await
        .map_err(|error| error.to_string())?;
    match bind_galgame_media_inner(
        state,
        BindGalgameMediaInput {
            project_id: spec.project_id,
            media_asset_id: asset.id,
            target_kind: plan.target_entity_kind.to_string(),
            target_id: plan.target_entity_id,
            role: plan.role.to_string(),
        },
    )
    .await
    {
        Ok(result) => Ok(result),
        Err(error) => {
            let cleanup_error = cleanup_generated_media_asset_record(state, asset.id)
                .await
                .err();
            Err(match cleanup_error {
                Some(cleanup_error) => format!("{error}；后续清理也失败：{cleanup_error}"),
                None => error,
            })
        }
    }
}

async fn cleanup_generated_media_asset_record(
    state: &AppState,
    media_asset_id: i64,
) -> Result<(), String> {
    state
        .orchestrator
        .delete_media_asset_with_provenance_records(media_asset_id)
        .await
        .map_err(app_error_message)
}
