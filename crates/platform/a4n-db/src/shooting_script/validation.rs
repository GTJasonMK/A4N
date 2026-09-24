use a4n_domain::chapter::ResolvedChapterSource;

use super::contracts::ShootingScriptDbError;

pub(super) fn validate_source(source: &ResolvedChapterSource) -> Result<(), ShootingScriptDbError> {
    require_positive(source.root_project_id, "root_project_id")?;
    require_positive(source.source_chapter_id, "source_chapter_id")?;
    require_text(&source.source_revision_key, "source_revision_key")?;
    validate_sha256(&source.content_sha256, "source_content_sha256")?;
    if source.content.trim().is_empty() {
        return Err(ShootingScriptDbError::InvalidArgument(
            "source content 不能为空".to_string(),
        ));
    }
    match (source.source_snapshot_id, source.source_snapshot_chapter_id) {
        (None, None) => {}
        (Some(snapshot_id), Some(snapshot_chapter_id)) => {
            require_positive(snapshot_id, "source_snapshot_id")?;
            require_positive(snapshot_chapter_id, "source_snapshot_chapter_id")?;
        }
        _ => {
            return Err(ShootingScriptDbError::InvalidArgument(
                "source_snapshot_id 与 source_snapshot_chapter_id 必须同时为空或同时存在"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_workflow_run_id(
    workflow_run_id: Option<i64>,
) -> Result<(), ShootingScriptDbError> {
    if let Some(workflow_run_id) = workflow_run_id {
        require_positive(workflow_run_id, "workflow_run_id")?;
    }
    Ok(())
}

pub(super) fn validate_sha256(value: &str, field: &str) -> Result<(), ShootingScriptDbError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ShootingScriptDbError::InvalidArgument(format!(
            "{field} 必须是 64 位十六进制 SHA-256"
        )));
    }
    Ok(())
}

pub(super) fn require_positive(value: i64, field: &str) -> Result<(), ShootingScriptDbError> {
    if value <= 0 {
        return Err(ShootingScriptDbError::InvalidArgument(format!(
            "{field} 必须大于 0"
        )));
    }
    Ok(())
}

pub(super) fn require_text<'a>(
    value: &'a str,
    field: &str,
) -> Result<&'a str, ShootingScriptDbError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ShootingScriptDbError::InvalidArgument(format!(
            "{field} 不能为空"
        )));
    }
    Ok(value)
}
