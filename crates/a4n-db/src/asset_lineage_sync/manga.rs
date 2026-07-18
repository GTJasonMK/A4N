use super::*;

pub(super) async fn sync_manga_prompt<C>(
    db: &C,
    family: &FamilyLineageContext,
    row: &chapter_manga_prompt::Model,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<Option<SyncedPromptRef>, AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let Some(source) = source_context_for_row(
        family,
        row.project_id,
        Some(row.chapter_id),
        "chapter_manga_prompt",
        row.id,
        summary,
    ) else {
        return Ok(None);
    };
    let Some((target_kind, prompt_kind)) = manga_generation_kinds(&row.prompt_type) else {
        push_unsupported_prompt_type_warning(
            summary,
            "chapter_manga_prompt",
            row.id,
            &row.prompt_type,
        );
        return Ok(None);
    };
    let Some(status) = canonical_spec_status(&row.status) else {
        push_unsupported_status_warning(summary, "chapter_manga_prompt", row.id, &row.status);
        return Ok(None);
    };
    let canonical_key = legacy_key("chapter_manga_prompt", row.id, "prompt");
    let params_json = json!({
        "canonical_schema_version": CANONICAL_SCHEMA_VERSION,
        "canonical_key": canonical_key,
        "legacy_table": "chapter_manga_prompt",
        "legacy_id": row.id,
        "asset_family": "manga",
        "prompt_type": row.prompt_type,
        "page_number": row.page_number,
        "panel_number": row.panel_number,
        "dialogue_text": row.dialogue_text,
        "characters_in_frame": row.characters_in_frame,
        "layout": row.layout,
        "camera_angle": row.camera_angle,
        "dialogue_position": row.dialogue_position,
        "generated_image_id": row.generated_image_id,
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
            target_entity_kind: "chapter_manga_prompt".to_string(),
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
        &row.image_prompt,
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
