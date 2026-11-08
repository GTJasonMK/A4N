use a4n_domain::chapter::ChapterRecord;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, ChatResponse};
use a4n_rag_core::{ChapterRagChunkingMode, RagRecord};
use tokio_util::sync::CancellationToken;

use super::OrchestratorContext;
use crate::{
    rag::chapter_rag_records_with_provider,
    structured_llm::{ChatJsonStreamInput, StructuredLlmStreamResult},
};

impl OrchestratorContext {
    pub(crate) async fn validate_llm_response<T>(
        &self,
        request: &ChatRequest,
        response: ChatResponse,
        context: &str,
        cancel_token: Option<&CancellationToken>,
        validate: impl FnMut(&str) -> Result<T, AppError>,
    ) -> Result<(ChatResponse, T), AppError> {
        crate::structured_llm::correct_llm_response(
            request,
            response,
            context,
            cancel_token,
            validate,
            |corrective_request| async move {
                let workflow_run_id = corrective_request
                    .trace_context
                    .as_ref()
                    .and_then(|trace| trace.workflow_run_id);
                if let Some(id) = workflow_run_id
                    && self.workflow_run_status_record_by_id(id).await?.status
                        == a4n_domain::workflow::WORKFLOW_STATUS_CANCELLED
                {
                    return Err(AppError::Cancelled);
                }
                let response = self
                    .stream_complete_llm_with_sink(corrective_request, None, cancel_token)
                    .await?;
                if let Some(id) = workflow_run_id
                    && self.workflow_run_status_record_by_id(id).await?.status
                        == a4n_domain::workflow::WORKFLOW_STATUS_CANCELLED
                {
                    return Err(AppError::Cancelled);
                }
                Ok(response)
            },
        )
        .await
    }

    /// 生产 LLM 完整结果入口：只走统一 stream driver，禁止 provider `chat()`。
    pub(crate) async fn stream_complete_llm(
        &self,
        request: ChatRequest,
    ) -> Result<ChatResponse, AppError> {
        self.stream_complete_llm_with_sink(request, None, None)
            .await
    }

    pub(crate) async fn stream_complete_llm_with_sink(
        &self,
        request: ChatRequest,
        stream_reporter: Option<&(dyn Fn(&str) + Send + Sync)>,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<ChatResponse, AppError> {
        crate::generation::stream::execute_llm_chat_response_with_sink(
            self.llm.as_ref(),
            request,
            cancel_token.cloned().unwrap_or_default(),
            stream_reporter,
        )
        .await
    }

    pub(crate) fn llm_model_name(&self) -> String {
        self.llm.model_name().to_string()
    }

    /// 领域 LLM 调用需要的 provider 引用（不暴露字段）。
    pub(crate) fn llm_backend(&self) -> &dyn a4n_llm_provider_contract::provider::LlmProvider {
        self.llm.as_ref()
    }

    /// 结构化里程碑（「进行到哪了」）与原始 delta（「还活着」）取自同一个 token 流，
    /// 调用方可同时获得二者而不重复请求模型。
    pub(crate) async fn chat_json_stream_result_with_sink(
        &self,
        input: ChatJsonStreamInput<'_>,
        report_progress: impl FnMut(u32, &str) -> Result<(), String>,
        on_delta: Option<&mut (dyn FnMut(&str) + Send)>,
    ) -> Result<StructuredLlmStreamResult, String> {
        let ChatJsonStreamInput {
            request,
            prompt_name,
            truncated_message,
            structured_progress,
            cancel_token,
            cancelled_message,
        } = input;
        a4n_llm_structured_stream::chat_json_stream_result_with_sink(
            a4n_llm_structured_stream::ChatJsonStreamInput {
                llm: self.llm.as_ref(),
                request,
                prompt_name,
                truncated_message,
                structured_progress,
                cancel_token,
                cancelled_message,
            },
            report_progress,
            on_delta,
        )
        .await
    }

    pub(crate) async fn chapter_rag_records(
        &self,
        chapter: &ChapterRecord,
        mode: ChapterRagChunkingMode,
    ) -> Result<Vec<RagRecord>, String> {
        chapter_rag_records_with_provider(chapter, self.embedding.as_ref(), mode).await
    }
}
