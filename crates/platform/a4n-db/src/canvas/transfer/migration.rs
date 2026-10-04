use std::collections::HashSet;

use a4n_domain::project::ProjectRecord;
use chrono::Utc;

use super::types::{
    VideoStudioProjectImportError, VideoStudioProjectImportMediaFileWriter,
    VideoStudioProjectImportRows,
};
use crate::canvas::transfer::import_project::import_video_studio_project_record;

/// Copy only the records that are owned by a canvas out of a derived video
/// project. Chapter backed media and chapter generation rows are intentionally
/// left in the source video project.
pub async fn migrate_derived_video_canvas_project_record<Writer>(
    db: &impl crate::DbExecutor,
    source: &ProjectRecord,
    title: String,
    rows: super::types::VideoStudioProjectExportRows,
    media_file_writer: &mut Writer,
) -> Result<Option<i64>, VideoStudioProjectImportError>
where
    Writer: VideoStudioProjectImportMediaFileWriter,
{
    let mut rows = canvas_owned_rows(rows);
    if rows.canvas_graphs.is_empty()
        && rows.canvas_text_assets.is_empty()
        && rows.generation_specs.is_empty()
        && rows.generated_images.is_empty()
        && rows.generated_videos.is_empty()
        && rows.media_assets.is_empty()
    {
        return Ok(None);
    }

    let mut project = source.clone();
    project.title = title;
    project.project_kind = "canvas".to_string();
    project.project_type = "script".to_string();
    project.root_project_id = None;
    project.source_snapshot_id = None;
    project.source_sync_status = "current".to_string();
    project.target_chapters = 0;
    project.current_chapter = 0;
    project.total_chapters = None;
    project.chapters_per_part = None;
    project.raw_blueprint = None;
    project.import_analysis_progress = None;
    project.export_config = None;
    project.created_at = Utc::now().naive_utc();
    project.updated_at = project.created_at;

    let id = import_video_studio_project_record(
        db,
        source.user_id,
        VideoStudioProjectImportRows {
            project,
            canvas_graphs: std::mem::take(&mut rows.canvas_graphs),
            canvas_nodes: std::mem::take(&mut rows.canvas_nodes),
            canvas_edges: std::mem::take(&mut rows.canvas_edges),
            canvas_text_assets: std::mem::take(&mut rows.canvas_text_assets),
            generation_specs: std::mem::take(&mut rows.generation_specs),
            prompt_artifacts: std::mem::take(&mut rows.prompt_artifacts),
            generated_images: std::mem::take(&mut rows.generated_images),
            generated_videos: std::mem::take(&mut rows.generated_videos),
            media_assets: std::mem::take(&mut rows.media_assets),
            media_provenance: std::mem::take(&mut rows.media_provenance),
            media_bindings: std::mem::take(&mut rows.media_bindings),
            video_segments: std::mem::take(&mut rows.video_segments),
            assembly_timelines: std::mem::take(&mut rows.assembly_timelines),
            assembly_clips: std::mem::take(&mut rows.assembly_clips),
        },
        Utc::now().naive_utc(),
        media_file_writer,
    )
    .await?;

    if let Err(error) = copy_project_members(db, source, id).await {
        use a4n_db_models::models::project::novel_project;
        use sea_orm::EntityTrait;
        novel_project::Entity::delete_by_id(id).exec(db).await?;
        return Err(error);
    }
    Ok(Some(id))
}

