use super::*;

#[test]
fn video_llm_step_error_includes_prompt_name_for_timeout() {
    let err = video_llm_step_error(
        "chapter_video_prompt",
        AppError::Timeout("LLM API响应读取超过 120 秒".to_string()),
    );

    let AppError::Timeout(message) = err else {
        panic!("timeout should stay timeout");
    };
    assert!(message.contains("chapter_video_prompt"));
    assert!(message.contains("120 秒"));
}

#[test]
fn video_llm_step_error_preserves_cancelled() {
    assert!(matches!(
        video_llm_step_error("chapter_video_prompt", AppError::Cancelled),
        AppError::Cancelled
    ));
}
