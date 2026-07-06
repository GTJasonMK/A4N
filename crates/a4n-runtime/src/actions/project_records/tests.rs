use super::{
    FreeformGeneratedResetTargets, MissingCanvasRef, PipelineProjectState, ProgressCounts,
    ProjectProgressStatus, VideoProgressFacts, clear_reset_generated_ref_payload,
    comic_project_progress_from_counts, galgame_project_progress_from_counts,
    is_freeform_generated_media_asset, is_freeform_imported_canvas_image,
    is_freeform_imported_canvas_video, optional_text, project_list_offset, required_optional_text,
    reset_missing_canvas_ref_for_node, video_progress_facts_from_rows,
    video_project_progress_from_counts,
};
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    canvas::canvas_node,
    media::media_asset,
};
use chrono::{Duration, NaiveDate, NaiveDateTime};
use serde_json::json;

fn test_time(offset_seconds: i64) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .expect("valid test date")
        .and_hms_opt(0, 0, 0)
        .expect("valid test time")
        + Duration::seconds(offset_seconds)
}

fn video_prompt_row(
    id: i64,
    shot_number: i32,
    status: &str,
    generated_video_id: Option<i64>,
    updated_at: NaiveDateTime,
) -> chapter_video_prompt::Model {
    chapter_video_prompt::Model {
        id,
        project_id: 9,
        chapter_id: 3,
        style: None,
        prompt_type: "video_prompt".to_string(),
        workflow_mode: "first_frame_chain".to_string(),
        shot_number,
        title: Some(format!("镜头 {shot_number}")),
        scene_summary: None,
        visual_prompt: "visual".to_string(),
        motion_prompt: None,
        negative_prompt: None,
        duration_secs: 6,
        aspect_ratio: Some("16:9".to_string()),
        reference_image_ids_json: None,
        status: status.to_string(),
        generated_video_id,
        metadata_json: None,
        created_at: test_time(0),
        updated_at,
    }
}

fn generated_video_row(
    id: i64,
    shot_number: i32,
    prompt_id: i64,
    status: &str,
    updated_at: NaiveDateTime,
) -> generated_video::Model {
    generated_video::Model {
        id,
        project_id: 9,
        chapter_id: Some(3),
        video_type: "chapter_video".to_string(),
        workflow_mode: Some("first_frame_chain".to_string()),
        file_path: None,
        thumbnail_path: None,
        prompt: "prompt".to_string(),
        negative_prompt: None,
        model: "test-video-model".to_string(),
        width: 1024,
        height: 576,
        duration_secs: 6.0,
        fps: 24,
        file_size: None,
        source_video_id: None,
        source_image_id: None,
        status: status.to_string(),
        provider_job_id: None,
        error_message: None,
        metadata_json: Some(json!({
            "prompt_id": prompt_id,
            "shot_number": shot_number
        })),
        is_favorite: false,
        created_at: test_time(0),
        updated_at,
    }
}

fn canvas_image_row(
    model: &str,
    metadata_json: Option<serde_json::Value>,
) -> generated_image::Model {
    generated_image::Model {
        id: 41,
        project_id: 9,
        image_type: "canvas_image".to_string(),
        file_path: "projects/9/images/imported.png".to_string(),
        prompt: "导入画布图片".to_string(),
        model: model.to_string(),
        width: 1024,
        height: 1024,
        file_size: Some(4),
        ai_score: None,
        is_favorite: false,
        chapter_id: None,
        panel_id: None,
        metadata_json,
        created_at: test_time(0),
    }
}

fn canvas_video_row(
    model: &str,
    metadata_json: Option<serde_json::Value>,
) -> generated_video::Model {
    generated_video::Model {
        id: 42,
        project_id: 9,
        chapter_id: None,
        video_type: "canvas_video".to_string(),
        workflow_mode: Some("canvas_video".to_string()),
        file_path: Some("projects/9/videos/canvas.mp4".to_string()),
        thumbnail_path: Some("projects/9/videos/canvas.jpg".to_string()),
        prompt: "画布视频".to_string(),
        negative_prompt: None,
        model: model.to_string(),
        width: 1280,
        height: 720,
        duration_secs: 6.0,
        fps: 24,
        file_size: Some(4),
        source_video_id: None,
        source_image_id: None,
        status: "completed".to_string(),
        provider_job_id: None,
        error_message: None,
        metadata_json,
        is_favorite: false,
        created_at: test_time(0),
        updated_at: test_time(0),
    }
}

