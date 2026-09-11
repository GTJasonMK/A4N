use super::super::super::contracts::{
    COMPRESSION_COMPRESSED_SCENE, COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW,
    COMPRESSION_MINIMAL, LlmContextPackRequirement,
};
use a4n_error::AppError;

pub(super) fn remaining_required_min_token_suffix(
    requirements: &[LlmContextPackRequirement],
) -> Result<Vec<usize>, AppError> {
    let mut suffix = vec![0_usize; requirements.len() + 1];
    for index in (0..requirements.len()).rev() {
        let current_min = if requirements[index].required {
            requirements[index]
                .variants
                .iter()
                .map(|variant| variant.token_estimate)
                .min()
                .unwrap_or(0)
        } else {
            0
        };
        suffix[index] = checked_context_token_add(
            suffix[index + 1],
            current_min,
            "后续必需上下文最小 token 之和",
        )?;
    }
    Ok(suffix)
}

pub(super) fn checked_context_token_add(
    left: usize,
    right: usize,
    operation: &str,
) -> Result<usize, AppError> {
    left.checked_add(right)
        .ok_or_else(|| context_token_overflow(operation))
}

pub(super) fn context_token_overflow(operation: &str) -> AppError {
    AppError::InvalidArgument(format!("LLM 上下文 token 预算计算溢出 usize：{operation}"))
}

pub(super) fn compression_rank(level: &str) -> usize {
    match level {
        COMPRESSION_FULL_RAW => 4,
        COMPRESSION_FOCUSED_RAW => 3,
        COMPRESSION_COMPRESSED_SCENE => 2,
        COMPRESSION_MINIMAL => 1,
        _ => 0,
    }
}
