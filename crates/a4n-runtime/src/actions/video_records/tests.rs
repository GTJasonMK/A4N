use super::assembly::{
    ApplyAssemblyPostprocessEditInput, AssemblyCacheClip, AssemblyCacheManifest, AssemblyClip,
    active_assembly_push_source_ids, assembly_cache_matches, assembly_clip_shot_number,
    assembly_clip_sort_order, assembly_dimensions, assembly_filter_complex, assembly_fps,
    assembly_insert_metadata_with_position, assembly_push_rows,
    build_assembly_cache_manifest_with_audio_probe, ensure_active_pushes_match_clip_prefix,
    ensure_insert_position_not_inside_pushed_prefix, next_assembly_insert_sort_order,
    parse_postprocess_edit_metadata, postprocess_edit_range, selected_assembly_push_source_clips,
    validate_assembly_workflow_mode, validate_postprocess_operation, write_assembly_cache_manifest,
};
use super::imports::{
    delete_video_record_then_files, import_video_extension, import_video_source_path,
    imported_chapter_video_metadata, parse_frame_rate, parse_video_probe_properties,
    trimmed_video_metadata, validate_reference_media_import_options, validate_video_trim_range,
    video_delete_file_paths,
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

fn build_test_assembly_filter_manifest(
    clips: &[AssemblyClip],
    audio_video_ids: &[i64],
) -> AssemblyCacheManifest {
    let clips = clips
        .iter()
        .map(|clip| {
            let has_audio = audio_video_ids.contains(&clip.video_id);
            AssemblyCacheClip {
                video_id: clip.video_id,
                video_type: clip.video_type.clone(),
                shot_number: clip.shot_number,
                sort_order: clip.sort_order,
                insert_after_video_id: clip.insert_after_video_id,
                file_path: clip.file_path.clone(),
                file_size: 0,
                modified_unix_millis: 0,
                has_audio,
                duration_secs: clip.duration_secs,
                width: clip.width,
                height: clip.height,
                fps: clip.fps,
            }
        })
        .collect::<Vec<_>>();
    AssemblyCacheManifest {
        version: 2,
        project_id: 1,
        chapter_id: 1,
        workflow_mode: "first_frame_chain".to_string(),
        width: 1280,
        height: 720,
        fps: 24,
        audio_enabled: clips.iter().any(|clip| clip.has_audio),
        clips,
    }
}

#[test]
fn video_delete_file_paths_ignore_blank_optional_thumbnail() {
    let data_dir = unique_temp_dir("a4n_video_delete_blank_thumb");
    let video_path = data_dir.join("videos/chapter_1/video.mp4");
    std::fs::create_dir_all(video_path.parent().expect("video parent")).expect("create parent");
    std::fs::write(&video_path, b"video").expect("write video");

    let paths = video_delete_file_paths(
        &data_dir,
        Some("videos/chapter_1/video.mp4"),
        Some(" \n\t "),
    )
    .expect("blank optional thumbnail should be ignored");

    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].0, "视频文件");
    assert_eq!(
        paths[0].1,
        video_path.canonicalize().expect("canonical video")
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn video_delete_file_paths_deduplicate_video_and_thumbnail_same_file() {
    let data_dir = unique_temp_dir("a4n_video_delete_dedup");
    let video_path = data_dir.join("videos/chapter_1/video.mp4");
    std::fs::create_dir_all(video_path.parent().expect("video parent")).expect("create parent");
    std::fs::write(&video_path, b"video").expect("write video");

    let paths = video_delete_file_paths(
        &data_dir,
        Some("videos/chapter_1/video.mp4"),
        Some("videos/chapter_1/video.mp4"),
    )
    .expect("same relative file should resolve once");

    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].0, "视频文件");
    assert_eq!(
        paths[0].1,
        video_path.canonicalize().expect("canonical video")
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn video_delete_file_paths_skip_missing_files() {
    let data_dir = unique_temp_dir("a4n_video_delete_missing");
    std::fs::create_dir_all(&data_dir).expect("create data dir");

    let paths = video_delete_file_paths(
        &data_dir,
        Some("videos/chapter_1/missing.mp4"),
        Some("videos/chapter_1/missing-thumb.jpg"),
    )
    .expect("missing delete targets should be treated as already deleted");

    assert!(paths.is_empty());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn video_delete_file_paths_reject_existing_file_outside_data_dir() {
    let root = unique_temp_dir("a4n_video_delete_outside_root");
    let data_dir = root.join("data");
    let outside = root.join("outside.mp4");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::write(&outside, b"video").expect("write outside video");

    let err = video_delete_file_paths(&data_dir, Some(&outside.to_string_lossy()), None)
        .expect_err("existing outside file must not be accepted for deletion");

    assert!(err.contains("相对路径"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn video_delete_file_paths_reject_missing_traversal_path() {
    let root = unique_temp_dir("a4n_video_delete_missing_traversal");
    let data_dir = root.join("data");
    std::fs::create_dir_all(&data_dir).expect("create data dir");

    let err = video_delete_file_paths(&data_dir, Some("../missing.mp4"), None)
        .expect_err("missing traversal target must not be treated as already deleted");

    assert!(err.contains("不在应用数据目录内"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn video_delete_file_paths_reject_existing_directory() {
    let data_dir = unique_temp_dir("a4n_video_delete_directory");
    let directory = data_dir.join("videos/chapter_1/not_a_file.mp4");
    std::fs::create_dir_all(&directory).expect("create directory target");

    let err = video_delete_file_paths(&data_dir, Some("videos/chapter_1/not_a_file.mp4"), None)
        .expect_err("directory target must not be accepted as a file");

    assert_eq!(err, "视频文件不是文件");
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn validate_assembly_workflow_mode_accepts_known_modes() {
    assert_eq!(
        validate_assembly_workflow_mode("boundary_frames"),
        Ok("boundary_frames")
    );
    assert_eq!(
        validate_assembly_workflow_mode(" first_frame_chain "),
        Ok("first_frame_chain")
    );
    assert_eq!(
        validate_assembly_workflow_mode("reference_media"),
        Ok("reference_media")
    );
    assert_eq!(
        validate_assembly_workflow_mode("chat_video"),
        Ok("chat_video")
    );
}

#[test]
fn validate_assembly_workflow_mode_rejects_blank_or_unknown() {
    assert!(validate_assembly_workflow_mode(" ").is_err());
    assert!(validate_assembly_workflow_mode("unknown").is_err());
}

#[test]
fn assembly_clip_shot_number_requires_positive_integer_metadata() {
    assert_eq!(
        assembly_clip_shot_number(&video_with_metadata(1, Some(json!({ "shot_number": 3 }))))
            .expect("valid shot number"),
        3
    );

    for metadata in [
        None,
        Some(json!({})),
        Some(json!({ "shot_number": 0 })),
        Some(json!({ "shot_number": "3" })),
    ] {
        let err = assembly_clip_shot_number(&video_with_metadata(2, metadata))
            .expect_err("invalid shot number should fail");
        assert!(err.contains("缺少有效 shot_number"));
    }
}

#[test]
fn assembly_clip_sort_order_supports_insert_clips_between_shots() {
    let first = video_with_metadata(1, Some(json!({ "shot_number": 1 })));
    let second = video_with_metadata(2, Some(json!({ "shot_number": 2 })));
    let mut insert = video_with_metadata(
        3,
        Some(json!({
            "shot_number": 1,
            "assembly_insert": {
                "sort_order": 1500.0,
                "insert_after_video_id": 1,
                "insert_after_shot_number": 1
            }
        })),
    );
    insert.video_type = "assembly_insert".to_string();

    let mut ordered = vec![second, insert, first];
    ordered.sort_by(|left, right| {
        assembly_clip_sort_order(left)
            .expect("valid left order")
            .total_cmp(&assembly_clip_sort_order(right).expect("valid right order"))
    });

    assert_eq!(
        ordered
            .into_iter()
            .map(|video| video.id)
            .collect::<Vec<_>>(),
        vec![1, 3, 2]
    );
}

#[test]
fn assembly_clip_sort_order_rejects_malformed_insert_metadata() {
    let mut bad_insert_anchor = video_with_metadata(
        3,
        Some(json!({
            "shot_number": 1,
            "assembly_insert": {
                "sort_order": 1500.0,
                "insert_after_video_id": "1",
                "insert_after_shot_number": 1
            }
        })),
    );
    bad_insert_anchor.video_type = "assembly_insert".to_string();

    let err = assembly_clip_sort_order(&bad_insert_anchor)
        .expect_err("invalid insert_after_video_id should fail explicitly");
    assert!(err.contains("insert_after_video_id"));
    assert!(err.contains("必须为空或正整数"));

    let mut bad_insert_shot = video_with_metadata(
        4,
        Some(json!({
            "shot_number": 1,
            "assembly_insert": {
                "sort_order": 1500.0,
                "insert_after_video_id": 1,
                "insert_after_shot_number": 0
            }
        })),
    );
    bad_insert_shot.video_type = "assembly_insert".to_string();

    let err = assembly_clip_sort_order(&bad_insert_shot)
        .expect_err("invalid insert_after_shot_number should fail explicitly");
    assert!(err.contains("insert_after_shot_number"));
    assert!(err.contains("必须为空或正整数"));
}

#[test]
fn assembly_insert_metadata_with_position_rejects_malformed_base_metadata() {
    let err = assembly_insert_metadata_with_position(Some(json!("legacy")), 500.0, None, None)
        .expect_err("non-object metadata should fail");
    assert!(err.contains("metadata 必须是对象"));

    let err = assembly_insert_metadata_with_position(
        Some(json!({ "assembly_insert": "legacy" })),
        500.0,
        Some(1),
        Some(1),
    )
    .expect_err("non-object assembly_insert should fail");
    assert!(err.contains("assembly_insert metadata 必须是对象"));
}

#[test]
fn next_assembly_insert_sort_order_places_clip_after_anchor() {
    let clips = vec![
        assembly_clip(1, 1280, 720, 24),
        assembly_clip(2, 1280, 720, 24),
    ];

    let (order, shot_number) =
        next_assembly_insert_sort_order(&clips, Some(1)).expect("anchor exists");

    assert!(order > clips[0].sort_order);
    assert!(order < clips[1].sort_order);
    assert_eq!(shot_number, Some(1));
}

#[test]
fn next_assembly_insert_sort_order_places_clip_before_first_without_anchor() {
    let clips = vec![
        assembly_clip(1, 1280, 720, 24),
        assembly_clip(2, 1280, 720, 24),
    ];

    let (order, shot_number) =
        next_assembly_insert_sort_order(&clips, None).expect("start insert should be valid");

    assert!(order < clips[0].sort_order);
    assert_eq!(shot_number, None);
}

#[test]
fn next_assembly_insert_sort_order_rejects_missing_anchor() {
    let clips = vec![assembly_clip(1, 1280, 720, 24)];

    let err =
        next_assembly_insert_sort_order(&clips, Some(999)).expect_err("missing anchor should fail");

    assert!(err.contains("不在当前成片片段中"));
}

#[test]
fn assembly_push_rows_parse_completed_push_metadata_in_order() {
    let mut failed_push = assembly_push_video(99, 3, "pending", vec![9]);
    failed_push.status = "failed".to_string();
    let rows = vec![
        assembly_push_video(12, 2, "rejected", vec![3]),
        failed_push,
        assembly_push_video(11, 1, "accepted", vec![1, 2]),
        assembly_push_video(13, 3, "pending", vec![4]),
    ];

    let pushes = assembly_push_rows(&rows, "first_frame_chain").expect("valid pushes");

    assert_eq!(
        pushes.iter().map(|push| push.video.id).collect::<Vec<_>>(),
        vec![11, 12, 13]
    );
    assert_eq!(active_assembly_push_source_ids(&pushes), vec![1, 2, 4]);
}

#[test]
fn assembly_push_rows_reject_malformed_completed_push_metadata() {
    let mut missing_status = assembly_push_video(100, 4, "pending", vec![4]);
    if let Some(metadata) = missing_status.metadata_json.as_mut()
        && let Some(push) = metadata
            .get_mut("assembly_push")
            .and_then(serde_json::Value::as_object_mut)
    {
        push.remove("status");
    }
    let missing_status_err = assembly_push_rows(&[missing_status], "first_frame_chain")
        .expect_err("missing status must not become pending");
    assert!(missing_status_err.contains("缺少 assembly_push.status"));

    let invalid_status = assembly_push_video(101, 5, "done", vec![5]);
    let invalid_status_err = assembly_push_rows(&[invalid_status], "first_frame_chain")
        .expect_err("invalid status must fail visibly");
    assert!(invalid_status_err.contains("assembly_push.status 无效"));

    let empty_sources = {
        let mut video = video_with_metadata(
            102,
            Some(json!({
                "assembly_push": {
                    "status": "pending",
                    "push_index": 6,
                    "source_video_ids": []
                }
            })),
        );
        video.video_type = "assembly_push".to_string();
        video
    };
    let empty_sources_err = assembly_push_rows(&[empty_sources], "first_frame_chain")
        .expect_err("empty source ids must fail visibly");
    assert!(empty_sources_err.contains("source_video_ids 不能为空"));
}

#[test]
fn ensure_active_pushes_match_clip_prefix_accepts_contiguous_prefix() {
    let clips = vec![
        assembly_clip(1, 1280, 720, 24),
        assembly_clip(2, 1280, 720, 24),
        assembly_clip(3, 1280, 720, 24),
    ];

    ensure_active_pushes_match_clip_prefix(&clips, &[1, 2])
        .expect("active push ids should match the clip prefix");
}

#[test]
fn ensure_active_pushes_match_clip_prefix_rejects_gap_or_overflow() {
    let clips = vec![
        assembly_clip(1, 1280, 720, 24),
        assembly_clip(2, 1280, 720, 24),
    ];

    let gap = ensure_active_pushes_match_clip_prefix(&clips, &[1, 3])
        .expect_err("gap should break pushed prefix continuity");
    assert!(gap.contains("不连续"));

    let overflow = ensure_active_pushes_match_clip_prefix(&clips, &[1, 2, 3])
        .expect_err("pushed source count cannot exceed current clips");
    assert!(overflow.contains("超过当前成片片段数量"));
}

#[test]
fn selected_assembly_push_source_clips_limits_range_by_end_video_id() {
    let pending = vec![
        assembly_clip(3, 1280, 720, 24),
        assembly_clip(4, 1280, 720, 24),
        assembly_clip(5, 1280, 720, 24),
    ];

    let single = selected_assembly_push_source_clips(&pending, Some(3))
        .expect("first pending clip can be pushed alone");
    assert_eq!(
        single.iter().map(|clip| clip.video_id).collect::<Vec<_>>(),
        vec![3]
    );

    let partial = selected_assembly_push_source_clips(&pending, Some(4))
        .expect("range should end at selected pending clip");
    assert_eq!(
        partial.iter().map(|clip| clip.video_id).collect::<Vec<_>>(),
        vec![3, 4]
    );

    let all = selected_assembly_push_source_clips(&pending, None)
        .expect("blank selection pushes all pending clips");
    assert_eq!(
        all.iter().map(|clip| clip.video_id).collect::<Vec<_>>(),
        vec![3, 4, 5]
    );
}

#[test]
fn selected_assembly_push_source_clips_rejects_non_pending_end_video() {
    let pending = vec![
        assembly_clip(3, 1280, 720, 24),
        assembly_clip(4, 1280, 720, 24),
    ];

    let err = selected_assembly_push_source_clips(&pending, Some(2))
        .expect_err("already-pushed or skipped end clip should be rejected");

    assert!(err.contains("不在可推送后续片段中"));
}

#[test]
fn ensure_insert_position_not_inside_pushed_prefix_blocks_start_and_middle() {
    let clips = vec![
        assembly_clip(1, 1280, 720, 24),
        assembly_clip(2, 1280, 720, 24),
        assembly_clip(3, 1280, 720, 24),
    ];
    let rows = vec![assembly_push_video(11, 1, "accepted", vec![1, 2])];
    let pushes = assembly_push_rows(&rows, "first_frame_chain").expect("valid pushes");

    let start_err = ensure_insert_position_not_inside_pushed_prefix(&clips, &pushes, None)
        .expect_err("start insert should be blocked after any active push");
    assert!(start_err.contains("开头插入"));

    let middle_err = ensure_insert_position_not_inside_pushed_prefix(&clips, &pushes, Some(1))
        .expect_err("middle insert should be blocked inside pushed prefix");
    assert!(middle_err.contains("中间插入"));

    ensure_insert_position_not_inside_pushed_prefix(&clips, &pushes, Some(2))
        .expect("insert after the pushed prefix tail should be allowed");
    ensure_insert_position_not_inside_pushed_prefix(&clips, &pushes, Some(3))
        .expect("insert after unpushed clips should be allowed");
}

#[test]
fn ensure_insert_position_not_inside_pushed_prefix_ignores_rejected_pushes() {
    let clips = vec![assembly_clip(1, 1280, 720, 24)];
    let rows = vec![assembly_push_video(11, 1, "rejected", vec![1])];
    let pushes = assembly_push_rows(&rows, "first_frame_chain").expect("valid pushes");

    ensure_insert_position_not_inside_pushed_prefix(&clips, &pushes, None)
        .expect("rejected pushes should not lock the assembly prefix");
}

#[test]
fn validate_postprocess_operation_accepts_only_supported_operations() {
    assert_eq!(validate_postprocess_operation(" insert "), Ok("insert"));
    assert_eq!(validate_postprocess_operation("delete"), Ok("delete"));
    assert_eq!(validate_postprocess_operation("replace"), Ok("replace"));
    assert!(validate_postprocess_operation(" ").is_err());
    assert!(validate_postprocess_operation("split").is_err());
}

#[test]
fn parse_postprocess_edit_metadata_requires_valid_version_chain_fields() {
    let mut valid = video_with_metadata(
        21,
        Some(json!({
            "postprocess_edit": {
                "push_video_id": 11,
                "base_video_id": 12,
                "operation": "replace",
                "source_video_id": 13
            }
        })),
    );
    valid.video_type = "assembly_postprocess_edit".to_string();

    let parsed = parse_postprocess_edit_metadata(&valid).expect("valid metadata should parse");
    assert_eq!(parsed.push_video_id, 11);
    assert_eq!(parsed.base_video_id, 12);

    let mut missing_push = video_with_metadata(
        22,
        Some(json!({
            "postprocess_edit": {
                "base_video_id": 12,
                "operation": "delete"
            }
        })),
    );
    missing_push.video_type = "assembly_postprocess_edit".to_string();
    let err = parse_postprocess_edit_metadata(&missing_push)
        .expect_err("missing push_video_id should fail");
    assert!(err.contains("push_video_id"));
    assert!(err.contains("必须是正整数"));

    let mut bad_operation = video_with_metadata(
        23,
        Some(json!({
            "postprocess_edit": {
                "push_video_id": 11,
                "base_video_id": 12,
                "operation": "merge"
            }
        })),
    );
    bad_operation.video_type = "assembly_postprocess_edit".to_string();
    let err =
        parse_postprocess_edit_metadata(&bad_operation).expect_err("unknown operation should fail");
    assert!(err.contains("后处理操作无效"));

    let mut bad_source = video_with_metadata(
        24,
        Some(json!({
            "postprocess_edit": {
                "push_video_id": 11,
                "base_video_id": 12,
                "operation": "insert",
                "source_video_id": 0
            }
        })),
    );
    bad_source.video_type = "assembly_postprocess_edit".to_string();
    let err = parse_postprocess_edit_metadata(&bad_source)
        .expect_err("invalid source_video_id should fail");
    assert!(err.contains("source_video_id"));
    assert!(err.contains("必须为空或正整数"));
}

#[test]
fn postprocess_edit_range_validates_insert_and_delete_boundaries() {
    let mut base = video_with_metadata(1, Some(json!({ "shot_number": 1 })));
    base.duration_secs = 10.0;

    let insert = ApplyAssemblyPostprocessEditInput {
        operation: "insert".to_string(),
        at_secs: Some(9.5),
        start_secs: None,
        end_secs: None,
        source_video_id: None,
        source_file_name: None,
        source_mime_type: None,
        source_data_base64: None,
    };
    assert_eq!(
        postprocess_edit_range(&base, &insert, "insert", None)
            .expect("insert at finite time should be valid"),
        (9.5, 9.5, None)
    );

    let delete = ApplyAssemblyPostprocessEditInput {
        operation: "delete".to_string(),
        at_secs: None,
        start_secs: Some(2.0),
        end_secs: Some(4.0),
        source_video_id: None,
        source_file_name: None,
        source_mime_type: None,
        source_data_base64: None,
    };
    assert_eq!(
        postprocess_edit_range(&base, &delete, "delete", None)
            .expect("forward delete range should be valid"),
        (2.0, 4.0, None)
    );

    let bad_insert = ApplyAssemblyPostprocessEditInput {
        operation: "insert".to_string(),
        at_secs: Some(10.5),
        start_secs: None,
        end_secs: None,
        source_video_id: None,
        source_file_name: None,
        source_mime_type: None,
        source_data_base64: None,
    };
    assert!(
        postprocess_edit_range(&base, &bad_insert, "insert", None)
            .expect_err("insert beyond duration should fail")
            .contains("超出当前片段范围")
    );

    let bad_delete = ApplyAssemblyPostprocessEditInput {
        operation: "delete".to_string(),
        at_secs: None,
        start_secs: Some(4.0),
        end_secs: Some(2.0),
        source_video_id: None,
        source_file_name: None,
        source_mime_type: None,
        source_data_base64: None,
    };
    assert!(
        postprocess_edit_range(&base, &bad_delete, "delete", None)
            .expect_err("backward delete range should fail")
            .contains("时间范围无效")
    );
}

#[test]
fn parse_frame_rate_accepts_fractional_and_plain_values() {
    assert_eq!(parse_frame_rate("24000/1001"), Some(24));
    assert_eq!(parse_frame_rate("30"), Some(30));
    assert_eq!(parse_frame_rate("0/0"), None);
}

#[test]
fn parse_video_probe_properties_accepts_valid_video_stream() {
    let properties = parse_video_probe_properties(
        br#"{
            "streams": [
                {
                    "width": 1920,
                    "height": 1080,
                    "r_frame_rate": "30000/1001"
                }
            ],
            "format": {
                "duration": "12.345"
            }
        }"#,
        "导入视频",
    )
    .expect("valid ffprobe output should parse");

    assert_eq!(properties.width, 1920);
    assert_eq!(properties.height, 1080);
    assert!((properties.duration_secs - 12.345).abs() < f32::EPSILON);
    assert_eq!(properties.fps, 30);
}

#[test]
fn parse_video_probe_properties_rejects_invalid_json() {
    let err = parse_video_probe_properties(b"{not-json", "导入参考媒体视频")
        .expect_err("invalid ffprobe JSON should fail");

    assert!(err.contains("导入参考媒体视频元数据不是有效 JSON"));
}

#[test]
fn parse_video_probe_properties_rejects_missing_video_stream() {
    let err = parse_video_probe_properties(
        br#"{
            "streams": [],
            "format": {
                "duration": "8.0"
            }
        }"#,
        "导入视频",
    )
    .expect_err("missing video stream should fail");

    assert_eq!(err, "导入视频缺少视频流");
}

#[test]
fn parse_video_probe_properties_rejects_missing_duration() {
    let err = parse_video_probe_properties(
        br#"{
            "streams": [
                {
                    "width": 1280,
                    "height": 720,
                    "r_frame_rate": "24"
                }
            ],
            "format": {
                "duration": "0"
            }
        }"#,
        "插入视频",
    )
    .expect_err("missing positive duration should fail");

    assert_eq!(err, "插入视频缺少有效时长");
}

#[test]
fn assembly_dimensions_use_first_clip_and_force_even_values() {
    let clips = vec![
        assembly_clip(1, 1281, 721, 25),
        assembly_clip(2, 1920, 1080, 30),
    ];

    assert_eq!(assembly_dimensions(&clips), (1282, 722));
    assert_eq!(assembly_fps(&clips), 25);
    assert_eq!(assembly_dimensions(&[]), (1280, 720));
    assert_eq!(assembly_fps(&[]), 24);
}

#[test]
fn assembly_filter_complex_normalizes_and_concats_video_streams() {
    let clips = vec![
        assembly_clip(1, 1280, 720, 24),
        assembly_clip(2, 1280, 720, 24),
    ];
    let manifest = build_test_assembly_filter_manifest(&clips, &[]);
    let (filter, has_audio) =
        assembly_filter_complex(&clips, 1280, 720, 24, &manifest).expect("build filter");

    assert!(!has_audio);
    assert!(filter.contains("[0:v:0]scale=1280:720"));
    assert!(filter.contains("[1:v:0]scale=1280:720"));
    assert!(filter.contains("fps=24"));
    assert!(filter.ends_with("[v0][v1]concat=n=2:v=1:a=0[outv]"));
}

#[test]
fn assembly_filter_complex_handles_single_video_only_clip() {
    let clips = vec![assembly_clip(1, 1280, 720, 24)];
    let manifest = build_test_assembly_filter_manifest(&clips, &[]);
    let (filter, has_audio) =
        assembly_filter_complex(&clips, 1280, 720, 24, &manifest).expect("build filter");

    assert!(!has_audio);
    assert!(filter.contains("[0:v:0]scale=1280:720"));
    assert!(filter.ends_with("[v0]concat=n=1:v=1:a=0[outv]"));
}

#[test]
fn assembly_filter_complex_preserves_audio_and_pads_silent_clips() {
    let clips = vec![
        assembly_clip(1, 1280, 720, 24),
        assembly_clip(2, 1280, 720, 24),
    ];
    let manifest = build_test_assembly_filter_manifest(&clips, &[1]);
    let (filter, has_audio) =
        assembly_filter_complex(&clips, 1280, 720, 24, &manifest).expect("build filter");

    assert!(has_audio);
    assert!(filter.contains("[0:a:0]aresample=async=1:first_pts=0"));
    assert!(filter.contains("anullsrc=channel_layout=stereo:sample_rate=48000"));
    assert!(filter.contains("atrim=duration=8.000"));
    assert!(filter.ends_with("[v0][a0][v1][a1]concat=n=2:v=1:a=1[outv][outa]"));
}

#[test]
fn assembly_cache_matches_same_manifest_and_output() {
    let data_dir = unique_temp_dir("a4n_assembly_cache_hit");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let clip_path = data_dir.join("clip_1.mp4");
    let output_path = data_dir.join("assembled.mp4");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");
    std::fs::write(&clip_path, b"clip").expect("write clip");
    std::fs::write(&output_path, b"assembled").expect("write output");
    let mut clip = assembly_clip(1, 1280, 720, 24);
    clip.path = clip_path;
    clip.file_path = "videos/chapter_1/clip_1.mp4".to_string();
    let manifest = build_test_assembly_cache_manifest(&[clip], &[]);
    write_assembly_cache_manifest(&manifest_path, &manifest).expect("write manifest");

    assert!(assembly_cache_matches(&output_path, &manifest_path, &manifest).expect("match cache"));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn assembly_cache_misses_after_clip_file_changes() {
    let data_dir = unique_temp_dir("a4n_assembly_cache_file_change");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let clip_path = data_dir.join("clip_1.mp4");
    let output_path = data_dir.join("assembled.mp4");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");
    std::fs::write(&clip_path, b"clip").expect("write clip");
    std::fs::write(&output_path, b"assembled").expect("write output");
    let mut clip = assembly_clip(1, 1280, 720, 24);
    clip.path = clip_path.clone();
    clip.file_path = "videos/chapter_1/clip_1.mp4".to_string();
    let original_manifest = build_test_assembly_cache_manifest(&[clip.clone()], &[]);
    write_assembly_cache_manifest(&manifest_path, &original_manifest).expect("write manifest");

    std::fs::write(&clip_path, b"clip changed").expect("change clip");
    let changed_manifest = build_test_assembly_cache_manifest(&[clip], &[]);

    assert!(
        !assembly_cache_matches(&output_path, &manifest_path, &changed_manifest)
            .expect("changed clip should miss")
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn assembly_cache_misses_after_clip_order_changes() {
    let data_dir = unique_temp_dir("a4n_assembly_cache_order_change");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let first_path = data_dir.join("clip_1.mp4");
    let second_path = data_dir.join("clip_2.mp4");
    let output_path = data_dir.join("assembled.mp4");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");
    std::fs::write(&first_path, b"clip one").expect("write first clip");
    std::fs::write(&second_path, b"clip two").expect("write second clip");
    std::fs::write(&output_path, b"assembled").expect("write output");
    let mut first = assembly_clip(1, 1280, 720, 24);
    first.path = first_path;
    first.file_path = "videos/chapter_1/clip_1.mp4".to_string();
    let mut second = assembly_clip(2, 1280, 720, 24);
    second.path = second_path;
    second.file_path = "videos/chapter_1/clip_2.mp4".to_string();
    let original_manifest =
        build_test_assembly_cache_manifest(&[first.clone(), second.clone()], &[]);
    write_assembly_cache_manifest(&manifest_path, &original_manifest).expect("write manifest");
    let reordered_manifest = build_test_assembly_cache_manifest(&[second, first], &[]);

    assert!(
        !assembly_cache_matches(&output_path, &manifest_path, &reordered_manifest)
            .expect("reordered clips should miss")
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn assembly_cache_rejects_invalid_manifest_shape() {
    let data_dir = unique_temp_dir("a4n_assembly_cache_invalid_manifest");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    let clip_path = data_dir.join("clip_1.mp4");
    let output_path = data_dir.join("assembled.mp4");
    let manifest_path = data_dir.join("assembled.mp4.cache.json");
    std::fs::write(&clip_path, b"clip").expect("write clip");
    std::fs::write(&output_path, b"assembled").expect("write output");
    std::fs::write(&manifest_path, br#"{"version":1}"#).expect("write invalid manifest");
    let mut clip = assembly_clip(1, 1280, 720, 24);
    clip.path = clip_path;
    clip.file_path = "videos/chapter_1/clip_1.mp4".to_string();
    let manifest = build_test_assembly_cache_manifest(&[clip], &[]);

    let err = assembly_cache_matches(&output_path, &manifest_path, &manifest)
        .expect_err("invalid manifest shape should be explicit");
    assert!(err.contains("格式无效"));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn import_video_source_path_accepts_supported_video_file() {
    let data_dir = unique_temp_dir("a4n_video_import_source");
    let video_path = data_dir.join("external.MP4");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::write(&video_path, b"video").expect("write video");

    let resolved =
        import_video_source_path(&video_path.to_string_lossy()).expect("mp4 should import");

    assert_eq!(
        resolved,
        video_path.canonicalize().expect("canonical import source")
    );
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn import_video_source_path_rejects_unsupported_extension() {
    let data_dir = unique_temp_dir("a4n_video_import_bad_ext");
    let video_path = data_dir.join("external.avi");
    std::fs::create_dir_all(&data_dir).expect("create data dir");
    std::fs::write(&video_path, b"video").expect("write video");

    let err = import_video_source_path(&video_path.to_string_lossy())
        .expect_err("unsupported extension should fail");

    assert!(err.contains("导入视频格式不支持"));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn import_video_extension_normalizes_supported_extensions() {
    for (name, expected) in [
        ("clip.mp4", "mp4"),
        ("clip.WEBM", "webm"),
        ("clip.mov", "mov"),
        ("clip.M4V", "m4v"),
    ] {
        assert_eq!(
            import_video_extension(PathBuf::from(name).as_path()).expect("supported video"),
            expected
        );
    }
}

#[test]
fn reference_media_import_options_reject_invalid_duration_and_aspect_ratio() {
    for duration_secs in [0, -1, 61] {
        let err = validate_reference_media_import_options(duration_secs, "16:9")
            .expect_err("reference media import duration must be bounded");

        assert_eq!(err, "参考媒体视频时长必须在 1 到 60 秒之间");
    }

    let err = validate_reference_media_import_options(6, "2:1")
        .expect_err("reference media import aspect ratio must be explicit");

    assert!(err.contains("不支持画幅 2:1"));
}

#[test]
fn reference_media_import_options_normalize_declared_aspect_ratio() {
    let options = validate_reference_media_import_options(6, " 3:4 ")
        .expect("valid reference media import options");

    assert_eq!(options.aspect_ratio, "3:4");
}

#[tokio::test]
async fn delete_video_record_then_files_removes_files_after_record_cleanup() {
    let data_dir = unique_temp_dir("a4n_video_delete_order_success");
    let video_path = data_dir.join("videos/chapter_1/video.mp4");
    std::fs::create_dir_all(video_path.parent().expect("video parent")).expect("create parent");
    std::fs::write(&video_path, b"video").expect("write video");
    let cleanup_called = Arc::new(AtomicBool::new(false));

    delete_video_record_then_files(vec![("视频文件", video_path.clone())], {
        let cleanup_called = cleanup_called.clone();
        let video_path = video_path.clone();
        || async move {
            assert!(video_path.exists());
            cleanup_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    })
    .await
    .expect("file cleanup should run after record deletion succeeds");

    assert!(!video_path.exists());
    assert!(cleanup_called.load(Ordering::SeqCst));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[tokio::test]
async fn delete_video_record_then_files_keeps_files_when_record_cleanup_fails() {
    let data_dir = unique_temp_dir("a4n_video_delete_record_failure");
    let video_path = data_dir.join("videos/chapter_1/video.mp4");
    std::fs::create_dir_all(video_path.parent().expect("video parent")).expect("create parent");
    std::fs::write(&video_path, b"video").expect("write video");

    let err = delete_video_record_then_files(vec![("视频文件", video_path.clone())], {
        || async move { Err("记录删除失败".to_string()) }
    })
    .await
    .expect_err("file cleanup must not run after record deletion fails");

    assert_eq!(err, "记录删除失败");
    assert!(video_path.exists());
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[tokio::test]
async fn delete_video_record_then_files_reports_file_delete_error_after_record_cleanup() {
    let data_dir = unique_temp_dir("a4n_video_delete_order_failure");
    let bad_target = data_dir.join("videos/chapter_1/not_a_file.mp4");
    std::fs::create_dir_all(&bad_target).expect("create directory target");
    let cleanup_called = Arc::new(AtomicBool::new(false));

    let err = delete_video_record_then_files(vec![("视频文件", bad_target.clone())], {
        let cleanup_called = cleanup_called.clone();
        || async move {
            cleanup_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    })
    .await
    .expect_err("file cleanup error should be returned after record cleanup");

    assert!(err.contains("视频文件删除失败"));
    assert!(bad_target.exists());
    assert!(cleanup_called.load(Ordering::SeqCst));
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn validate_video_trim_range_rejects_non_finite_values() {
    for (start_secs, end_secs) in [
        (f32::NAN, 2.0),
        (0.0, f32::NAN),
        (f32::INFINITY, f32::INFINITY),
        (0.0, f32::INFINITY),
    ] {
        let err = validate_video_trim_range(start_secs, end_secs)
            .expect_err("non-finite trim range should be rejected");

        assert_eq!(err, "裁剪时间范围无效");
    }
}

#[test]
fn validate_video_trim_range_accepts_finite_forward_range() {
    validate_video_trim_range(0.25, 1.5).expect("finite forward range should be valid");
}

#[test]
fn trimmed_video_metadata_uses_source_video_column_as_source_id_fact() {
    let source_without_context = video_with_metadata(7, Some(json!({ "prompt_id": 3 })));
    let metadata = trimmed_video_metadata(&source_without_context, 1.25, 4.5);

    assert_eq!(
        metadata
            .get("trim")
            .and_then(|value| value.get("start_secs")),
        Some(&json!(1.25))
    );
    assert_eq!(
        metadata.get("trim").and_then(|value| value.get("end_secs")),
        Some(&json!(4.5))
    );
    assert!(
        metadata.get("source_video_id").is_none(),
        "source_video_id is recorded by the generated_video column, not duplicated in metadata"
    );
    assert!(
        metadata.get("source").is_none(),
        "missing source context should not be recorded as source=null"
    );

    let source_context = json!({ "content_sha256": "same-source" });
    let source_with_context =
        video_with_metadata(8, Some(json!({ "source": source_context.clone() })));
    let metadata = trimmed_video_metadata(&source_with_context, 0.0, 2.0);

    assert_eq!(metadata.get("source"), Some(&source_context));
    assert!(metadata.get("source_video_id").is_none());
}

#[test]
fn imported_chapter_video_metadata_omits_missing_source_context() {
    let prompt_without_context = prompt_with_metadata(9, Some(json!({ "draft": true })));
    let metadata = imported_chapter_video_metadata(
        &prompt_without_context,
        "external.mp4",
        "mp4",
        "2026-06-29T19:19:00Z".to_string(),
    );

    assert_eq!(metadata.get("prompt_id"), Some(&json!(9)));
    assert_eq!(metadata.get("shot_number"), Some(&json!(2)));
    assert_eq!(
        metadata.get("source_prompt_type"),
        Some(&json!("video_prompt"))
    );
    assert!(
        metadata.get("source").is_none(),
        "missing prompt source context should not be recorded as source=null"
    );
    assert_eq!(
        metadata
            .get("import")
            .and_then(|value| value.get("source_file_name")),
        Some(&json!("external.mp4"))
    );

    let source_context = json!({ "content_sha256": "same-source" });
    let prompt_with_context =
        prompt_with_metadata(10, Some(json!({ "source": source_context.clone() })));
    let metadata = imported_chapter_video_metadata(
        &prompt_with_context,
        "external.webm",
        "webm",
        "2026-06-29T19:20:00Z".to_string(),
    );

    assert_eq!(metadata.get("source"), Some(&source_context));
    assert_eq!(
        metadata
            .get("import")
            .and_then(|value| value.get("source_extension")),
        Some(&json!("webm"))
    );
}
