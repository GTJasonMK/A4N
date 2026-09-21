#[test]
fn chapter_display_title_uses_neutral_fallback_when_title_is_missing() {
    let mut ch = sample_chapter();
    ch.title = Some("  ".into());

    assert_eq!(chapter_display_title(&ch), "第3章");
}
