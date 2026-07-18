pub mod json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReverseContextKind {
    Blueprint,
    Outline,
    PartOutline,
}

#[derive(Debug, Clone)]
pub struct ReverseContentRequest<'a> {
    pub project_id: i64,
    pub manual_sample: &'a str,
    pub auto_context: bool,
    pub context_limit: Option<usize>,
}

pub fn required_reverse_context_limit(limit: Option<usize>) -> Result<usize, String> {
    match limit {
        Some(limit) if limit > 0 => Ok(limit),
        _ => Err("启用项目上下文时必须提供 context_limit 且大于 0".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::required_reverse_context_limit;

    #[test]
    fn context_limit_requires_explicit_value() {
        let err = required_reverse_context_limit(None).unwrap_err();

        assert!(err.contains("context_limit"));
    }

    #[test]
    fn context_limit_rejects_zero() {
        let err = required_reverse_context_limit(Some(0)).unwrap_err();

        assert!(err.contains("大于 0"));
    }

    #[test]
    fn context_limit_accepts_positive_value() {
        let limit = required_reverse_context_limit(Some(50)).unwrap();

        assert_eq!(limit, 50);
    }
}
