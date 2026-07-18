use a4n_core::models::{
    ai::{generated_image, generated_video},
    canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset},
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_binding, media_provenance},
    project::novel_project,
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use a4n_orchestrator::canvas::VideoStudioProjectImportRows;
#[cfg(test)]
use a4n_orchestrator::project::SOURCE_SYNC_CURRENT;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

const VIDEO_STUDIO_PACKAGE_VERSION: &str = "video_studio_project_v1";
const MANIFEST_PATH: &str = "a4n-video-studio-project.json";
const PACKAGE_MODE: &str = "video_studio_project";
const MEDIA_OWNER_IMAGE: &str = "generated_image";
const MEDIA_OWNER_VIDEO: &str = "generated_video";
const MEDIA_OWNER_MEDIA_ASSET: &str = "media_asset";
const MEDIA_FIELD_FILE: &str = "file_path";
const MEDIA_FIELD_THUMBNAIL: &str = "thumbnail_path";

mod export;
mod import;
mod package_io;
mod validation;

pub(super) use export::export_video_studio_project_package_bytes;
pub(super) use import::import_video_studio_project_package_bytes;

#[derive(Debug, Serialize, Deserialize)]
struct VideoStudioProjectPackage {
    version: String,
    exported_at: String,
    mode: String,
    project: novel_project::Model,
    canvas_graphs: Vec<canvas_graph::Model>,
    canvas_nodes: Vec<canvas_node::Model>,
    canvas_edges: Vec<canvas_edge::Model>,
    canvas_text_assets: Vec<canvas_text_asset::Model>,
    generation_specs: Vec<generation_spec::Model>,
    prompt_artifacts: Vec<prompt_artifact::Model>,
    generated_images: Vec<generated_image::Model>,
    generated_videos: Vec<generated_video::Model>,
    media_assets: Vec<media_asset::Model>,
    media_provenance: Vec<media_provenance::Model>,
    media_bindings: Vec<media_binding::Model>,
    video_segments: Vec<video_segment::Model>,
    assembly_timelines: Vec<assembly_timeline::Model>,
    assembly_clips: Vec<assembly_clip::Model>,
    media_files: Vec<VideoStudioMediaFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VideoStudioMediaFileEntry {
    owner_kind: String,
    owner_id: i64,
    field: String,
    original_path: String,
    package_path: String,
}

#[derive(Debug)]
struct ImportMediaPayload {
    entries: HashMap<(String, i64, String), VideoStudioMediaFileEntry>,
    bytes: HashMap<String, Vec<u8>>,
}

impl VideoStudioProjectPackage {
    fn into_import_rows(self) -> VideoStudioProjectImportRows {
        VideoStudioProjectImportRows {
            project: self.project,
            canvas_graphs: self.canvas_graphs,
            canvas_nodes: self.canvas_nodes,
            canvas_edges: self.canvas_edges,
            canvas_text_assets: self.canvas_text_assets,
            generation_specs: self.generation_specs,
            prompt_artifacts: self.prompt_artifacts,
            generated_images: self.generated_images,
            generated_videos: self.generated_videos,
            media_assets: self.media_assets,
            media_provenance: self.media_provenance,
            media_bindings: self.media_bindings,
            video_segments: self.video_segments,
            assembly_timelines: self.assembly_timelines,
            assembly_clips: self.assembly_clips,
        }
    }
}

fn media_file_key(owner_kind: &str, owner_id: i64, field: &str) -> (String, i64, String) {
    (owner_kind.to_string(), owner_id, field.to_string())
}

fn required_package_extension(path: &str) -> Result<String, String> {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| format!("媒体包路径缺少扩展名: {path}"))
}
