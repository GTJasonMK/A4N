use super::contracts::ManifestIds;

use crate::project::{collect_unique_ids, video_studio_package::VideoStudioProjectPackage};

pub(super) fn validate_and_collect_manifest_ids(
    package: &VideoStudioProjectPackage,
) -> Result<ManifestIds, String> {
    let graph_ids = collect_unique_ids(
        package.canvas_graphs.iter().map(|record| record.id),
        "canvas_graph.id",
    )?;
    let node_ids = collect_unique_ids(
        package.canvas_nodes.iter().map(|record| record.id),
        "canvas_node.id",
    )?;
    let text_asset_ids = collect_unique_ids(
        package.canvas_text_assets.iter().map(|record| record.id),
        "canvas_text_asset.id",
    )?;
    let generation_spec_ids = collect_unique_ids(
        package.generation_specs.iter().map(|record| record.id),
        "generation_spec.id",
    )?;
    let prompt_artifact_ids = collect_unique_ids(
        package.prompt_artifacts.iter().map(|record| record.id),
        "prompt_artifact.id",
    )?;
    let image_ids = collect_unique_ids(
        package.generated_images.iter().map(|record| record.id),
        "generated_image.id",
    )?;
    let video_ids = collect_unique_ids(
        package.generated_videos.iter().map(|record| record.id),
        "generated_video.id",
    )?;
    let media_asset_ids = collect_unique_ids(
        package.media_assets.iter().map(|record| record.id),
        "media_asset.id",
    )?;
    let video_segment_ids = collect_unique_ids(
        package.video_segments.iter().map(|record| record.id),
        "video_segment.id",
    )?;
    let assembly_timeline_ids = collect_unique_ids(
        package.assembly_timelines.iter().map(|record| record.id),
        "assembly_timeline.id",
    )?;
    let assembly_clip_ids = collect_unique_ids(
        package.assembly_clips.iter().map(|record| record.id),
        "assembly_clip.id",
    )?;

    Ok(ManifestIds {
        graph_ids,
        node_ids,
        text_asset_ids,
        generation_spec_ids,
        prompt_artifact_ids,
        image_ids,
        video_ids,
        media_asset_ids,
        video_segment_ids,
        assembly_timeline_ids,
        assembly_clip_ids,
    })
}
