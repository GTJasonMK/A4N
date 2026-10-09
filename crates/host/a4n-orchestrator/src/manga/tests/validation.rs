#[test]
fn page_scope_rejects_chapterwide_pagination_stages() {
    use a4n_api_contract::manga::MangaPipelineStopAfter;

    assert!(
        crate::manga::validate_manga_page_scope(Some(2), Some(MangaPipelineStopAfter::PageSlots))
            .is_err()
    );
    assert!(
        crate::manga::validate_manga_page_scope(
            Some(2),
            Some(MangaPipelineStopAfter::PageAdaptation)
        )
        .is_ok()
    );
    assert!(crate::manga::validate_manga_page_scope(Some(0), None).is_err());
}
