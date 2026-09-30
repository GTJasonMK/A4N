use super::manga_page_pipeline_call_order;

#[test]
fn each_page_stage_has_a_stable_unique_call_order() {
    assert_eq!(manga_page_pipeline_call_order(0, 0).unwrap(), 800);
    assert_eq!(manga_page_pipeline_call_order(0, 2).unwrap(), 802);
    assert_eq!(manga_page_pipeline_call_order(1, 0).unwrap(), 803);
    assert!(manga_page_pipeline_call_order(0, 3).is_err());
    assert!(manga_page_pipeline_call_order(i32::MAX as usize, 0).is_err());
}
