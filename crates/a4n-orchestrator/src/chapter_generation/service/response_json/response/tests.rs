use a4n_core::error::AppError;

use super::{normalize_generated_chapter_response, GeneratedChapterMetadata};

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
