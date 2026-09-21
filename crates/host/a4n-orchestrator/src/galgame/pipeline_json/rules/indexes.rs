use std::collections::HashSet;

use a4n_error::AppError;

use super::invalid;

pub(in crate::galgame::pipeline_json) fn require_nonempty<T>(
    values: &[T],
    label: &str,
) -> Result<(), AppError> {
    if values.is_empty() {
        Err(invalid(format!("{label} 不能为空")))
    } else {
        Ok(())
    }
}

pub(in crate::galgame::pipeline_json) fn require_positive(
    value: i32,
    label: &str,
) -> Result<(), AppError> {
    if value > 0 {
        Ok(())
    } else {
        Err(invalid(format!("{label} 必须是正整数")))
    }
}

pub(in crate::galgame::pipeline_json) fn require_positive_i64(
    value: i64,
    label: &str,
) -> Result<(), AppError> {
    if value > 0 {
        Ok(())
    } else {
        Err(invalid(format!("{label} 必须是正整数")))
    }
}

pub(in crate::galgame::pipeline_json) fn ensure_contiguous_indexes<I>(
    indexes: I,
    label: &str,
) -> Result<(), AppError>
where
    I: IntoIterator<Item = i32>,
{
    let mut indexes = indexes.into_iter().collect::<Vec<_>>();
    indexes.sort_unstable();
    for (offset, index) in indexes.iter().enumerate() {
        let expected = i32::try_from(offset + 1)
            .map_err(|_| invalid(format!("{label} 索引超出 i32 可表示范围")))?;
        if *index != expected {
            return Err(invalid(format!(
                "{label} index 必须从 1 开始连续递增，期望 {expected}，实际 {index}"
            )));
        }
    }
    Ok(())
}

pub(in crate::galgame::pipeline_json) fn ensure_unique_scene_indexes<I>(
    indexes: I,
    context: &str,
) -> Result<(), AppError>
where
    I: IntoIterator<Item = i32>,
{
    ensure_unique_indexes(indexes, &format!("{context}.scene_lines.scene_index"))
}

pub(in crate::galgame::pipeline_json) fn ensure_unique_indexes<I>(
    indexes: I,
    label: &str,
) -> Result<(), AppError>
where
    I: IntoIterator<Item = i32>,
{
    let mut seen = HashSet::new();
    for index in indexes {
        if !seen.insert(index) {
            return Err(invalid(format!("{label} 重复: {index}")));
        }
    }
    Ok(())
}
