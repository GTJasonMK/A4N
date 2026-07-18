use super::*;
use chrono::Utc;

fn video_for_tail_capture(video_type: &str, workflow_mode: Option<&str>) -> generated_video::Model {
    let now = Utc::now().naive_utc();
    generated_video::Model {
        id: 42,
        project_id: 1,
        chapter_id: Some(1),
        video_type: video_type.to_string(),
        workflow_mode: workflow_mode.map(ToString::to_string),
        file_path: None,
        thumbnail_path: None,
        prompt: "prompt".to_string(),
        negative_prompt: None,
        model: "model".to_string(),
        width: 1280,
        height: 720,
        duration_secs: 5.0,
        fps: 24,
        file_size: None,
        source_video_id: None,
        source_image_id: None,
        status: "completed".to_string(),
        provider_job_id: None,
        error_message: None,
        metadata_json: None,
        is_favorite: false,
        created_at: now,
        updated_at: now,
    }
}

fn canvas_video_with_metadata(metadata_json: Option<serde_json::Value>) -> generated_video::Model {
    let mut video = video_for_tail_capture("canvas_video", Some("canvas_video"));
    video.chapter_id = None;
    video.status = "running".to_string();
    video.provider_job_id = Some("provider-job".to_string());
    video.metadata_json = metadata_json;
    video
}

#[test]
fn required_video_chapter_id_rejects_missing_id() {
    let err = required_video_chapter_id(None, 42).expect_err("chapter id should be required");

    assert!(err.contains("视频 #42 缺少 chapter_id"));
}

#[test]
fn tail_frame_capture_applies_to_chapter_video_workflows() {
    for workflow in ["boundary_frames", "first_frame_chain", "chat_video"] {
        assert!(should_capture_video_tail_frame(&video_for_tail_capture(
            "chapter_video",
            Some(workflow),
        )));
    }
    assert!(!should_capture_video_tail_frame(&video_for_tail_capture(
        "reference_media_video",
        Some("reference_media"),
    )));
    assert!(!should_capture_video_tail_frame(&video_for_tail_capture(
        "chapter_video",
        Some("reference_media"),
    )));
}

#[test]
fn completed_provider_metadata_omits_null_provider_metadata() {
    assert_eq!(completed_provider_metadata(Value::Null, None, None), None);
    assert_eq!(
        completed_provider_metadata(json!({"job_id": "provider-job"}), None, None),
        Some(json!({"job_id": "provider-job"}))
    );

    assert_eq!(
        completed_provider_metadata(Value::Null, Some(json!({"image_id": 9})), None),
        Some(json!({"tail_frame": {"image_id": 9}}))
    );
    assert_eq!(
        completed_provider_metadata(
            Value::Null,
            Some(json!({"image_id": 9})),
            Some("tail capture failed".to_string()),
        ),
        Some(json!({
            "tail_frame": {"image_id": 9},
            "tail_frame_capture_error": "tail capture failed",
        }))
    );
    assert_eq!(
        completed_provider_metadata(
            json!({"job_id": "provider-job"}),
            Some(json!({"image_id": 9})),
            Some("tail capture failed".to_string()),
        ),
        Some(json!({
            "provider": {"job_id": "provider-job"},
            "tail_frame": {"image_id": 9},
            "tail_frame_capture_error": "tail capture failed",
        }))
    );
}

