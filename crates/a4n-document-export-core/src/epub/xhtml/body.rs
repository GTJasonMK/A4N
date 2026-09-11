use super::escape_xml;

pub(super) fn render_paragraphs(content: &str) -> String {
    content
        .split('\n')
        .filter(|paragraph| !paragraph.trim().is_empty())
        .map(|paragraph| format!("    <p>{}</p>", escape_xml(paragraph)))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_non_empty_lines_as_escaped_paragraphs() {
        let body = render_paragraphs("第一段 & 内容\n   \n  第二段 <继续>  \n");

        assert_eq!(
            body,
            "    <p>第一段 &amp; 内容</p>\n    <p>  第二段 &lt;继续&gt;  </p>"
        );
    }

    #[test]
    fn renders_empty_content_as_empty_body() {
        assert!(render_paragraphs("\n \t\n").is_empty());
    }
}