fn canvas_node_row(
    ref_type: Option<&str>,
    ref_id: Option<i64>,
    config_json: serde_json::Value,
) -> canvas_node::Model {
    canvas_node::Model {
        id: 7,
        graph_id: 3,
        node_kind: "VideoNode".to_string(),
        ref_type: ref_type.map(str::to_string),
        ref_id,
        config_json,
        position_json: json!({"x": 0, "y": 0}),
        state: "ready".to_string(),
        created_at: test_time(0),
        updated_at: test_time(0),
    }
}

fn media_asset_row(origin: &str) -> media_asset::Model {
    media_asset::Model {
        id: 43,
        project_id: 9,
        root_project_id: 9,
        source_snapshot_id: None,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        media_kind: "video".to_string(),
        origin: origin.to_string(),
        file_path: Some("projects/9/media/asset.mp4".to_string()),
        thumbnail_path: None,
        width: Some(1280),
        height: Some(720),
        duration_secs: Some(6.0),
        fps: Some(24.0),
        file_size: Some(4),
        checksum: Some("checksum".to_string()),
        status: "completed".to_string(),
        error_message: None,
        created_at: test_time(0),
        updated_at: test_time(0),
    }
}

#[test]
fn project_list_offset_rejects_overflow() {
    let err = project_list_offset(u64::MAX, u64::MAX)
        .expect_err("oversized pagination should be rejected");

    assert!(err.contains("项目分页参数过大"));
}

#[test]
fn project_list_offset_uses_one_based_pages() {
    assert_eq!(project_list_offset(1, 20), Ok(0));
    assert_eq!(project_list_offset(3, 20), Ok(40));
}

#[test]
fn required_optional_text_rejects_missing_or_blank_value() {
    assert!(required_optional_text(None, "项目类型").is_err());
    assert!(required_optional_text(Some(" \n\t "), "项目类型").is_err());
    assert_eq!(
        required_optional_text(Some(" novel_long "), "项目类型").expect("valid text"),
        "novel_long"
    );
}

#[test]
fn optional_text_allows_missing_or_blank_description() {
    assert_eq!(optional_text(None), "");
    assert_eq!(optional_text(Some(" \n\t ")), "");
    assert_eq!(optional_text(Some(" 简介 ")), "简介");
}

#[test]
fn freeform_reset_preserves_imported_canvas_image_and_video_records() {
    let imported_image = canvas_image_row(
        "external-import",
        Some(json!({
            "canvas_import": true,
            "source": "video_canvas"
        })),
    );
    let imported_video = canvas_video_row(
        "external-import",
        Some(json!({
            "canvas_import": {
                "external": true,
                "source_file_name": "clip.mp4"
            }
        })),
    );

    assert!(is_freeform_imported_canvas_image(&imported_image));
    assert!(is_freeform_imported_canvas_video(&imported_video));
}

#[test]
fn freeform_reset_deletes_generated_canvas_media_and_preserves_imported_assets() {
    let generated_image = canvas_image_row("image-provider", Some(json!({"canvas_import": true})));
    let generated_video =
        canvas_video_row("video-provider", Some(json!({"canvas": {"node_id": 1}})));
    let imported_asset = media_asset_row("imported");
    let generated_asset = media_asset_row("generated");
    let preview_asset = media_asset_row("assembled_preview");

    assert!(!is_freeform_imported_canvas_image(&generated_image));
    assert!(!is_freeform_imported_canvas_video(&generated_video));
    assert!(!is_freeform_generated_media_asset(&imported_asset));
    assert!(is_freeform_generated_media_asset(&generated_asset));
    assert!(is_freeform_generated_media_asset(&preview_asset));
}

