//! 漫画计划共享字段规则。

use std::collections::BTreeSet;

use super::MangaPlanValidationError;
use crate::media_authoring::AdaptationBeatRef;

pub(super) fn unique_required_beat_refs(
    refs: &[AdaptationBeatRef],
    path: String,
) -> Result<BTreeSet<AdaptationBeatRef>, MangaPlanValidationError> {
    if refs.is_empty() {
        return invalid(path, "至少需要一个本项目改编节拍引用");
    }
    let mut seen = BTreeSet::new();
    for (index, beat_ref) in refs.iter().cloned().enumerate() {
        beat_ref
            .validate()
            .map_err(|message| validation_error(format!("{path}[{index}]"), message))?;
        if !seen.insert(beat_ref) {
            return invalid(format!("{path}[{index}]"), "改编节拍引用重复");
        }
    }
    Ok(seen)
}

pub(super) fn unique_texts(
    values: &[String],
    path: String,
    required: bool,
) -> Result<(), MangaPlanValidationError> {
    if required && values.is_empty() {
        return invalid(path, "至少需要一项");
    }
    let mut seen = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        required_text(value, format!("{path}[{index}]"))?;
        if !seen.insert(value.trim()) {
            return invalid(format!("{path}[{index}]"), "存在重复值");
        }
    }
    Ok(())
}

pub(super) fn optional_text(
    value: Option<&str>,
    path: String,
) -> Result<(), MangaPlanValidationError> {
    if let Some(value) = value {
        required_text(value, path)?;
    }
    Ok(())
}

pub(super) fn required_text(
    value: &str,
    path: impl Into<String>,
) -> Result<(), MangaPlanValidationError> {
    if value.trim().is_empty() {
        return invalid(path, "不能为空");
    }
    Ok(())
}

pub(super) fn index_u32(offset: usize, path: &str) -> Result<u32, MangaPlanValidationError> {
    u32::try_from(offset + 1).map_err(|_| validation_error(path, "条目数量超出 u32 范围"))
}

pub(super) fn validation_error(
    path: impl Into<String>,
    message: impl Into<String>,
) -> MangaPlanValidationError {
    MangaPlanValidationError {
        path: path.into(),
        message: message.into(),
    }
}

pub(super) fn invalid<T>(
    path: impl Into<String>,
    message: impl Into<String>,
) -> Result<T, MangaPlanValidationError> {
    Err(validation_error(path, message))
}
