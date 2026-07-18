use super::*;

#[test]
fn chapter_outline_ids_in_part_range_selects_only_target_range() {
    let part = sample_part_for_plan(2, 11, 20);
    let outlines = [8, 12, 20, 21]
        .into_iter()
        .map(|chapter_number| sample_chapter_outline(chapter_number, Some(part.id)))
        .collect::<Vec<_>>();
    let deleted_ids = chapter_outline_ids_in_part_range(&outlines, &part);
    assert_eq!(deleted_ids, vec![112, 120]);
}
#[test]
fn chapter_outline_ids_in_part_ranges_selects_by_chapter_range_not_part_id() {
    let mut first_part = sample_part_for_plan(2, 11, 20);
    first_part.id = 20;
    let mut second_part = sample_part_for_plan(3, 21, 30);
    second_part.id = 30;
    let outlines = [
        sample_chapter_outline(12, None),
        sample_chapter_outline(18, Some(999)),
        sample_chapter_outline(25, Some(second_part.id)),
        sample_chapter_outline(31, Some(second_part.id)),
    ];
    let deleted_ids =
        chapter_outline_ids_in_part_ranges(&outlines, &[first_part.clone(), second_part.clone()]);
    assert_eq!(deleted_ids, vec![112, 118, 125]);
}