#[test]
fn freeform_reset_marks_canvas_nodes_that_reference_deleted_generated_assets() {
    let targets = FreeformGeneratedResetTargets {
        image_ids: vec![11],
        video_ids: vec![22],
        media_asset_ids: vec![33],
        generation_spec_ids: vec![44],
        prompt_artifact_ids: vec![55],
        ..Default::default()
    };

    assert_eq!(
        reset_missing_canvas_ref_for_node(
            &canvas_node_row(
                Some("media_asset"),
                Some(33),
                json!({"title": "canonical media"})
            ),
            &targets,
        ),
        Some(MissingCanvasRef {
            ref_type: "media_asset",
            ref_id: 33,
            clear_node_ref: true
        })
    );
    assert_eq!(
        reset_missing_canvas_ref_for_node(
            &canvas_node_row(
                Some("inline"),
                None,
                json!({"generated_image_id": 11, "title": "generated image"})
            ),
            &targets,
        ),
        Some(MissingCanvasRef {
            ref_type: "generated_image",
            ref_id: 11,
            clear_node_ref: false
        })
    );
    assert_eq!(
        reset_missing_canvas_ref_for_node(
            &canvas_node_row(
                Some("inline"),
                None,
                json!({"video_ids": [22, 99], "title": "assembly"})
            ),
            &targets,
        ),
        Some(MissingCanvasRef {
            ref_type: "generated_video",
            ref_id: 22,
            clear_node_ref: false
        })
    );
    assert_eq!(
        reset_missing_canvas_ref_for_node(
            &canvas_node_row(
                Some("media_asset"),
                Some(44),
                json!({"generated_image_id": 99, "title": "preserved import"})
            ),
            &targets,
        ),
        None
    );
    assert_eq!(
        reset_missing_canvas_ref_for_node(
            &canvas_node_row(
                Some("generation_spec"),
                Some(44),
                json!({"title": "generation spec"})
            ),
            &targets,
        ),
        Some(MissingCanvasRef {
            ref_type: "generation_spec",
            ref_id: 44,
            clear_node_ref: true
        })
    );
    assert_eq!(
        reset_missing_canvas_ref_for_node(
            &canvas_node_row(
                Some("prompt_artifact"),
                Some(55),
                json!({"title": "prompt artifact"})
            ),
            &targets,
        ),
        Some(MissingCanvasRef {
            ref_type: "prompt_artifact",
            ref_id: 55,
            clear_node_ref: true
        })
    );
    assert_eq!(
        reset_missing_canvas_ref_for_node(
            &canvas_node_row(
                Some("inline"),
                None,
                json!({"generation_spec_id": 44, "prompt_artifact_id": 66})
            ),
            &targets,
        ),
        Some(MissingCanvasRef {
            ref_type: "generation_spec",
            ref_id: 44,
            clear_node_ref: false
        })
    );
    assert_eq!(
        reset_missing_canvas_ref_for_node(
            &canvas_node_row(
                Some("inline"),
                None,
                json!({"generation_spec_id": 99, "prompt_artifact_id": 55})
            ),
            &targets,
        ),
        Some(MissingCanvasRef {
            ref_type: "prompt_artifact",
            ref_id: 55,
            clear_node_ref: false
        })
    );
}

#[test]
fn freeform_reset_missing_cleanup_removes_only_reset_generated_refs() {
    let targets = FreeformGeneratedResetTargets {
        image_ids: vec![11],
        video_ids: vec![22],
        media_asset_ids: vec![33],
        generation_spec_ids: vec![44],
        prompt_artifact_ids: vec![55],
        ..Default::default()
    };
    let mut config = json!({
        "generated_image_id": 11,
        "source_image_id": 99,
        "reference_image_ids": [11, 12],
        "generated_video_id": 22,
        "video_ids": [22, 23],
        "media_asset_id": 33,
        "media_asset_ids": [33, 34],
        "generation_spec_id": 44,
        "generation_spec_ids": [44, 45],
        "prompt_artifact_id": 55,
        "prompt_artifact_ids": [55, 56],
        "title": "kept"
    });
    let object = config.as_object_mut().expect("object config");

    clear_reset_generated_ref_payload(object, &targets);

    assert!(config.get("generated_image_id").is_none());
    assert_eq!(config["source_image_id"], json!(99));
    assert_eq!(config["reference_image_ids"], json!([12]));
    assert!(config.get("generated_video_id").is_none());
    assert_eq!(config["video_ids"], json!([23]));
    assert!(config.get("media_asset_id").is_none());
    assert_eq!(config["media_asset_ids"], json!([34]));
    assert!(config.get("generation_spec_id").is_none());
    assert_eq!(config["generation_spec_ids"], json!([45]));
    assert!(config.get("prompt_artifact_id").is_none());
    assert_eq!(config["prompt_artifact_ids"], json!([56]));
    assert_eq!(config["title"], json!("kept"));
}

