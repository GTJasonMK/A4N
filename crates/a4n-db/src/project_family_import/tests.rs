use super::*;
use std::error::Error;

#[test]
fn project_family_import_database_error_keeps_db_source() {
    let error = ProjectFamilyImportError::Database(DbErr::Custom("begin failed".to_string()));

    assert_eq!(error.to_string(), "Custom Error: begin failed");
    assert!(matches!(
        error,
        ProjectFamilyImportError::Database(DbErr::Custom(message))
            if message == "begin failed"
    ));
}

#[test]
fn project_family_import_commit_error_keeps_context_and_db_source() {
    let error = ProjectFamilyImportError::Commit(DbErr::Custom("commit failed".to_string()));

    assert_eq!(
        error.to_string(),
        "提交项目族导入事务失败: Custom Error: commit failed"
    );
    assert!(error.source().is_some());
}

#[test]
fn project_family_import_invalid_package_has_no_db_source() {
    let error = ProjectFamilyImportError::InvalidPackage("映射缺失".to_string());

    assert_eq!(error.to_string(), "映射缺失");
    assert!(error.source().is_none());
}

#[test]
fn project_family_import_context_error_keeps_operation_and_db_source() {
    let error = ProjectFamilyImportError::DatabaseContext {
        context: "导入测试记录失败",
        source: DbErr::Custom("insert failed".to_string()),
    };

    assert_eq!(
        error.to_string(),
        "导入测试记录失败: Custom Error: insert failed"
    );
    assert!(error.source().is_some());
}

#[test]
fn project_family_import_media_error_has_distinct_classification() {
    let error = ProjectFamilyImportError::MediaImport(ProjectFamilyMediaImportError::Write(
        "write failed".to_string(),
    ));

    assert_eq!(error.to_string(), "write failed");
    assert!(matches!(
        error,
        ProjectFamilyImportError::MediaImport(ProjectFamilyMediaImportError::Write(message))
            if message == "write failed"
    ));
}
