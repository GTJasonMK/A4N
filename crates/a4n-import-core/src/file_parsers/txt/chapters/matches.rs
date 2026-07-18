use regex::Regex;

use super::super::helpers::{
    line_end_at_or_after, line_start_at_or_before, normalize_chapter_title_line,
};

pub(super) fn find_chapter_matches<'a>(patterns: &[Regex], text: &'a str) -> Vec<(usize, &'a str)> {
    let mut matches: Vec<(usize, &str)> = Vec::new();
    for pattern in patterns {
        for item in pattern.find_iter(text) {
            let line_start = line_start_at_or_before(text, item.start());
            let line_end = line_end_at_or_after(text, item.end());
            let title = normalize_chapter_title_line(&text[line_start..line_end]);
            matches.push((line_start, title));
        }
    }

    matches.sort_by_key(|(position, _)| *position);
    matches.dedup_by_key(|(position, _)| *position);
    matches
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::find_chapter_matches;

    #[test]
    fn sorts_and_deduplicates_overlapping_pattern_matches() {
        let patterns = [
            Regex::new(r"(?m)^#{1,6}\s*第\d+章").expect("valid pattern"),
            Regex::new(r"(?m)^#\s*第1章").expect("valid overlapping pattern"),
        ];
        let text = "# 第1章 开始\n正文\n## 第2章 继续\n正文";

        let matches = find_chapter_matches(&patterns, text);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0], (0, "第1章 开始"));
        assert!(matches[1].0 > matches[0].0);
        assert_eq!(matches[1].1, "第2章 继续");
    }
}
