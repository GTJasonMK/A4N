use a4n_error::AppError;

use super::rules::{checked_context_token_add, context_token_overflow};

pub(super) struct InputCapacity {
    pub(super) raw_available_input_tokens: usize,
    pub(super) available_input_tokens: usize,
}

pub(super) fn calculate_input_capacity(
    model_context_tokens: usize,
    output_token_reserve: usize,
    safety_margin_tokens: usize,
    input_utilization_percent: u8,
    dynamic_delta_tokens: usize,
) -> Result<InputCapacity, AppError> {
    if input_utilization_percent == 0 || input_utilization_percent > 100 {
        return Err(AppError::InvalidArgument(format!(
            "LLM 上下文输入利用率必须在 1..=100，当前为 {input_utilization_percent}"
        )));
    }
    let reserved_tokens = checked_context_token_add(
        output_token_reserve,
        safety_margin_tokens,
        "输出预留与安全余量之和",
    )?;
    if model_context_tokens <= reserved_tokens {
        return Err(AppError::InvalidArgument(format!(
            "模型上下文窗口不足：context={model_context_tokens}, output_reserve={output_token_reserve}, safety_margin={safety_margin_tokens}"
        )));
    }
    let raw_available_input_tokens = model_context_tokens - reserved_tokens;
    let available_input_tokens = raw_available_input_tokens
        .checked_mul(input_utilization_percent as usize)
        .ok_or_else(|| context_token_overflow("可用输入 token 与利用率乘积"))?
        / 100;
    if dynamic_delta_tokens > available_input_tokens {
        return Err(AppError::InvalidArgument(format!(
            "当前任务动态输入约 {dynamic_delta_tokens} tokens，超过模型可用输入窗口 {available_input_tokens}"
        )));
    }
    Ok(InputCapacity {
        raw_available_input_tokens,
        available_input_tokens,
    })
}
