use super::{validate_import_analysis_stats, validate_parsed_chapter_input};
use crate::import_analysis::{ParsedChapterInput, StartImportAnalysisInput};

fn parsed_chapter(number: i32, title: &str, content: &str) -> ParsedChapterInput {
    ParsedChapterInput {
        number,
        title: title.to_string(),
        content: content.to_string(),
        content_hash: Some("abc123".to_string()),
    }
}

fn start_input(dedup_paragraphs: i32, dedup_rate: Option<f32>) -> StartImportAnalysisInput {
    StartImportAnalysisInput {
        filename: "import.txt".to_string(),
        file_type: "txt".to_string(),
        detected_encoding: Some("utf-8".to_string()),
        chapters: vec![parsed_chapter(1, "旧城回声", "正文")],
        dedup_paragraphs,
        dedup_rate,
    }
}

#[test]
fn validate_import_analysis_stats_rejects_negative_dedup_paragraphs() {
    let input = start_input(-1, Some(0.0));

    let err = validate_import_analysis_stats(&input)
        .expect_err("negative dedup paragraph count must fail");

    assert!(err.contains("去重段落数不能为负数"));
    assert!(err.contains("-1"));
}

#[test]
fn validate_import_analysis_stats_rejects_out_of_range_dedup_rate() {
    let input = start_input(1, Some(1.5));

    let err = validate_import_analysis_stats(&input).expect_err("dedup rate above 1 must fail");

    assert!(err.contains("去重率必须在 0 到 1 之间"));
    assert!(err.contains("1.5"));
}

#[test]
fn validate_import_analysis_stats_accepts_valid_rate_bounds() {
    validate_import_analysis_stats(&start_input(0, Some(0.0)))
        .expect("zero dedup rate should be valid");
    validate_import_analysis_stats(&start_input(1, Some(1.0)))
        .expect("full dedup rate should be valid");
    validate_import_analysis_stats(&start_input(1, None))
        .expect("missing dedup rate should be valid");
}

#[test]
fn validate_parsed_chapter_input_rejects_non_positive_number() {
    let chapter = parsed_chapter(0, "旧城回声", "正文");

    let err =
        validate_parsed_chapter_input(&chapter).expect_err("non-positive chapter number must fail");

    assert!(err.contains("必须大于 0"));
}

#[test]
fn validate_parsed_chapter_input_rejects_blank_title() {
    let chapter = parsed_chapter(3, " \n\t ", "正文");

    let err = validate_parsed_chapter_input(&chapter).expect_err("blank title must fail");

    assert!(err.contains("缺少章节标题"));
}

#[test]
fn validate_parsed_chapter_input_rejects_blank_content() {
    let chapter = parsed_chapter(3, "旧城回声", " \n\t ");

    let err = validate_parsed_chapter_input(&chapter).expect_err("blank content must fail");

    assert!(err.contains("正文为空"));
}

#[test]
fn validate_parsed_chapter_input_accepts_real_chapter() {
    let chapter = parsed_chapter(3, "  旧城回声  ", "  林岚发现星潮线索。  ");

    let validated = validate_parsed_chapter_input(&chapter).expect("valid chapter should pass");

    assert_eq!(validated.number, 3);
    assert_eq!(validated.title, "旧城回声");
    assert_eq!(validated.content, "  林岚发现星潮线索。  ");
    assert_eq!(validated.content_hash, Some("abc123"));
}
