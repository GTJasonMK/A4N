use super::super::contracts::ProjectRollbackImpactReadError;

pub(super) fn rollback_count_i32(
    count: u64,
    label: &str,
) -> Result<i32, ProjectRollbackImpactReadError> {
    i32::try_from(count).map_err(|_| {
        ProjectRollbackImpactReadError::InvalidArgument(format!("{label}超出 i32 范围"))
    })
}
