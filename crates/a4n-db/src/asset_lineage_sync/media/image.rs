use a4n_core::models::{ai::generated_image, chapter::chapter_manga_prompt, media::media_asset};
use sea_orm::ConnectionTrait;
use serde_json::json;

use crate::asset_lineage_sync::context::media_source_context_for_row;
use crate::asset_lineage_sync::persistence::{
    MediaUpsertInput, ensure_media_binding, upsert_media_asset,
};
use crate::asset_lineage_sync::rules::legacy_key;
use crate::asset_lineage_sync::types::{FamilyLineageContext, SyncedPromptRef};
use crate::asset_lineage_sync::{
    AssetLineageBackfillSummary, AssetLineageSyncError, CANONICAL_SCHEMA_VERSION,
};

pub(in crate::asset_lineage_sync) async fn sync_generated_image_media<C>(
    db: &C,
    family: &FamilyLineageContext,
    row: &generated_image::Model,
    prompt: Option<&chapter_manga_prompt::Model>,
    prompt_ref: Option<&SyncedPromptRef>,
    source_media_asset_id: Option<i64>,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<Option<media_asset::Model>, AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let Some(project_context) = family.contexts_by_project_id.get(&row.project_id) else {
        summary.warnings.push(format!(
            "generated_image #{} 所属项目 {} 不在当前项目族中，已跳过。",
            row.id, row.project_id
        ));
        return Ok(None);
    };
    let Some(source) = media_source_context_for_row(
        project_context,
        row.chapter_id,
        "generated_image",
        row.id,
        summary,
    ) else {
        return Ok(None);
    };
    let request_snapshot_json = json!({
        "canonical_schema_version": CANONICAL_SCHEMA_VERSION,
        "canonical_key": legacy_key("generated_image", row.id, "media"),
        "legacy_table": "generated_image",
        "legacy_id": row.id,
        "asset_family": if row.image_type == "video_tail_frame" { "video" } else { "manga" },
        "image_type": row.image_type,
        "prompt": row.prompt,
        "model": row.model,
        "chapter_id": row.chapter_id,
        "panel_id": row.panel_id,
        "metadata": row.metadata_json,
        "linked_chapter_manga_prompt_id": prompt.map(|prompt| prompt.id),
    });
    let media = upsert_media_asset(
        db,
        family,
        MediaUpsertInput {
            canonical_key: legacy_key("generated_image", row.id, "media"),
            project_id: row.project_id,
            root_project_id: project_context.root_project_id,
            source_snapshot_id: source.source_snapshot_id,
            source_chapter_id: source.source_chapter_id,
            source_snapshot_chapter_id: source.source_snapshot_chapter_id,
            media_kind: "image".to_string(),
            origin: if row.image_type == "video_tail_frame" {
                "extracted_frame".to_string()
            } else {
                "generated".to_string()
            },
            file_path: Some(row.file_path.clone()),
            thumbnail_path: None,
            width: Some(row.width),
            height: Some(row.height),
            duration_secs: None,
            fps: None,
            file_size: row.file_size,
            status: "completed".to_string(),
            error_message: None,
            generation_spec_id: prompt_ref.map(|prompt| prompt.generation_spec_id),
            prompt_artifact_id: prompt_ref.map(|prompt| prompt.prompt_artifact_id),
            provider_job_id: None,
            source_media_asset_id,
            source_external_path: None,
            imported_from_user: false,
            request_snapshot_json,
            created_at: row.created_at,
        },
        summary,
    )
    .await?;
    if let Some(prompt) = prompt {
        ensure_media_binding(
            db,
            media.id,
            "chapter_manga_prompt",
            prompt.id,
            "generated_result",
            summary,
        )
        .await?;
    }
    if let Some(chapter_id) = row.chapter_id {
        ensure_media_binding(
            db,
            media.id,
            "chapter",
            chapter_id,
            if row.image_type == "video_tail_frame" {
                "end_frame"
            } else {
                "generated_result"
            },
            summary,
        )
        .await?;
    }
    Ok(Some(media))
}
