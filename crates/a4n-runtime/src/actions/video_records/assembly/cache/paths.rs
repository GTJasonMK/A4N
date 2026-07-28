use std::path::{Path, PathBuf};

pub(in crate::actions::video_records::assembly) fn assembled_video_output_path(
    chapter_videos_dir: &Path,
    workflow_mode: &str,
) -> Result<PathBuf, String> {
    Ok(chapter_videos_dir.join(format!("assembled_{workflow_mode}.mp4")))
}

pub(in crate::actions::video_records::assembly) fn assembly_cache_manifest_path(
    output_path: &Path,
) -> PathBuf {
    let file_name = output_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("assembled.mp4");
    output_path.with_file_name(format!("{file_name}.cache.json"))
}
