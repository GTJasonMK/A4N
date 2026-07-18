use super::assembly::{
    ApplyAssemblyPostprocessEditInput, AssemblyCacheManifest, AssemblyClip,
    active_assembly_push_source_ids, assembly_cache_matches, assembly_clip_shot_number,
    assembly_clip_sort_order, assembly_dimensions, assembly_fps,
    assembly_insert_metadata_with_position, assembly_push_rows,
    build_assembly_cache_manifest_with_audio_probe, ensure_active_pushes_match_clip_prefix,
    ensure_insert_position_not_inside_pushed_prefix, next_assembly_insert_sort_order,
    parse_postprocess_edit_metadata, postprocess_edit_range, selected_assembly_push_source_clips,
    validate_assembly_workflow_mode, validate_postprocess_operation, write_assembly_cache_manifest,
};
use super::imports::{
    delete_video_record_then_files, import_video_extension, import_video_source_path,
    imported_chapter_video_metadata, trimmed_video_metadata,
    validate_reference_media_import_options, validate_video_trim_range, video_delete_file_paths,
    video_probe_error_message,
};
use a4n_core::models::ai::generated_video;
use a4n_core::models::chapter::chapter_video_prompt;
use chrono::Utc;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .expect("system clock must be after UNIX_EPOCH");
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
}

fn video_with_metadata(id: i64, metadata_json: Option<Value>) -> generated_video::Model {
    let now = Utc::now().naive_utc();
    generated_video::Model {
        id,
        project_id: 1,
        chapter_id: Some(1),
        video_type: "chapter_video".to_string(),
        workflow_mode: Some("first_frame_chain".to_string()),
        file_path: Some(format!("video_{id}.mp4")),
        thumbnail_path: None,
        prompt: "prompt".to_string(),
        negative_prompt: None,
        model: "model".to_string(),
        width: 1281,
        height: 721,
        duration_secs: 8.0,
        fps: 24,
        file_size: None,
        source_video_id: None,
        source_image_id: None,
        status: "completed".to_string(),
        provider_job_id: None,
        error_message: None,
        metadata_json,
        is_favorite: false,
        created_at: now,
        updated_at: now,
    }
}

fn prompt_with_metadata(id: i64, metadata_json: Option<Value>) -> chapter_video_prompt::Model {
    let now = Utc::now().naive_utc();
    chapter_video_prompt::Model {
        id,
        project_id: 1,
        chapter_id: 1,
        style: Some("电影感写实".to_string()),
        prompt_type: "video_prompt".to_string(),
        workflow_mode: "first_frame_chain".to_string(),
        shot_number: 2,
        title: Some("镜头标题".to_string()),
        scene_summary: Some("镜头摘要".to_string()),
        visual_prompt: "visual prompt".to_string(),
        motion_prompt: Some("motion prompt".to_string()),
        negative_prompt: None,
        duration_secs: 4,
        aspect_ratio: Some("16:9".to_string()),
        reference_image_ids_json: None,
        status: "pending".to_string(),
        generated_video_id: None,
        metadata_json,
        created_at: now,
        updated_at: now,
    }
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

fn assembly_push_video(
    id: i64,
    push_index: i32,
    status: &str,
    source_video_ids: Vec<i64>,
) -> generated_video::Model {
    let mut video = video_with_metadata(
        id,
        Some(json!({
            "assembly_push": {
                "status": status,
                "push_index": push_index,
                "source_video_ids": source_video_ids
            }
        })),
    );
    video.video_type = "assembly_push".to_string();
    video.prompt = format!("push {push_index}");
    video
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
