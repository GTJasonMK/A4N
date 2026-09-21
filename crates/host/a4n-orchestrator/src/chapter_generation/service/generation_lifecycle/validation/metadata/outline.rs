pub(super) fn extract_marked_outline_value(outline: &str, labels: &[&str]) -> Option<String> {
    outline.lines().find_map(|line| {
        let line = line.trim().trim_start_matches(['#', '-', '*', ' ']).trim();
        labels.iter().find_map(|label| {
            ['：', ':'].iter().find_map(|separator| {
                line.strip_prefix(&format!("{label}{separator}"))
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
            })
        })
    })
}
