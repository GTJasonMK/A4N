use std::collections::HashSet;

pub(super) fn validate_default_named_rows<T>(
    kind: &str,
    rows: &[T],
    is_default: impl Fn(&T) -> bool,
    name: impl for<'a> Fn(&'a T) -> &'a str,
    validate: impl Fn(&str, &T) -> Result<(), String>,
) -> Result<(), String> {
    ensure_at_most_one_default(kind, rows.iter().filter(|row| is_default(*row)).count())?;

    let mut seen_names = HashSet::new();
    let identity_kind = format!("{kind} name");
    for (index, row) in rows.iter().enumerate() {
        let label = format!("{kind} #{}", index + 1);
        let name = required_text(name(row), &label, "name")?;
        ensure_unique_identity(&mut seen_names, &identity_kind, name)?;
        validate(&label, row)?;
    }
    Ok(())
}

pub(super) fn ensure_at_most_one_default(kind: &str, count: usize) -> Result<(), String> {
    if count > 1 {
        return Err(format!("{kind}最多只能有一个默认项，当前为 {count} 个"));
    }
    Ok(())
}

pub(super) fn ensure_unique_identity(
    seen: &mut HashSet<String>,
    kind: &str,
    value: impl AsRef<str>,
) -> Result<(), String> {
    let value = value.as_ref();
    if !seen.insert(value.to_string()) {
        return Err(format!("{kind} 重复：{value}"));
    }
    Ok(())
}

pub(super) fn required_text<'a>(
    value: &'a str,
    label: &str,
    field: &str,
) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{label} 缺少 {field}"));
    }
    Ok(value)
}

pub(super) fn positive_i32(value: i32, label: &str, field: &str) -> Result<(), String> {
    if value <= 0 {
        return Err(format!("{label} {field} 必须为正整数"));
    }
    Ok(())
}

pub(super) fn trim_string(value: &mut String) {
    let trimmed = value.trim();
    if trimmed.len() != value.len() {
        *value = trimmed.to_string();
    }
}
