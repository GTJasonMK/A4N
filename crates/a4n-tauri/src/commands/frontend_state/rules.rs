const MAX_KEY_LEN: usize = 256;
const MAX_VALUE_BYTES: usize = 10 * 1024 * 1024;

pub(super) fn validate_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("前端状态 key 不能为空".into());
    }
    if key.len() > MAX_KEY_LEN {
        return Err("前端状态 key 过长".into());
    }
    if !(key.starts_with("a4n:") || key.starts_with("a4n-")) {
        return Err("前端状态 key 必须使用 a4n 前缀".into());
    }
    if key.chars().any(|ch| ch.is_control()) {
        return Err("前端状态 key 包含非法控制字符".into());
    }
    Ok(())
}

pub(super) fn validate_value(value: &str) -> Result<(), String> {
    if value.len() > MAX_VALUE_BYTES {
        return Err("前端状态值过大".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_key;

    #[test]
    fn frontend_state_rejects_non_a4n_keys() {
        assert!(validate_key("a4n:ok").is_ok());
        assert!(validate_key("a4n-theme-mode").is_ok());
        assert!(validate_key("other:key").is_err());
        assert!(validate_key("").is_err());
    }
}
