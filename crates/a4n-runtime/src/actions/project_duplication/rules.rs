use a4n_core::models::project::novel_project;
use a4n_core::state::validator::StateValidator;
use a4n_orchestrator::project::PROJECT_KIND_TEXT;

pub(super) fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label}不能为空"))
    } else {
        Ok(value)
    }
}

pub(super) fn required_project_status(value: &str, label: &str) -> Result<String, String> {
    let value = required_text(value, label)?;
    let status = StateValidator::parse_status(value).map_err(|e| e.to_string())?;
    Ok(status.as_str().to_string())
}

pub(super) fn ensure_text_project_for_copy(
    project: &novel_project::Model,
    operation: &str,
) -> Result<(), String> {
    if project.project_kind != PROJECT_KIND_TEXT {
        return Err(format!("{operation}仅支持文本项目"));
    }
    Ok(())
}
