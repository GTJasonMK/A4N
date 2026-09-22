use super::manga_drawing_prompt_call_order;

#[test]
fn drawing_prompt_call_order_is_shared_by_preview_and_execution() {
    assert_eq!(manga_drawing_prompt_call_order(0).unwrap(), 800);
    assert!(manga_drawing_prompt_call_order(i32::MAX as usize).is_err());
}
