pub(super) fn validate_part_chapter_ranges(ranges: &mut [(i32, i32, i32)]) -> Result<(), String> {
    ranges.sort_by_key(|(start_chapter, end_chapter, part_number)| {
        (*start_chapter, *end_chapter, *part_number)
    });
    for pair in ranges.windows(2) {
        let (prev_start, prev_end, prev_part) = pair[0];
        let (start, end, part) = pair[1];
        if start <= prev_end {
            return Err(format!(
                "reverse_part_outline.parts 章节范围重叠: part {prev_part}({prev_start}-{prev_end}) 与 part {part}({start}-{end})"
            ));
        }
        // 已知下一段 start > prev_end 且 start 仍在 i32 内，因此 prev_end 不可能为 i32::MAX。
        let expected_start = prev_end + 1;
        if start > expected_start {
            return Err(format!(
                "reverse_part_outline.parts 章节范围有缺口: 第 {expected_start} 章到第 {} 章未覆盖",
                start - 1
            ));
        }
    }
    Ok(())
}
