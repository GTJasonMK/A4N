pub(super) use a4n_api_contract::config::{CreateLlmInput, LlmConfigSummary, UpdateLlmInput};

pub(super) fn positive_model_output_cap(value: i32) -> Result<usize, String> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "模型最大输出必须大于 0".to_string())
}

pub(super) fn positive_model_context_cap(value: i32) -> Result<usize, String> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "模型上下文窗口必须大于 0".to_string())
}
