#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmFinishReason {
    Stop,
    ToolCalls,
    Truncated,
}

pub fn classify_finish_reason(finish_reason: Option<&str>) -> Result<LlmFinishReason, String> {
    match finish_reason {
        Some("stop") => Ok(LlmFinishReason::Stop),
        Some("tool_calls") => Ok(LlmFinishReason::ToolCalls),
        Some("length" | "max_tokens") => Ok(LlmFinishReason::Truncated),
        None | Some("") => Err("模型流结束但缺少 finish_reason".to_string()),
        Some(reason) => Err(format!("模型未正常完成：finish_reason={reason}")),
    }
}

pub fn validate_finish_reason(finish_reason: Option<&str>) -> Result<(), String> {
    match classify_finish_reason(finish_reason)? {
        LlmFinishReason::Stop | LlmFinishReason::ToolCalls => Ok(()),
        LlmFinishReason::Truncated => Err("模型输出被 max_tokens 截断，任务未完成".to_string()),
    }
}
