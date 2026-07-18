use super::rules::{ensure_chapter_content_not_empty, ensure_import_chapter_number};
use a4n_core::error::AppError;

#[test]
fn ensure_import_chapter_number_rejects_non_positive_number() {
    let err = ensure_import_chapter_number(0).expect_err("chapter number must be positive");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message) if message == "章节序号必须大于 0"
    ));
    assert!(ensure_import_chapter_number(1).is_ok());
}

#[test]
fn ensure_chapter_content_not_empty_rejects_blank_content() {
    let err =
        ensure_chapter_content_not_empty(" \n\t ").expect_err("blank chapter content must fail");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message) if message == "章节正文不能为空"
    ));
    assert!(ensure_chapter_content_not_empty("有效正文").is_ok());
}
