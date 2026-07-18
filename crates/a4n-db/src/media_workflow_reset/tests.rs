use sea_orm::DbErr;

use super::rules::{
    validate_media_workflow_reset_project_id, validate_media_workflow_reset_project_kind,
};

#[test]
fn media_workflow_reset_requires_positive_project_id() {
    assert!(matches!(
        validate_media_workflow_reset_project_id(0),
        Err(DbErr::Custom(message)) if message == "project_id 必须是正整数"
    ));
    assert!(validate_media_workflow_reset_project_id(1).is_ok());
}

#[test]
fn media_workflow_reset_accepts_only_supported_project_kinds() {
    for project_kind in ["comic", "video", "galgame"] {
        assert!(validate_media_workflow_reset_project_kind(project_kind).is_ok());
    }
    assert!(matches!(
        validate_media_workflow_reset_project_kind("text"),
        Err(DbErr::Custom(message))
            if message == "v2 媒体工作流重置不支持项目类型: text"
    ));
}
