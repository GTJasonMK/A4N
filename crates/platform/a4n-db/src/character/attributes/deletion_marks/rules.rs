pub(super) fn next_deletion_mark_count(
    current_count: i32,
    last_marked_chapter: i32,
    chapter_number: i32,
) -> i32 {
    if chapter_number == last_marked_chapter + 1 {
        current_count.max(1) + 1
    } else {
        1
    }
}

pub(super) fn retained_deletion_mark_count(
    first_chapter: i32,
    rewind_to_chapter: i32,
) -> Option<i32> {
    if rewind_to_chapter <= 0 {
        return None;
    }
    if first_chapter <= 0 {
        return None;
    }
    if first_chapter > rewind_to_chapter {
        return None;
    }
    Some(rewind_to_chapter - first_chapter + 1)
}
