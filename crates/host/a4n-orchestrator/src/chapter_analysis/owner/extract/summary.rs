pub(in crate::chapter_analysis::owner) fn should_replace_summary(
    existing: Option<&str>,
    chapter_number: i32,
) -> bool {
    let Some(text) = existing.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    text.contains("摘要待生成")
        || text.contains("待分析")
        || text == format!("第{chapter_number}章")
        || (text.starts_with(&format!("第{chapter_number}章：")) && text.ends_with('字'))
}
