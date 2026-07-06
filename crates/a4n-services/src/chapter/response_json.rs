use a4n_core::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GeneratedChapterPayload {
    pub(super) title: String,
    pub(super) summary: String,
    pub(super) content: String,
}

#[derive(Debug, Clone)]
pub(super) struct GeneratedChapterMetadata {
    pub(super) title: String,
    pub(super) summary: String,
}

pub(super) fn normalize_generated_chapter_response(
    raw: &str,
    metadata: &GeneratedChapterMetadata,
) -> Result<GeneratedChapterPayload, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidArgument(
            "LLM 未返回可用章节正文，未写入正文".to_string(),
        ));
    }
    if looks_like_structured_or_fenced_response(trimmed) {
        return Err(AppError::InvalidArgument(format!(
            "章节正文 LLM 响应必须是纯文本正文，不能返回 JSON、Markdown 代码块或思考标签；输出预览：{}",
            response_preview(trimmed)
        )));
    }

    Ok(GeneratedChapterPayload {
        title: metadata.title.clone(),
        summary: metadata.summary.clone(),
        content: trimmed.to_string(),
    })
}

pub fn normalize_generated_chapter_content_layout(content: &str) -> String {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let candidates: Vec<String> = if normalized.contains('\n') {
        normalized
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect()
    } else {
        let trimmed = normalized.trim();
        if trimmed.is_empty() {
            Vec::new()
        } else {
            vec![trimmed.to_string()]
        }
    };

    candidates
        .iter()
        .flat_map(|paragraph| split_long_paragraph(paragraph))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn looks_like_structured_or_fenced_response(raw: &str) -> bool {
    let trimmed = raw.trim_start();
    trimmed.starts_with("```")
        || trimmed.starts_with('{')
        || trimmed.starts_with('[')
        || trimmed.starts_with("<think>")
}

fn response_preview(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }
    let prefix = trimmed.chars().take(180).collect::<String>();
    if trimmed.chars().count() <= 260 {
        return prefix;
    }
    let suffix = trimmed
        .chars()
        .rev()
        .take(80)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{prefix} … {suffix}")
}

fn split_long_paragraph(paragraph: &str) -> Vec<String> {
    const TARGET_MIN_CHARS: usize = 160;
    const HARD_MAX_CHARS: usize = 360;

    let trimmed = paragraph.trim();
    if trimmed.chars().count() <= HARD_MAX_CHARS {
        return vec![trimmed.to_string()];
    }

    let chars: Vec<char> = trimmed.chars().collect();
    let mut paragraphs = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;
    let mut index = 0usize;

    while index < chars.len() {
        let ch = chars[index];
        current.push(ch);
        current_len += 1;

        if is_sentence_terminal(ch) {
            while index + 1 < chars.len() && is_closing_punctuation(chars[index + 1]) {
                index += 1;
                current.push(chars[index]);
                current_len += 1;
            }

            if current_len >= TARGET_MIN_CHARS {
                push_non_empty_paragraph(&mut paragraphs, &mut current);
                current_len = 0;
            }
        } else if current_len >= HARD_MAX_CHARS {
            push_non_empty_paragraph(&mut paragraphs, &mut current);
            current_len = 0;
        }

        index += 1;
    }

    push_non_empty_paragraph(&mut paragraphs, &mut current);
    if paragraphs.is_empty() {
        vec![trimmed.to_string()]
    } else {
        paragraphs
    }
}

fn push_non_empty_paragraph(paragraphs: &mut Vec<String>, current: &mut String) {
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        paragraphs.push(trimmed.to_string());
    }
    current.clear();
}

fn is_sentence_terminal(ch: char) -> bool {
    matches!(ch, '。' | '！' | '？' | '；' | '…' | '!' | '?' | ';')
}

