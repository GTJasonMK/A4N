//! 立绘参考图 ID 纯规则。

/// 规范化立绘参考图 ID：正整数、去重保序。
pub fn validate_reference_image_ids(image_ids: &[i64]) -> Result<Vec<i64>, String> {
    let mut seen = std::collections::HashSet::new();
    let mut normalized = Vec::with_capacity(image_ids.len());
    for image_id in image_ids.iter().copied() {
        if image_id <= 0 {
            return Err(format!("参考图 ID 必须为正整数：{image_id}"));
        }
        if !seen.insert(image_id) {
            return Err(format!("参考图包含重复 ID：{image_id}"));
        }
        normalized.push(image_id);
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_unique_positive_ids() {
        assert_eq!(validate_reference_image_ids(&[3, 5]).unwrap(), vec![3, 5]);
        assert_eq!(
            validate_reference_image_ids(&[]).unwrap(),
            Vec::<i64>::new()
        );
    }

    #[test]
    fn rejects_non_positive_or_duplicate() {
        let err = validate_reference_image_ids(&[0]).unwrap_err();
        assert!(err.contains("必须为正整数"));
        let err = validate_reference_image_ids(&[2, 2]).unwrap_err();
        assert!(err.contains("重复 ID：2"));
    }
}
