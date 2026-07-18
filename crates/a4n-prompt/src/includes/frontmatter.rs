/// 从 Markdown 全文剥离开头的 YAML frontmatter，返回正文。
pub(super) fn strip_frontmatter(content: &str) -> &str {
    if !content.starts_with("---") {
        return content;
    }
    let after_first = &content[3..];
    if let Some(end) = after_first.find("\n---") {
        let body_start = 3 + end + 4;
        let rest = &content[body_start..];
        if let Some(stripped) = rest.strip_prefix('\n') {
            stripped
        } else {
            rest
        }
    } else {
        content
    }
}
