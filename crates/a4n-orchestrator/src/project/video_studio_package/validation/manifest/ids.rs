use super::super::*;
use super::contracts::ManifestIds;

pub(super) fn validate_and_collect_manifest_ids(
    package: &VideoStudioProjectPackage,
) -> Result<ManifestIds, String> {
    validate_unique_ids(
        package.canvas_graphs.iter().map(|record| record.id),
        "canvas_graph.id",
    )?;
    validate_unique_ids(
        package.canvas_nodes.iter().map(|record| record.id),
        "canvas_node.id",
    )?;
    validate_unique_ids(
        package.canvas_edges.iter().map(|record| record.id),
        "canvas_edge.id",
    )?;
    validate_unique_ids(
        package.canvas_text_assets.iter().map(|record| record.id),
        "canvas_text_asset.id",
    )?;
    validate_unique_ids(
        package.generation_specs.iter().map(|record| record.id),
        "generation_spec.id",
    )?;
    validate_unique_ids(
        package.prompt_artifacts.iter().map(|record| record.id),
        "prompt_artifact.id",
    )?;
    validate_unique_ids(
        package.generated_images.iter().map(|record| record.id),
        "generated_image.id",
    )?;
    validate_unique_ids(
        package.generated_videos.iter().map(|record| record.id),
        "generated_video.id",
    )?;
    validate_unique_ids(
        package.media_assets.iter().map(|record| record.id),
        "media_asset.id",
    )?;
    validate_unique_ids(
        package.media_bindings.iter().map(|record| record.id),
        "media_binding.id",
    )?;
    validate_unique_ids(
        package.video_segments.iter().map(|record| record.id),
        "video_segment.id",
    )?;
    validate_unique_ids(
        package.assembly_timelines.iter().map(|record| record.id),
        "assembly_timeline.id",
    )?;
    validate_unique_ids(
        package.assembly_clips.iter().map(|record| record.id),
        "assembly_clip.id",
    )?;

    Ok(ManifestIds {
        graph_ids: package
            .canvas_graphs
            .iter()
            .map(|record| record.id)
            .collect(),
        node_ids: package
            .canvas_nodes
            .iter()
            .map(|record| record.id)
            .collect(),
        text_asset_ids: package
            .canvas_text_assets
            .iter()
            .map(|record| record.id)
            .collect(),
        generation_spec_ids: package
            .generation_specs
            .iter()
            .map(|record| record.id)
            .collect(),
        prompt_artifact_ids: package
            .prompt_artifacts
            .iter()
            .map(|record| record.id)
            .collect(),
        image_ids: package
            .generated_images
            .iter()
            .map(|record| record.id)
            .collect(),
        video_ids: package
            .generated_videos
            .iter()
            .map(|record| record.id)
            .collect(),
        media_asset_ids: package
            .media_assets
            .iter()
            .map(|record| record.id)
            .collect(),
        video_segment_ids: package
            .video_segments
            .iter()
            .map(|record| record.id)
            .collect(),
        assembly_timeline_ids: package
            .assembly_timelines
            .iter()
            .map(|record| record.id)
            .collect(),
        assembly_clip_ids: package
            .assembly_clips
            .iter()
            .map(|record| record.id)
            .collect(),
    })
}
