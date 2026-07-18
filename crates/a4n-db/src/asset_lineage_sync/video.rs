use super::*;

pub(super) async fn sync_video_prompt<C>(
    db: &C,
    family: &FamilyLineageContext,
    row: &chapter_video_prompt::Model,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<Option<SyncedPromptRef>, AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let Some(source) = source_context_for_row(
        family,
        row.project_id,
        Some(row.chapter_id),
        "chapter_video_prompt",
        row.id,
        summary,
    ) else {
        return Ok(None);
    };
    let Some((target_kind, prompt_kind)) =
        video_generation_kinds(&row.prompt_type, row.workflow_mode.as_str())
    else {
        push_unsupported_prompt_type_warning(
            summary,
            "chapter_video_prompt",
            row.id,
            &row.prompt_type,
        );
        return Ok(None);
    };
    let Some(status) = canonical_spec_status(&row.status) else {
        push_unsupported_status_warning(summary, "chapter_video_prompt", row.id, &row.status);
        return Ok(None);
    };
    let canonical_key = legacy_key("chapter_video_prompt", row.id, "prompt");
    let params_json = json!({
        "canonical_schema_version": CANONICAL_SCHEMA_VERSION,
        "canonical_key": canonical_key,
        "legacy_table": "chapter_video_prompt",
        "legacy_id": row.id,
        "asset_family": "video",
        "prompt_type": row.prompt_type,
        "workflow_mode": row.workflow_mode,
        "shot_number": row.shot_number,
        "title": row.title,
        "scene_summary": row.scene_summary,
        "visual_prompt": row.visual_prompt,
        "motion_prompt": row.motion_prompt,
        "negative_prompt": row.negative_prompt,
        "duration_secs": row.duration_secs,
        "aspect_ratio": row.aspect_ratio,
        "reference_image_ids": row.reference_image_ids_json,
        "metadata": row.metadata_json,
        "generated_video_id": row.generated_video_id,
    });
    let spec = upsert_generation_spec(
        db,
        family,
        GenerationSpecUpsertInput {
            canonical_key,
            project_id: source.project_id,
            root_project_id: source.root_project_id,
            source_snapshot_id: source.source_snapshot_id,
            source_chapter_id: source.source_chapter_id,
            source_snapshot_chapter_id: source.source_snapshot_chapter_id,
            target_kind: target_kind.to_string(),
            target_entity_kind: "chapter_video_prompt".to_string(),
            target_entity_id: Some(row.id),
            status: status.to_string(),
            source_version_hash: source.source_version_hash,
            params_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        },
        summary,
    )
    .await?;
    let artifact = upsert_prompt_artifact(
        db,
        spec.id,
        prompt_kind,
        &video_prompt_artifact_text(row),
        row.created_at,
        row.updated_at,
        summary,
    )
    .await?;
    Ok(Some(SyncedPromptRef {
        generation_spec_id: spec.id,
        prompt_artifact_id: artifact.id,
    }))
}