#[test]
fn comic_project_progress_reports_running_pipeline_before_idle_counts() {
    let mut pipeline = PipelineProjectState::default();
    pipeline.running = true;
    pipeline.message = Some("正在生成漫画页".to_string());
    let summary = comic_project_progress_from_counts(ProgressCounts::default(), pipeline);

    assert_eq!(summary.kind, "comic");
    assert_eq!(summary.stage, "manga_running");
    assert_eq!(summary.label, "漫画生成中");
    assert_eq!(summary.status, ProjectProgressStatus::Running);
    assert_eq!(summary.detail.as_deref(), Some("正在生成漫画页"));
}

#[test]
fn comic_project_progress_reports_completed_when_all_pages_complete() {
    let counts = ProgressCounts {
        completed: 4,
        ..Default::default()
    };
    let summary = comic_project_progress_from_counts(counts, PipelineProjectState::default());

    assert_eq!(summary.stage, "manga_ready");
    assert_eq!(summary.label, "漫画已生成");
    assert_eq!(summary.current, Some(4));
    assert_eq!(summary.total, Some(4));
    assert_eq!(summary.status, ProjectProgressStatus::Completed);
}

#[test]
fn video_project_progress_reports_failed_before_partial_completion() {
    let facts = VideoProgressFacts {
        prompt_counts: ProgressCounts {
            failed: 1,
            ..Default::default()
        },
        video_counts: ProgressCounts {
            completed: 2,
            ..Default::default()
        },
        video_prompt_total: 4,
        frame_prompt_total: 0,
        segment_plan_total: 0,
    };
    let summary = video_project_progress_from_counts(facts);

    assert_eq!(summary.stage, "video_failed");
    assert_eq!(summary.label, "视频生成失败");
    assert_eq!(summary.current, Some(2));
    assert_eq!(summary.total, Some(4));
    assert_eq!(summary.status, ProjectProgressStatus::Failed);
}

#[test]
fn video_project_progress_reports_completed_when_all_prompted_segments_have_video() {
    let facts = VideoProgressFacts {
        prompt_counts: ProgressCounts {
            completed: 3,
            ..Default::default()
        },
        video_counts: ProgressCounts {
            completed: 3,
            ..Default::default()
        },
        video_prompt_total: 3,
        frame_prompt_total: 0,
        segment_plan_total: 0,
    };
    let summary = video_project_progress_from_counts(facts);

    assert_eq!(summary.stage, "video_ready");
    assert_eq!(summary.label, "视频已生成");
    assert_eq!(summary.current, Some(3));
    assert_eq!(summary.total, Some(3));
    assert_eq!(summary.status, ProjectProgressStatus::Completed);
}

#[test]
fn video_project_progress_reports_prompt_ready_without_generated_videos() {
    let facts = VideoProgressFacts {
        prompt_counts: ProgressCounts {
            completed: 2,
            ..Default::default()
        },
        video_counts: ProgressCounts::default(),
        video_prompt_total: 2,
        frame_prompt_total: 0,
        segment_plan_total: 0,
    };
    let summary = video_project_progress_from_counts(facts);

    assert_eq!(summary.stage, "video_ready_to_generate");
    assert_eq!(summary.label, "视频待生成");
    assert_eq!(summary.current, Some(0));
    assert_eq!(summary.total, Some(2));
    assert_eq!(summary.status, ProjectProgressStatus::Idle);
}

#[test]
fn galgame_project_progress_reports_empty_before_script_exists() {
    let summary =
        galgame_project_progress_from_counts(false, 0, 0, PipelineProjectState::default());

    assert_eq!(summary.kind, "galgame");
    assert_eq!(summary.stage, "galgame_empty");
    assert_eq!(summary.label, "Galgame 待生成");
    assert_eq!(summary.current, Some(0));
    assert_eq!(summary.total, None);
    assert_eq!(summary.status, ProjectProgressStatus::Idle);
    assert_eq!(summary.detail.as_deref(), Some("尚未生成剧本"));
}