fn canvas_owned_rows(
    mut rows: super::types::VideoStudioProjectExportRows,
) -> super::types::VideoStudioProjectExportRows {
    let graph_ids = rows
        .canvas_graphs
        .iter()
        .filter(|graph| {
            rows.canvas_nodes
                .iter()
                .any(|node| node.graph_id == graph.id)
                || rows
                    .canvas_edges
                    .iter()
                    .any(|edge| edge.graph_id == graph.id)
        })
        .map(|graph| graph.id)
        .collect::<HashSet<_>>();
    rows.canvas_graphs
        .retain(|graph| graph_ids.contains(&graph.id));
    rows.canvas_nodes
        .retain(|node| graph_ids.contains(&node.graph_id));
    rows.canvas_edges
        .retain(|edge| graph_ids.contains(&edge.graph_id));

    rows.generated_images
        .retain(|image| matches!(image.image_type.as_str(), "canvas_image" | "canvas_frame"));
    rows.generated_videos.retain(|video| {
        video.video_type == a4n_domain::video_workflow::VIDEO_TYPE_CANVAS
            || video.video_type == a4n_domain::video_workflow::VIDEO_TYPE_CANVAS_ASSEMBLY
            || matches!(
                video.workflow_mode.as_deref(),
                Some(
                    a4n_domain::video_workflow::WORKFLOW_MODE_CANVAS_VIDEO
                        | a4n_domain::video_workflow::WORKFLOW_MODE_CANVAS_TRIM
                        | a4n_domain::video_workflow::WORKFLOW_MODE_CANVAS_ASSEMBLY
                )
            )
    });
    let spec_ids = rows
        .generation_specs
        .iter()
        .filter(|spec| {
            spec.source_context_kind.as_deref()
                == Some(a4n_db_models::models::generation::generation_spec::SOURCE_CONTEXT_FREEFORM)
        })
        .map(|spec| spec.id)
        .collect::<HashSet<_>>();
    rows.generation_specs
        .retain(|spec| spec_ids.contains(&spec.id));
    rows.prompt_artifacts
        .retain(|artifact| spec_ids.contains(&artifact.generation_spec_id));

    let media_ids = rows
        .media_assets
        .iter()
        .filter(|asset| {
            asset.source_snapshot_id.is_none()
                && asset.source_chapter_id.is_none()
                && asset.source_snapshot_chapter_id.is_none()
        })
        .map(|asset| asset.id)
        .collect::<HashSet<_>>();
    rows.media_assets
        .retain(|asset| media_ids.contains(&asset.id));
    rows.media_provenance.retain(|provenance| {
        media_ids.contains(&provenance.media_asset_id)
            && provenance
                .generation_spec_id
                .is_none_or(|id| spec_ids.contains(&id))
    });
    rows.media_bindings
        .retain(|binding| media_ids.contains(&binding.media_asset_id));

    let segment_ids = rows
        .video_segments
        .iter()
        .filter(|segment| {
            segment.source_chapter_id.is_none() && segment.source_snapshot_chapter_id.is_none()
        })
        .map(|segment| segment.id)
        .collect::<HashSet<_>>();
    rows.video_segments
        .retain(|segment| segment_ids.contains(&segment.id));
    let timeline_ids = rows
        .assembly_timelines
        .iter()
        .filter(|timeline| {
            timeline.source_chapter_id.is_none() && timeline.source_snapshot_chapter_id.is_none()
        })
        .map(|timeline| timeline.id)
        .collect::<HashSet<_>>();
    rows.assembly_timelines
        .retain(|timeline| timeline_ids.contains(&timeline.id));
    rows.assembly_clips.retain(|clip| {
        timeline_ids.contains(&clip.timeline_id)
            && media_ids.contains(&clip.media_asset_id)
            && clip
                .source_segment_id
                .is_none_or(|id| segment_ids.contains(&id))
    });
    rows
}

async fn copy_project_members(
    db: &impl crate::DbExecutor,
    source: &ProjectRecord,
    target_project_id: i64,
) -> Result<(), VideoStudioProjectImportError> {
    use a4n_db_models::models::project::project_member;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

    let source_project_id = source.root_project_id.unwrap_or(source.id);
    let members = project_member::Entity::find()
        .filter(project_member::Column::ProjectId.eq(source_project_id))
        .all(db)
        .await?;
    for member in members {
        project_member::ActiveModel {
            project_id: Set(target_project_id),
            user_id: Set(member.user_id),
            role: Set(member.role),
            invited_by_user_id: Set(member.invited_by_user_id),
            created_at: Set(Utc::now().naive_utc()),
            updated_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
}
