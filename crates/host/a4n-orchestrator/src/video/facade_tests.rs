use super::source::validate_import_chapter_video_prompt_fields;

#[test]
fn import_prompt_validation_accepts_ready_video_prompt() {
    assert!(
        validate_import_chapter_video_prompt_fields(
            7,
            10,
            20,
            "video_prompt",
            "completed",
            10,
            20,
        )
        .is_ok()
    );
}

#[test]
fn import_prompt_validation_rejects_wrong_project() {
    let err =
        validate_import_chapter_video_prompt_fields(7, 11, 20, "video_prompt", "completed", 10, 20)
            .unwrap_err();
    assert_eq!(err, "视频提示词 #7 不属于当前项目 10");
}

#[test]
fn import_prompt_validation_rejects_wrong_chapter() {
    let err =
        validate_import_chapter_video_prompt_fields(7, 10, 21, "video_prompt", "completed", 10, 20)
            .unwrap_err();
    assert_eq!(err, "视频提示词 #7 不属于当前章节 20");
}

#[test]
fn import_prompt_validation_rejects_non_video_prompt_type() {
    let err =
        validate_import_chapter_video_prompt_fields(7, 10, 20, "analysis", "completed", 10, 20)
            .unwrap_err();
    assert_eq!(err, "请选择 video_prompt 类型的提示词导入视频");
}

#[test]
fn import_prompt_validation_rejects_running_prompt() {
    let err =
        validate_import_chapter_video_prompt_fields(7, 10, 20, "video_prompt", "running", 10, 20)
            .unwrap_err();
    assert_eq!(err, "视频提示词 #7 正在运行，不能导入视频");
}
