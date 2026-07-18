use a4n_core::models::chapter::chapter_video_prompt;
use chrono::Utc;

use super::super::VideoDbError;
use super::mutations::{
    ensure_chapter_video_prompt_not_running, validate_chapter_video_prompt_duration,
};

fn prompt(status: &str) -> chapter_video_prompt::Model {
    let now = Utc::now().naive_utc();
    chapter_video_prompt::Model {
        id: 1,
        project_id: 1,
        chapter_id: 1,
        style: None,
        prompt_type: "storyboard_prompt".to_string(),
        workflow_mode: "storyboard".to_string(),
        shot_number: 1,
        title: None,
        scene_summary: None,
        visual_prompt: "visual".to_string(),
        motion_prompt: None,
        negative_prompt: None,
        duration_secs: 5,
        aspect_ratio: None,
        reference_image_ids_json: None,
        status: status.to_string(),
        generated_video_id: None,
        metadata_json: None,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn duration_accepts_supported_range_and_rejects_outside_values() {
    assert!(validate_chapter_video_prompt_duration(1).is_ok());
    assert!(validate_chapter_video_prompt_duration(60).is_ok());
    assert!(validate_chapter_video_prompt_duration(0).is_err());
    assert!(validate_chapter_video_prompt_duration(61).is_err());
}

#[test]
fn running_prompt_rejects_destructive_operation() {
    assert!(ensure_chapter_video_prompt_not_running(&prompt("pending"), "delete").is_ok());
    assert!(matches!(
        ensure_chapter_video_prompt_not_running(&prompt("running"), "delete"),
        Err(VideoDbError::InvalidState { current, operation })
            if current == "running" && operation == "delete"
    ));
}
