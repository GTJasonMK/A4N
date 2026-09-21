use std::path::PathBuf;

use anyhow::{Context, anyhow};

pub(crate) fn optional_non_empty_string(key: &str) -> anyhow::Result<Option<String>> {
    match std::env::var(key) {
        Ok(value) => normalize_non_empty_string(key, value).map(Some),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(anyhow!("读取 {key} 失败: {error}")),
    }
}

pub(crate) fn string_or_default(key: &str, default_value: &str) -> anyhow::Result<String> {
    match optional_non_empty_string(key)? {
        Some(value) => Ok(value),
        None => normalize_non_empty_string(key, default_value.to_string()),
    }
}

pub(crate) fn optional_non_empty_path(key: &str) -> anyhow::Result<Option<PathBuf>> {
    let Some(value) = std::env::var_os(key) else {
        return Ok(None);
    };
    if value.to_string_lossy().trim().is_empty() {
        return Err(anyhow!("{key} 不能为空"));
    }
    Ok(Some(PathBuf::from(value)))
}

pub(crate) fn u64_or_default(key: &str, default: u64) -> anyhow::Result<u64> {
    optional_non_empty_string(key)?
        .map(|value| {
            value
                .parse::<u64>()
                .with_context(|| format!("{key} 必须是非负整数"))
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
}

pub(crate) fn optional_u32(key: &str) -> anyhow::Result<Option<u32>> {
    optional_non_empty_string(key)?
        .map(|value| {
            value
                .parse::<u32>()
                .with_context(|| format!("{key} 必须是非负整数"))
        })
        .transpose()
}

fn normalize_non_empty_string(key: &str, value: String) -> anyhow::Result<String> {
    let value = value.trim();
    if value.is_empty() {
        Err(anyhow!("{key} 不能为空"))
    } else {
        Ok(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_non_empty_string;

    #[test]
    fn configured_strings_are_trimmed_and_blank_is_rejected() {
        assert_eq!(
            normalize_non_empty_string("A4N_VALUE", " value ".into()).unwrap(),
            "value"
        );
        assert!(normalize_non_empty_string("A4N_VALUE", "  ".into()).is_err());
    }
}
