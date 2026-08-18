use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt, media::media_asset};
use a4n_domain::EXTERNAL_IMPORT_MODEL;
use sea_orm::ConnectionTrait;
use serde_json::json;

use crate::asset_lineage_sync::context::media_source_context_for_row;
use crate::asset_lineage_sync::persistence::{
    MediaUpsertInput, ensure_media_binding, upsert_media_asset,
};
use crate::asset_lineage_sync::rules::{
    canonical_media_status, legacy_key, push_unsupported_status_warning,
};
use crate::asset_lineage_sync::types::{FamilyLineageContext, SyncedPromptRef};
use crate::asset_lineage_sync::{
    AssetLineageBackfillSummary, AssetLineageSyncError, CANONICAL_SCHEMA_VERSION,
};

pub(in crate::asset_lineage_sync) async fn sync_generated_video_media<C>(
    db: &C,
    family: &FamilyLineageContext,
    row: &generated_video::Model,
    prompt: Option<&chapter_video_prompt::Model>,
    prompt_ref: Option<&SyncedPromptRef>,
    source_media_asset_id: Option<i64>,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<Option<media_asset::Model>, AssetLineageSyncError>
where
    C: ConnectionTrait,
{
    let Some(project_context) = family.contexts_by_project_id.get(&row.project_id) else {
        summary.warnings.push(format!(
            "generated_video #{} 所属项目 {} 不在当前项目族中，已跳过。",
            row.id, row.project_id
        ));
        return Ok(None);
    };
    let Some(status) = canonical_media_status(&row.status) else {
        push_unsupported_status_warning(summary, "generated_video", row.id, &row.status);
        return Ok(None);
    };
    let Some(source) = media_source_context_for_row(
        project_context,
        row.chapter_id,
        "generated_video",
        row.id,
        summary,
    ) else {
        return Ok(None);
    };
    let request_snapshot_json = json!({
        "canonical_schema_version": CANONICAL_SCHEMA_VERSION,
        "canonical_key": legacy_key("generated_video", row.id, "media"),
        "legacy_table": "generated_video",
        "legacy_id": row.id,
        "asset_family": "video",
        "video_type": row.video_type,
        "workflow_mode": row.workflow_mode,
        "prompt": row.prompt,
        "negative_prompt": row.negative_prompt,
        "model": row.model,
        "chapter_id": row.chapter_id,
        "source_video_id": row.source_video_id,
        "source_image_id": row.source_image_id,
        "metadata": row.metadata_json,
        "linked_chapter_video_prompt_id": prompt.map(|prompt| prompt.id),
    });
    let imported_from_user = row.model == EXTERNAL_IMPORT_MODEL;
    let media = upsert_media_asset(
        db,
        family,
        MediaUpsertInput {
            canonical_key: legacy_key("generated_video", row.id, "media"),
            project_id: row.project_id,
            root_project_id: project_context.root_project_id,
            source_snapshot_id: source.source_snapshot_id,
            source_chapter_id: source.source_chapter_id,
            source_snapshot_chapter_id: source.source_snapshot_chapter_id,
            media_kind: "video".to_string(),
            origin: if imported_from_user {
                "imported".to_string()
            } else {
                "generated".to_string()
            },
            file_path: row.file_path.clone(),
            thumbnail_path: row.thumbnail_path.clone(),
            width: Some(row.width),
            height: Some(row.height),
            duration_secs: Some(row.duration_secs),
            fps: Some(row.fps as f32),
            file_size: row.file_size,
            status: status.to_string(),
            error_message: row.error_message.clone(),
            generation_spec_id: prompt_ref.map(|prompt| prompt.generation_spec_id),
            prompt_artifact_id: prompt_ref.map(|prompt| prompt.prompt_artifact_id),
            provider_job_id: row.provider_job_id.clone(),
            source_media_asset_id,
            source_external_path: None,
            imported_from_user,
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
            "chapter_video_prompt",
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
            "generated_result",
            summary,
        )
        .await?;
    }
    Ok(Some(media))
}
