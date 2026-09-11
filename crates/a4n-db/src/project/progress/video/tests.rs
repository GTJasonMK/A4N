use super::facts::video_progress_facts_from_rows;
use super::identity::{GeneratedVideoProgressRow, VideoPromptProgressRow};
use super::status::video_project_progress_from_counts;
use a4n_db_models::models::chapter::chapter_video_prompt;
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
    _generated_video_id: Option<i64>,
    updated_at: NaiveDateTime,
) -> VideoPromptProgressRow {
    VideoPromptProgressRow {
        id,
        project_id: 9,
        chapter_id: 3,
        prompt_type: chapter_video_prompt::PROMPT_TYPE_VIDEO.to_string(),
        workflow_mode: "first_frame_chain".to_string(),
        shot_number,
        status: status.to_string(),
        updated_at,
    }
}

fn generated_video_row(
    id: i64,
    shot_number: i32,
    prompt_id: i64,
    status: &str,
    updated_at: NaiveDateTime,
) -> GeneratedVideoProgressRow {
    GeneratedVideoProgressRow {
        id,
        project_id: 9,
        chapter_id: Some(3),
        workflow_mode: Some("first_frame_chain".to_string()),
        status: status.to_string(),
        metadata_json: Some(json!({
            "prompt_id": prompt_id,
            "shot_number": shot_number
        })),
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
    let facts = video_progress_facts_from_rows(&prompts, &videos).unwrap();
    let summary = video_project_progress_from_counts(facts);

    assert_eq!(summary.label, "视频部分完成");
    assert_eq!(summary.detail.as_deref(), Some("已完成视频片段 2/3"));
}
