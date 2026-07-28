use super::assembly::{
    AssemblyCacheManifest, AssemblyClip, assembly_cache_is_fresh, assembly_cache_matches,
    build_assembly_cache_manifest_with_audio_probe, expired_assembly_cache_files,
    write_assembly_cache_manifest,
};
use super::imports::{
    import_video_source_path, video_probe_error_message,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .expect("system clock must be after UNIX_EPOCH");
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
}

fn assembly_clip(video_id: i64, width: i32, height: i32, fps: i32) -> AssemblyClip {
    AssemblyClip {
        video_id,
        video_type: "chapter_video".to_string(),
        shot_number: video_id as i32,
        sort_order: video_id as f64 * 1000.0,
        insert_after_video_id: None,
        path: PathBuf::from(format!("video_{video_id}.mp4")),
        file_path: format!("video_{video_id}.mp4"),
        duration_secs: 8.0,
        width,
        height,
        fps,
    }
}

fn build_test_assembly_cache_manifest(
    clips: &[AssemblyClip],
    audio_video_ids: &[i64],
) -> AssemblyCacheManifest {
    build_assembly_cache_manifest_with_audio_probe(
        1,
        1,
        "first_frame_chain",
        1280,
        720,
        24,
        clips,
        &|path| {
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("");
            Ok(audio_video_ids.iter().any(|id| {
                file_name == format!("clip_{id}.mp4") || file_name == format!("video_{id}.mp4")
            }))
        },
    )
    .expect("build test manifest")
}
