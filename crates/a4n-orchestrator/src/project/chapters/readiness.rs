pub(crate) fn validate_chapter_generation_number(chapter_number: i32) -> Result<(), String> {
    if chapter_number <= 0 {
        return Err("章节序号必须大于 0".to_string());
    }
    Ok(())
}
