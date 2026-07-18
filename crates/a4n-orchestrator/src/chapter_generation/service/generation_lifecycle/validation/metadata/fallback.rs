pub(super) fn summary_from_generated_content(raw_content: &str) -> String {
    let mut summary = raw_content
        .trim()
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("章节正文已生成")
        .chars()
        .take(120)
        .collect::<String>();
    if summary.chars().count() == 120 {
        summary.push('…');
    }
    summary
}
