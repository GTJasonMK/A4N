use super::facts::video_progress_facts_from_rows;
use super::status::video_project_progress_from_counts;
use super::{CHAPTER_VIDEO_TYPE, VIDEO_PROMPT_TYPE};
use crate::project::progress::ProjectProgressStatus;
use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt};
use chrono::{Duration, NaiveDate, NaiveDateTime};
use serde_json::json;

fn naive_datetime() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

fn test_time(offset_seconds: i64) -> NaiveDateTime {
    naive_datetime() + Duration::seconds(offset_seconds)
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
        prompt_type: VIDEO_PROMPT_TYPE.to_string(),
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
        video_type: CHAPTER_VIDEO_TYPE.to_string(),
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
