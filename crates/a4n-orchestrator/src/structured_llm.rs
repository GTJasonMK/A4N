use a4n_core::error::AppError;
use a4n_llm::provider::{ChatRequest, ChatResponse};
use serde::de::DeserializeOwned;
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;

mod schema_repair;

pub(crate) use schema_repair::json_shape_schema_from_example;
pub use schema_repair::{
    StructuredOutputRepairInput, StructuredOutputRepairResult, StructuredRepairCheckpointContext,
    StructuredRepairMode, StructuredValueRepairInput,
};

pub use a4n_llm::structured_stream::StructuredLlmProgress;
pub use a4n_llm::structured_stream::StructuredLlmStreamResult;

pub struct ChatJsonStreamInput<'a> {
    pub request: ChatRequest,
    pub prompt_name: &'a str,
    pub truncated_message: &'a str,
    pub structured_progress: Option<StructuredLlmProgress>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub cancelled_message: &'a str,
}

pub fn parse_strict_json_response<T>(raw: &str, context: &str) -> Result<T, AppError>
where
    T: DeserializeOwned,
{
    let trimmed = raw.trim();
    let contract_hint = strict_json_contract_hint(trimmed);
    serde_json::from_str(trimmed).map_err(|err| {
        let parse_hint = strict_json_parse_error_hint(&err);
        AppError::InvalidArgument(format!(
            "{context} 响应必须是严格 JSON 对象{contract_hint}{parse_hint}: {err}; 输出预览：{}",
            json_response_preview(trimmed)
        ))
    })
}

fn strict_json_parse_error_hint(err: &serde_json::Error) -> &'static str {
    if matches!(err.classify(), serde_json::error::Category::Eof) {
        return "；响应在 JSON 闭合前结束，通常是模型输出被截断、连接提前结束或 max_tokens 不足";
    }
    ""
}

fn strict_json_contract_hint(text: &str) -> &'static str {
    let lower = text.trim_start().to_ascii_lowercase();
    if lower.starts_with("```") {
        return "；检测到 Markdown 代码围栏，严格 JSON 响应必须直接以 JSON 对象开始，不能用 ``` 或 ```json 包裹";
    }
    if lower.starts_with("<antml thinking>")
        || lower.starts_with("<thinking>")
        || lower.starts_with("<think>")
    {
        return "；检测到模型把 thinking/reasoning 标签写入了正文 content，说明当前 LLM/代理没有遵守严格 JSON response_format；请关闭该模型配置的思考输出、降低/禁用 reasoning，或改用支持 JSON mode 的 LLM 配置";
    }
    ""
}

fn json_response_preview(trimmed: &str) -> String {
    if trimmed.is_empty() {
        return "<空响应>".to_string();
    }

    let mut chars = trimmed.chars();
    let mut preview = chars.by_ref().take(240).collect::<String>();
    if chars.next().is_some() {
        preview.push_str("...");
    }
    preview
        .replace('\r', "\\r")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

impl OrchestratorContext {
    pub async fn chat_llm(&self, request: ChatRequest) -> Result<ChatResponse, AppError> {
        self.llm.chat(request).await
    }

    pub async fn chat_json_stream_with_progress(
        &self,
        input: ChatJsonStreamInput<'_>,
        report_progress: impl FnMut(u32, &str) -> Result<(), String>,
    ) -> Result<String, String> {
        let ChatJsonStreamInput {
            request,
            prompt_name,
            truncated_message,
            structured_progress,
            cancel_token,
            cancelled_message,
        } = input;
        a4n_llm::structured_stream::chat_json_stream_with_progress(
            a4n_llm::structured_stream::ChatJsonStreamInput {
                llm: self.llm.as_ref(),
                request,
                prompt_name,
                truncated_message,
                structured_progress,
                cancel_token,
                cancelled_message,
            },
            report_progress,
        )
        .await
    }

    pub async fn chat_json_stream_result_with_progress(
        &self,
        input: ChatJsonStreamInput<'_>,
        report_progress: impl FnMut(u32, &str) -> Result<(), String>,
    ) -> Result<StructuredLlmStreamResult, String> {
        let ChatJsonStreamInput {
            request,
            prompt_name,
            truncated_message,
            structured_progress,
            cancel_token,
            cancelled_message,
        } = input;
        a4n_llm::structured_stream::chat_json_stream_result_with_progress(
            a4n_llm::structured_stream::ChatJsonStreamInput {
                llm: self.llm.as_ref(),
                request,
                prompt_name,
                truncated_message,
                structured_progress,
                cancel_token,
                cancelled_message,
            },
            report_progress,
        )
        .await
    }
}
