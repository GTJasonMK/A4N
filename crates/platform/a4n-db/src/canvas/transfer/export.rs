use super::types::VideoStudioProjectExportRows;
use crate::canvas::CanvasDbError;
use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset},
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_binding, media_provenance},
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub async fn video_studio_project_export_rows<C>(
    db: &C,
    project_id: i64,
) -> Result<VideoStudioProjectExportRows, CanvasDbError>
where
    C: ConnectionTrait,
{
    let canvas_graphs = canvas_graph::Entity::find()
        .filter(canvas_graph::Column::ProjectId.eq(project_id))
        .order_by_asc(canvas_graph::Column::Id)
        .all(db)
        .await?;
    let graph_ids = canvas_graphs
        .iter()
        .map(|graph| graph.id)
        .collect::<Vec<_>>();
    let (canvas_nodes, canvas_edges) = futures::join!(
        async {
            if graph_ids.is_empty() {
                return Ok::<Vec<canvas_node::Model>, CanvasDbError>(Vec::new());
            }
            canvas_node::Entity::find()
                .filter(canvas_node::Column::GraphId.is_in(graph_ids.clone()))
                .order_by_asc(canvas_node::Column::Id)
                .all(db)
                .await
                .map_err(CanvasDbError::from)
        },
        async {
            if graph_ids.is_empty() {
                return Ok::<Vec<canvas_edge::Model>, CanvasDbError>(Vec::new());
            }
            canvas_edge::Entity::find()
                .filter(canvas_edge::Column::GraphId.is_in(graph_ids.clone()))
                .order_by_asc(canvas_edge::Column::Id)
                .all(db)
                .await
                .map_err(CanvasDbError::from)
        },
    );
    let canvas_nodes = canvas_nodes?;
    let canvas_edges = canvas_edges?;

    let (
        canvas_text_assets,
        generation_and_prompt,
        generated_images,
        generated_videos,
        media_and_links,
        video_segments,
        assembly_and_clips,
    ) = futures::join!(
        async {
            canvas_text_asset::Entity::find()
                .filter(canvas_text_asset::Column::ProjectId.eq(project_id))
                .order_by_asc(canvas_text_asset::Column::Id)
                .all(db)
                .await
                .map_err(CanvasDbError::from)
        },
        async {
            let generation_specs = generation_spec::Entity::find()
                .filter(generation_spec::Column::ProjectId.eq(project_id))
                .order_by_asc(generation_spec::Column::Id)
                .all(db)
                .await
                .map_err(CanvasDbError::from)?;
            let generation_spec_ids = generation_specs
                .iter()
                .map(|spec| spec.id)
                .collect::<Vec<_>>();
            let prompt_artifacts = if generation_spec_ids.is_empty() {
                Vec::new()
            } else {
                prompt_artifact::Entity::find()
                    .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids))
                    .order_by_asc(prompt_artifact::Column::Id)
                    .all(db)
                    .await
                    .map_err(CanvasDbError::from)?
            };
            Ok::<_, CanvasDbError>((generation_specs, prompt_artifacts))
        },
        async {
            generated_image::Entity::find()
                .filter(generated_image::Column::ProjectId.eq(project_id))
                .order_by_asc(generated_image::Column::Id)
                .all(db)
                .await
                .map_err(CanvasDbError::from)
        },
        async {
            generated_video::Entity::find()
                .filter(generated_video::Column::ProjectId.eq(project_id))
                .order_by_asc(generated_video::Column::Id)
                .all(db)
                .await
                .map_err(CanvasDbError::from)
        },
        async {
            let media_assets = media_asset::Entity::find()
                .filter(media_asset::Column::ProjectId.eq(project_id))
                .order_by_asc(media_asset::Column::Id)
                .all(db)
                .await
                .map_err(CanvasDbError::from)?;
            let media_asset_ids = media_assets
                .iter()
                .map(|asset| asset.id)
                .collect::<Vec<_>>();
            let (media_provenance, media_bindings) = futures::join!(
                async {
                    if media_asset_ids.is_empty() {
                        return Ok::<Vec<media_provenance::Model>, CanvasDbError>(Vec::new());
                    }
                    media_provenance::Entity::find()
                        .filter(
                            media_provenance::Column::MediaAssetId.is_in(media_asset_ids.clone()),
                        )
                        .all(db)
                        .await
                        .map_err(CanvasDbError::from)
                },
                async {
                    if media_asset_ids.is_empty() {
                        return Ok::<Vec<media_binding::Model>, CanvasDbError>(Vec::new());
                    }
                    media_binding::Entity::find()
                        .filter(media_binding::Column::MediaAssetId.is_in(media_asset_ids.clone()))
                        .order_by_asc(media_binding::Column::Id)
                        .all(db)
                        .await
                        .map_err(CanvasDbError::from)
                },
            );
            Ok::<_, CanvasDbError>((media_assets, media_provenance?, media_bindings?))
        },
        async {
            video_segment::Entity::find()
                .filter(video_segment::Column::ProjectId.eq(project_id))
                .order_by_asc(video_segment::Column::OrderIndex)
                .order_by_asc(video_segment::Column::Id)
                .all(db)
                .await
                .map_err(CanvasDbError::from)
        },
        async {
            let assembly_timelines = assembly_timeline::Entity::find()
                .filter(assembly_timeline::Column::ProjectId.eq(project_id))
                .order_by_asc(assembly_timeline::Column::Id)
                .all(db)
                .await
                .map_err(CanvasDbError::from)?;
            let timeline_ids = assembly_timelines
                .iter()
                .map(|timeline| timeline.id)
                .collect::<Vec<_>>();
            let assembly_clips = if timeline_ids.is_empty() {
                Vec::new()
            } else {
                assembly_clip::Entity::find()
                    .filter(assembly_clip::Column::TimelineId.is_in(timeline_ids))
                    .order_by_asc(assembly_clip::Column::TimelineId)
                    .order_by_asc(assembly_clip::Column::OrderIndex)
                    .order_by_asc(assembly_clip::Column::Id)
                    .all(db)
                    .await
                    .map_err(CanvasDbError::from)?
            };
            Ok::<_, CanvasDbError>((assembly_timelines, assembly_clips))
        },
    );
    let canvas_text_assets = canvas_text_assets?;
    let (generation_specs, prompt_artifacts) = generation_and_prompt?;
    let generated_images = generated_images?;
    let generated_videos = generated_videos?;
    let (media_assets, media_provenance, media_bindings) = media_and_links?;
    let video_segments = video_segments?;
    let (assembly_timelines, assembly_clips) = assembly_and_clips?;

    Ok(VideoStudioProjectExportRows {
        canvas_graphs,
        canvas_nodes,
        canvas_edges,
        canvas_text_assets,
        generation_specs,
        prompt_artifacts,
        generated_images,
        generated_videos,
        media_assets,
        media_provenance,
        media_bindings,
        video_segments,
        assembly_timelines,
        assembly_clips,
    })
}
