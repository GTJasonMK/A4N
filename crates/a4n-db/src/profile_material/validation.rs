use sea_orm::DbErr;

use super::contracts::ProfileMaterialDbError;

pub(super) fn require_positive(value: i64, field: &str) -> Result<i64, ProfileMaterialDbError> {
    if value <= 0 {
        return Err(ProfileMaterialDbError::InvalidArgument(format!(
            "{field} 必须大于 0"
        )));
    }
    Ok(value)
}

pub(super) fn require_text<'a>(
    value: &'a str,
    field: &str,
) -> Result<&'a str, ProfileMaterialDbError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ProfileMaterialDbError::InvalidArgument(format!(
            "{field} 不能为空"
        )));
    }
    Ok(value)
}

pub(super) fn database(operation: &'static str, source: DbErr) -> ProfileMaterialDbError {
    ProfileMaterialDbError::Database { operation, source }
}
