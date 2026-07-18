#[test]
fn required_chapter_title_rejects_missing_title() {
    let mut ch = sample_chapter();
    ch.title = Some("  ".into());

    let err = required_chapter_title(&ch).unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("缺少章节标题")));
}
