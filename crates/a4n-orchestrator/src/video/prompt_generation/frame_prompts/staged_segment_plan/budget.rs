pub(super) fn segment_plan_max_tokens(recommended_segment_count: i32) -> usize {
    let segment_count = recommended_segment_count.max(1) as usize;
    4_096_usize
        .saturating_add(segment_count.saturating_mul(3_072))
        .clamp(8_192, 24_576)
}

#[cfg(test)]
mod tests {
    use super::segment_plan_max_tokens;

    #[test]
    fn segment_plan_output_budget_scales_and_is_bounded() {
        assert_eq!(segment_plan_max_tokens(1), 8_192);
        assert_eq!(segment_plan_max_tokens(2), 10_240);
        assert_eq!(segment_plan_max_tokens(7), 24_576);
        assert_eq!(segment_plan_max_tokens(100), 24_576);
    }
}
