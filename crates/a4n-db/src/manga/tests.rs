use super::types::{
    manga_record_database, require_manga_text, require_positive_manga_id,
    validate_manga_page_position,
};
use super::*;
use std::error::Error;

mod pipeline_behavior;

#[test]
fn manga_record_input_validation_rejects_invalid_identity_and_position() {
    assert!(matches!(
        require_positive_manga_id(0, "project_id"),
        Err(MangaPromptRecordError::InvalidArgument(message))
            if message == "project_id 必须是正整数"
    ));
    assert!(matches!(
        require_manga_text("  ", "prompt_type"),
        Err(MangaPromptRecordError::InvalidArgument(message))
            if message == "prompt_type 不能为空"
    ));
    assert!(matches!(
        validate_manga_page_position(0, 0),
        Err(MangaPromptRecordError::InvalidArgument(message))
            if message == "page_number 必须是正整数"
    ));
    assert!(matches!(
        validate_manga_page_position(1, -1),
        Err(MangaPromptRecordError::InvalidArgument(message))
            if message == "panel_number 不能为负数"
    ));
}

#[test]
fn manga_record_database_error_keeps_operation_and_source() {
    let error = manga_record_database("读取漫画流水线状态失败")(sea_orm::DbErr::Custom(
        "fixture failure".to_string(),
    ));
    assert!(matches!(
        &error,
        MangaPromptRecordError::Database { operation, source }
            if *operation == "读取漫画流水线状态失败"
                && source.to_string() == "Custom Error: fixture failure"
    ));
    assert!(error.source().is_some());
    assert!(error.to_string().contains("读取漫画流水线状态失败"));
}
