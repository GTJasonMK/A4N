use sea_orm::DbErr;

pub(super) fn validate_media_workflow_reset_project_id(project_id: i64) -> Result<(), DbErr> {
    if project_id <= 0 {
        return Err(DbErr::Custom("project_id 必须是正整数".to_string()));
    }
    Ok(())
}

pub(super) fn validate_media_workflow_reset_project_kind(project_kind: &str) -> Result<(), DbErr> {
    if matches!(project_kind, "comic" | "video" | "galgame") {
        Ok(())
    } else {
        Err(DbErr::Custom(format!(
            "v2 媒体工作流重置不支持项目类型: {project_kind}"
        )))
    }
}
