use a4n_core::error::AppError;

pub(in crate::blueprint) fn increment_version(current: &str) -> Result<String, AppError> {
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