#[test]
fn galgame_project_progress_blocks_empty_script() {
    let summary = galgame_project_progress_from_counts(true, 2, 0, PipelineProjectState::default());

    assert_eq!(summary.stage, "galgame_script_empty");
    assert_eq!(summary.label, "Galgame 剧本为空");
    assert_eq!(summary.current, Some(0));
    assert_eq!(summary.total, Some(2));
    assert_eq!(summary.status, ProjectProgressStatus::Blocked);
    assert_eq!(summary.detail.as_deref(), Some("已创建剧本但没有行内容"));
}

#[test]
fn galgame_project_progress_reports_ready_script_counts() {
    let summary =
        galgame_project_progress_from_counts(true, 3, 12, PipelineProjectState::default());

    assert_eq!(summary.stage, "galgame_script_ready");
    assert_eq!(summary.label, "Galgame 剧本已生成");
    assert_eq!(summary.current, Some(12));
    assert_eq!(summary.total, Some(12));
    assert_eq!(summary.status, ProjectProgressStatus::Completed);
    assert_eq!(summary.detail.as_deref(), Some("3 场，12 行"));
}

#[test]
fn galgame_project_progress_prioritizes_running_pipeline() {
    let mut pipeline = PipelineProjectState::default();
    pipeline.running = true;
    pipeline.message = Some("正在切分场景".to_string());

    let summary = galgame_project_progress_from_counts(false, 0, 0, pipeline);

    assert_eq!(summary.stage, "galgame_pipeline_running");
    assert_eq!(summary.label, "Galgame 生成中");
    assert_eq!(summary.status, ProjectProgressStatus::Running);
    assert_eq!(summary.detail.as_deref(), Some("正在切分场景"));
}

#[test]
fn galgame_project_progress_prioritizes_failed_pipeline_over_ready_script() {
    let mut pipeline = PipelineProjectState::default();
    pipeline.failed = true;
    pipeline.message = Some("LLM failed".to_string());

    let summary = galgame_project_progress_from_counts(true, 3, 12, pipeline);

    assert_eq!(summary.stage, "galgame_pipeline_failed");
    assert_eq!(summary.label, "Galgame 管线失败");
    assert_eq!(summary.current, Some(12));
    assert_eq!(summary.total, Some(12));
    assert_eq!(summary.status, ProjectProgressStatus::Failed);
    assert_eq!(summary.detail.as_deref(), Some("LLM failed"));
}

#[test]
fn galgame_project_progress_reports_cancelled_pipeline() {
    let mut pipeline = PipelineProjectState::default();
    pipeline.cancelled = true;

    let summary = galgame_project_progress_from_counts(false, 0, 0, pipeline);

    assert_eq!(summary.stage, "galgame_pipeline_cancelled");
    assert_eq!(summary.label, "Galgame 管线已取消");
    assert_eq!(summary.status, ProjectProgressStatus::Blocked);
    assert_eq!(summary.detail, None);
}

#[test]
fn video_project_progress_ignores_stale_failed_video_when_prompt_is_current_pending() {
    let prompts = vec![
        video_prompt_row(1, 1, "completed", Some(79), test_time(30)),
        video_prompt_row(2, 2, "completed", Some(80), test_time(31)),
        video_prompt_row(3, 3, "pending", None, test_time(40)),
    ];
    let videos = vec![
        generated_video_row(79, 1, 1, "completed", test_time(30)),
        generated_video_row(80, 2, 2, "completed", test_time(31)),
        generated_video_row(78, 3, 3, "failed", test_time(20)),
    ];
    let facts = video_progress_facts_from_rows(&prompts, &videos);
    let summary = video_project_progress_from_counts(facts);

    assert_eq!(summary.stage, "video_partial");
    assert_eq!(summary.label, "视频部分完成");
    assert_eq!(summary.current, Some(2));
    assert_eq!(summary.total, Some(3));
    assert_eq!(summary.status, ProjectProgressStatus::Idle);
}
