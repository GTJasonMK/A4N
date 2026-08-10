use crate::error::AppError;
use a4n_domain::project::ProjectStatus;

pub fn increment_blueprint_version(current: &str) -> Result<String, AppError> {
    let (major, minor) = current.split_once('.').ok_or_else(|| {
        AppError::InvalidArgument(format!("蓝图版本号 {current:?} 必须使用 major.minor 格式"))
    })?;
    if major.is_empty() || minor.is_empty() || minor.contains('.') {
        return Err(AppError::InvalidArgument(format!(
            "蓝图版本号 {current:?} 必须使用 major.minor 格式"
        )));
    }
    let major = major.parse::<u32>().map_err(|_| {
        AppError::InvalidArgument(format!("蓝图版本号 {current:?} 的 major 必须是非负整数"))
    })?;
    let minor = minor.parse::<u32>().map_err(|_| {
        AppError::InvalidArgument(format!("蓝图版本号 {current:?} 的 minor 必须是非负整数"))
    })?;
    let next_minor = minor.checked_add(1).ok_or_else(|| {
        AppError::InvalidArgument(format!("蓝图版本号 {current:?} 的 minor 已超出可递增范围"))
    })?;
    Ok(format!("{major}.{next_minor}"))
}

pub fn project_status_after_blueprint_generation(previous: ProjectStatus) -> ProjectStatus {
    if previous.ordinal() < ProjectStatus::BlueprintReady.ordinal() {
        ProjectStatus::BlueprintReady
    } else {
        previous
    }
}