#[test]
fn cleanup_saved_video_files_removes_video_and_thumbnail() {
    let root = std::env::temp_dir().join(format!(
        "a4n-video-cleanup-test-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let thumbnail_rel = "projects/1/chapter-1/thumb.jpg";
    let thumbnail_path = root.join(thumbnail_rel);
    let video_path = root.join("projects/1/chapter-1/video.mp4");
    std::fs::create_dir_all(thumbnail_path.parent().expect("thumbnail parent"))
        .expect("create thumbnail dir");
    std::fs::write(&thumbnail_path, b"thumbnail").expect("write thumbnail");
    std::fs::write(&video_path, b"video").expect("write video");

    let cleanup_error = cleanup_saved_video_files(&root, &video_path, Some(thumbnail_rel));

    assert_eq!(cleanup_error, None);
    assert!(!video_path.exists());
    assert!(!thumbnail_path.exists());
    std::fs::remove_dir_all(&root).expect("remove temp cleanup dir");
}

#[test]
fn saved_video_failure_result_returns_failed_record_when_cleanup_succeeds() {
    assert_eq!(
        saved_video_failure_result(42, "provider failure".to_string(), None),
        Ok(42)
    );
}

#[test]
fn saved_video_failure_result_preserves_primary_and_cleanup_errors() {
    let error = saved_video_failure_result(
        42,
        "provider failure".to_string(),
        Some("cleanup failure".to_string()),
    )
    .expect_err("cleanup failure must remain visible");

    assert_eq!(error, "provider failure；cleanup failure");
}

#[test]
fn canvas_provider_poll_requires_explicit_config_id() {
    let chapter_video = video_for_tail_capture("chapter_video", Some("boundary_frames"));
    assert_eq!(
        provider_poll_config_id(&chapter_video).expect("chapter videos may use default config"),
        None
    );

    let err = provider_poll_config_id(&canvas_video_with_metadata(Some(json!({
        "canvas": {"workflow_run_id": 1, "graph_id": 2, "node_id": 3}
    }))))
    .expect_err("canvas provider poll must not fall back to default config");
    assert!(err.contains("缺少 metadata_json.config_id"));

    let err = provider_poll_config_id(&canvas_video_with_metadata(Some(json!({
        "config_id": 0,
        "canvas": {"workflow_run_id": 1, "graph_id": 2, "node_id": 3}
    }))))
    .expect_err("canvas provider poll should reject invalid config id");
    assert!(err.contains("metadata_json.config_id 必须是正整数"));

    assert_eq!(
        provider_poll_config_id(&canvas_video_with_metadata(Some(json!({
            "config_id": 9,
            "canvas": {"workflow_run_id": 1, "graph_id": 2, "node_id": 3}
        }))))
        .expect("valid canvas provider poll config should parse"),
        Some(9)
    );
}

#[test]
fn chapter_provider_poll_rejects_malformed_config_id() {
    let mut video = video_for_tail_capture("chapter_video", Some("boundary_frames"));

    video.metadata_json = Some(json!({ "config_id": "9" }));
    let err = provider_poll_config_id(&video)
        .expect_err("string config_id must not fall back to default config");
    assert!(err.contains("metadata_json.config_id 必须是正整数"));

    video.metadata_json = Some(json!({ "config_id": 0 }));
    let err = provider_poll_config_id(&video)
        .expect_err("non-positive config_id must not reach config lookup");
    assert!(err.contains("metadata_json.config_id 必须是正整数"));

    video.metadata_json = Some(json!({ "config_id": 9 }));
    assert_eq!(
        provider_poll_config_id(&video).expect("positive config_id should be reused"),
        Some(9)
    );
}

#[test]
fn canvas_provider_workflow_context_requires_canvas_metadata() {
    let chapter_video = video_for_tail_capture("chapter_video", Some("boundary_frames"));
    assert_eq!(
        canvas_video_workflow_context(&chapter_video)
            .expect("non-canvas videos do not need canvas workflow context"),
        None
    );

    let err = canvas_video_workflow_context(&canvas_video_with_metadata(None))
        .expect_err("canvas video should require metadata_json");
    assert!(err.contains("缺少 metadata_json"));

    let err = canvas_video_workflow_context(&canvas_video_with_metadata(Some(json!({
        "config_id": 9
    }))))
    .expect_err("canvas video should require canvas metadata object");
    assert!(err.contains("缺少 metadata_json.canvas"));

    let err = canvas_video_workflow_context(&canvas_video_with_metadata(Some(json!({
        "config_id": 9,
        "canvas": {"workflow_run_id": 1, "graph_id": 2}
    }))))
    .expect_err("canvas video should require node id");
    assert!(err.contains("metadata_json.canvas.node_id"));

    assert_eq!(
        canvas_video_workflow_context(&canvas_video_with_metadata(Some(json!({
            "config_id": 9,
            "canvas": {"workflow_run_id": 11, "graph_id": 22, "node_id": 33}
        }))))
        .expect("valid canvas workflow context should parse"),
        Some(CanvasVideoWorkflowContext {
            workflow_run_id: 11,
            graph_id: 22,
            node_id: 33,
        })
    );
}

#[test]
fn terminal_video_status_matches_completed_failed_cancelled() {
    assert!(is_terminal_generated_video_status("completed"));
    assert!(is_terminal_generated_video_status("failed"));
    assert!(is_terminal_generated_video_status("cancelled"));
    assert!(!is_terminal_generated_video_status("running"));
    assert!(!is_terminal_generated_video_status("pending"));
    assert!(!is_terminal_generated_video_status("interrupted"));
}

#[test]
fn active_provider_job_requires_non_terminal_status_and_nonblank_id() {
    assert_eq!(
        active_provider_job_id(Some(" job-1 "), "running"),
        Some("job-1")
    );
    assert_eq!(
        active_provider_job_id(Some("job-1"), "pending"),
        Some("job-1")
    );
    assert_eq!(
        active_provider_job_id(Some("job-1"), "interrupted"),
        Some("job-1")
    );
    assert_eq!(active_provider_job_id(Some("job-1"), "completed"), None);
    assert_eq!(active_provider_job_id(Some("job-1"), "failed"), None);
    assert_eq!(active_provider_job_id(Some("job-1"), "cancelled"), None);
    assert_eq!(active_provider_job_id(Some(" \t "), "running"), None);
    assert_eq!(active_provider_job_id(None, "running"), None);
}

#[test]
fn has_active_provider_job_matches_lifecycle_status_semantics() {
    assert!(has_active_provider_job(Some(" job-1 "), "running"));
    assert!(has_active_provider_job(Some("job-1"), "pending"));
    assert!(has_active_provider_job(Some("job-1"), "interrupted"));
    assert!(!has_active_provider_job(Some("job-1"), "completed"));
    assert!(!has_active_provider_job(Some("job-1"), "failed"));
    assert!(!has_active_provider_job(Some("job-1"), "cancelled"));
    assert!(!has_active_provider_job(Some(" \t "), "running"));
    assert!(!has_active_provider_job(None, "running"));
}

#[test]
fn provider_workflow_checkpoint_completed_only_for_terminal_provider_statuses() {
    assert!(is_terminal_provider_workflow_checkpoint_status(
        WORKFLOW_STATUS_COMPLETED
    ));
    assert!(is_terminal_provider_workflow_checkpoint_status(
        WORKFLOW_STATUS_FAILED
    ));
    assert!(!is_terminal_provider_workflow_checkpoint_status(
        WORKFLOW_STATUS_RUNNING
    ));
    assert!(!is_terminal_provider_workflow_checkpoint_status(
        WORKFLOW_STATUS_INTERRUPTED
    ));
}

#[test]
fn normalized_provider_job_id_trims_and_rejects_blank_values() {
    assert_eq!(normalized_provider_job_id(Some(" job-1 ")), Some("job-1"));
    assert_eq!(normalized_provider_job_id(Some(" \n\t ")), None);
    assert_eq!(normalized_provider_job_id(Some("")), None);
    assert_eq!(normalized_provider_job_id(None), None);
}
