use a4n_chapter_analysis::ensure_chapter_has_analysis_content;

#[test]
fn chapter_analysis_requires_available_content() {
    let mut ch = sample_chapter();
    ch.content = "   ".to_string();

    let err = ensure_chapter_has_analysis_content(&ch).unwrap_err();

    assert!(matches!(err, AppError::InvalidArgument(message) if message.contains("章节正文为空")));
}
