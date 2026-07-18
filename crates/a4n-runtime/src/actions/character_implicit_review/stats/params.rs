use super::super::types::{
    MAX_IMPLICIT_REVIEW_EVIDENCE_LIMIT, MAX_IMPLICIT_REVIEW_WINDOW_CHAPTERS,
};

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