fn is_closing_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '”' | '’' | '"' | '\'' | '」' | '』' | ')' | '）' | ']' | '】' | '》'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> GeneratedChapterMetadata {
        GeneratedChapterMetadata {
            title: "备用标题".to_string(),
            summary: "备用摘要".to_string(),
        }
    }

    #[test]
    fn rejects_structured_chapter_json_response() {
        let err = normalize_generated_chapter_response(
            r#"{"title":"灰烬低语","summary":"主角发现线索。","full_content":"正文内容"}"#,
            &metadata(),
        )
        .expect_err("chapter writing contract is plain text");

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("纯文本正文"));
    }

    #[test]
    fn rejects_fenced_chapter_json_response() {
        let err = normalize_generated_chapter_response(
            "```json\n{\"title\":\"标题\",\"summary\":\"摘要\",\"full_content\":\"正文\"}\n```",
            &metadata(),
        )
        .expect_err("markdown fenced JSON should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn accepts_plain_text_chapter_response_with_incidental_json() {
        let raw = "她低声说：{\"note\":\"不是模型输出契约\"}，然后继续向前。";
        let payload = normalize_generated_chapter_response(raw, &metadata())
            .expect("plain text is the current chapter-writing contract");

        assert_eq!(payload.title, "备用标题");
        assert_eq!(payload.summary, "备用摘要");
        assert_eq!(payload.content, raw);
    }

    #[test]
    fn rejects_truncated_chapter_json_with_preview() {
        let err = normalize_generated_chapter_response(
            r#"{"title":"标题","summary":"摘要","full_content":"正文写到一半"#,
            &metadata(),
        )
        .expect_err("json-like text must be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("纯文本正文"));
    }

    #[test]
    fn rejects_primary_json_without_content_field() {
        let err = normalize_generated_chapter_response(r#"{"title":"只有标题"}"#, &metadata())
            .expect_err("missing content should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("纯文本正文"));
    }

    #[test]
    fn rejects_generated_chapter_json_without_title() {
        let err = normalize_generated_chapter_response(
            r#"{"summary":"摘要","full_content":"正文内容"}"#,
            &metadata(),
        )
        .expect_err("missing title should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("纯文本正文"));
    }

    #[test]
    fn rejects_generated_chapter_json_without_summary() {
        let err = normalize_generated_chapter_response(
            r#"{"title":"标题","full_content":"正文内容"}"#,
            &metadata(),
        )
        .expect_err("missing summary should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("纯文本正文"));
    }

    #[test]
    fn rejects_think_tags_before_chapter_response_json() {
        let err = normalize_generated_chapter_response(
            "<think>先分析剧情走向</think>\n{\"title\":\"标题\",\"summary\":\"摘要\",\"full_content\":\"正文内容\"}",
            &metadata(),
        )
        .expect_err("think tags should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn preserves_existing_short_paragraphs() {
        let content = "第一段。\n\n第二段。\n第三段。";

        assert_eq!(
            normalize_generated_chapter_content_layout(content),
            "第一段。\n\n第二段。\n\n第三段。"
        );
    }

    #[test]
    fn splits_single_long_chinese_paragraph_at_sentence_boundaries() {
        let sentence = "陈恪站在廊下，看见雨线从檐角落下，像一排细密的银针。";
        let content = sentence.repeat(20);
        let normalized = normalize_generated_chapter_content_layout(&content);
        let paragraphs: Vec<&str> = normalized.split("\n\n").collect();

        assert!(paragraphs.len() > 1);
        assert!(paragraphs.iter().all(|paragraph| paragraph.ends_with('。')));
        assert_eq!(normalized.replace("\n\n", ""), content);
    }

    #[test]
    fn keeps_closing_quotes_with_sentence() {
        let sentence = "“你真的想清楚了吗？”";
        let content = sentence.repeat(30);
        let normalized = normalize_generated_chapter_content_layout(&content);

        assert!(!normalized.contains("\n\n”"));
        assert_eq!(normalized.replace("\n\n", ""), content);
    }
}
