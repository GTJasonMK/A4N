use super::super::rules::compression_rank;
use crate::context::LlmContextPackRef;

pub(super) fn select_variant(
    variants: &[LlmContextPackRef],
    used: usize,
    later_required_min_tokens: usize,
    available_input_tokens: usize,
) -> (Option<LlmContextPackRef>, Vec<LlmContextPackRef>) {
    let mut ranked = variants.to_vec();
    ranked.sort_by(|left, right| {
        compression_rank(right.compression_level.as_str())
            .cmp(&compression_rank(left.compression_level.as_str()))
            .then_with(|| left.token_estimate.cmp(&right.token_estimate))
    });
    let selected = ranked.iter().find(|variant| {
        let Some(used_with_variant) = used.checked_add(variant.token_estimate) else {
            return false;
        };
        used_with_variant
            .checked_add(later_required_min_tokens)
            .is_some_and(|required_total| required_total <= available_input_tokens)
    });
    (selected.cloned(), ranked)
}
