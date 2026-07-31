use super::{
    completion::completed_video_write_is_idempotent,
    error::normalize_generated_video_error_message,
    waiting::validate_generated_video_waiting_status,
};
use crate::video::VideoDbError;

#[test]
fn generated_video_waiting_status_accepts_only_pending_or_running() {
    assert!(validate_generated_video_waiting_status("pending").is_ok());
    assert!(validate_generated_video_waiting_status("running").is_ok());

    let err = validate_generated_video_waiting_status("completed")
        .expect_err("completed is not a waiting status");
    assert!(matches!(
        err,
        VideoDbError::InvalidArgument(message)
            if message == "视频等待状态无效: completed"
    ));
}

#[test]
fn generated_video_error_message_is_trimmed_and_must_not_be_blank() {
    let long = format!("  {}  ", "provider failed: ".to_string() + &"x".repeat(900));
    let normalized = normalize_generated_video_error_message(&long).expect("nonblank message");

    assert!(normalized.starts_with("provider failed: "));
    assert_eq!(normalized.len(), "provider failed: ".len() + 900);

    let err = normalize_generated_video_error_message(" \n\t ")
        .expect_err("failed records need an explicit reason");
    assert!(matches!(
        err,
        VideoDbError::InvalidArgument(message) if message == "视频失败原因不能为空"
    ));
}

#[test]
fn completed_video_write_is_idempotent_only_for_the_adopted_file() {
    assert!(
        completed_video_write_is_idempotent("running", None, "new.mp4").is_ok_and(|same| !same)
    );
    assert!(
        completed_video_write_is_idempotent("completed", Some("adopted.mp4"), "adopted.mp4")
            .is_ok_and(|same| same)
    );
    assert!(matches!(
        completed_video_write_is_idempotent(
            "completed",
            Some("adopted.mp4"),
            "duplicate.mp4"
        ),
        Err(VideoDbError::InvalidState { current, operation })
            if current == "completed" && operation == "用不同文件重复完成视频生成"
    ));
}
