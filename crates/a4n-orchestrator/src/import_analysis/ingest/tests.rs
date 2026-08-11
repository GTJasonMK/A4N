use super::validate_parsed_chapter_input;
use crate::import_analysis::ParsedChapterInput;

fn parsed_chapter(number: i32, title: &str, content: &str) -> ParsedChapterInput {
    ParsedChapterInput {
        number,
        title: title.to_string(),
        content: content.to_string(),
    }
}

#[test]
fn validate_parsed_chapter_input_rejects_non_positive_number() {
    let chapter = parsed_chapter(0, "旧城回声", "正文");

    let err =
        validate_parsed_chapter_input(&chapter).expect_err("non-positive chapter number must fail");

    assert!(err.to_string().contains("必须大于 0"));
}

#[test]
fn validate_parsed_chapter_input_rejects_blank_title() {
    let chapter = parsed_chapter(3, " \n\t ", "正文");

    let err = validate_parsed_chapter_input(&chapter).expect_err("blank title must fail");

    assert!(err.to_string().contains("缺少章节标题"));
}

#[test]
fn validate_parsed_chapter_input_rejects_blank_content() {
    let chapter = parsed_chapter(3, "旧城回声", " \n\t ");

    let err = validate_parsed_chapter_input(&chapter).expect_err("blank content must fail");

    assert!(err.to_string().contains("正文为空"));
}

#[test]
fn validate_parsed_chapter_input_accepts_real_chapter() {
    let chapter = parsed_chapter(3, "  旧城回声  ", "  林岚发现星潮线索。  ");

    let validated = validate_parsed_chapter_input(&chapter).expect("valid chapter should pass");

    assert_eq!(validated.number, 3);
    assert_eq!(validated.title, "旧城回声");
    assert_eq!(validated.content, "  林岚发现星潮线索。  ");
}
