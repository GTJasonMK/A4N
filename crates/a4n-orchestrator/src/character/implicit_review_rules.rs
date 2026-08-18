//! 角色隐性属性复审参数纯规则。

/// 默认复审 threshold。
pub(crate) const AUTO_IMPLICIT_REVIEW_THRESHOLD: usize = 5;
/// 默认窗口章节数。
pub(crate) const AUTO_IMPLICIT_REVIEW_WINDOW_CHAPTERS: usize = 10;
/// 默认证据条数上限。
pub(crate) const AUTO_IMPLICIT_REVIEW_EVIDENCE_LIMIT: usize = 10;
/// 窗口章节数上限（仅校验用）。
const MAX_IMPLICIT_REVIEW_WINDOW_CHAPTERS: usize = 50;
/// 证据条数上限（仅校验用）。
const MAX_IMPLICIT_REVIEW_EVIDENCE_LIMIT: usize = 50;

/// 校验隐性复审 threshold / 窗口 / 证据上限。
pub(crate) fn validate_implicit_review_params(
    threshold: usize,
    window_chapters: usize,
    evidence_limit: usize,
) -> Result<(usize, usize, usize), String> {
    if threshold == 0 {
        return Err("隐性属性复审 threshold 必须大于 0".to_string());
    }
    if !(1..=MAX_IMPLICIT_REVIEW_WINDOW_CHAPTERS).contains(&window_chapters) {
        return Err(format!(
            "隐性属性复审 window_chapters 必须在 1..={MAX_IMPLICIT_REVIEW_WINDOW_CHAPTERS} 之间"
        ));
    }
    if !(1..=MAX_IMPLICIT_REVIEW_EVIDENCE_LIMIT).contains(&evidence_limit) {
        return Err(format!(
            "隐性属性复审 evidence_limit 必须在 1..={MAX_IMPLICIT_REVIEW_EVIDENCE_LIMIT} 之间"
        ));
    }

    Ok((threshold, window_chapters, evidence_limit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_params_reject_zero_or_out_of_range_values() {
        let threshold_err = validate_implicit_review_params(0, 10, 10).expect_err("zero threshold");
        assert!(threshold_err.contains("threshold"));

        let window_err = validate_implicit_review_params(5, 51, 10).expect_err("oversized window");
        assert!(window_err.contains("window_chapters"));

        let evidence_err = validate_implicit_review_params(5, 10, 0).expect_err("zero evidence");
        assert!(evidence_err.contains("evidence_limit"));

        assert_eq!(
            validate_implicit_review_params(5, 10, 10).unwrap(),
            (5, 10, 10)
        );
    }
}
